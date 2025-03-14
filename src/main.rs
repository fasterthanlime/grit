// Rules:
// 1. Always use eprintln!(), not println!()
// 2. Be friendly with colors and emojis but not too uppity
// 3. FIRST come up with a plan, gathering all the data, THEN apply it
// 4. Ask for consent before applying the plan, showing the exact commands to run
// 5. When skipping a repo, explain why (couldn't parse git-rev, etc.)
// 6. Better to panic if git output isn't as expected than to do harmful things
// 7. When printing specific values, like paths, numbers, keywords like "yes" and "no", use colors suited to the theme

use camino::Utf8Path;
use camino::Utf8PathBuf;
use clap::Parser;
use cli::{Args, Commands, SyncMode};
use config::read_repos_from_default_config;
use eyre::Context;
use futures_util::StreamExt;
use owo_colors::OwoColorize;
use owo_colors::Style;
use std::fmt;
use std::io::{self, Write};

mod cheer;
mod cli;
mod config;
mod git;

#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    real_main().await
}

async fn real_main() -> eyre::Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    match args.command {
        Commands::Pull => sync_repos(SyncMode::Pull).await?,
        Commands::Push => sync_repos(SyncMode::Push).await?,
    }

    Ok(())
}

async fn sync_repos(mode: SyncMode) -> eyre::Result<()> {
    let repos = read_repos_from_default_config()?;
    let repo_statuses = futures_util::stream::iter(repos.iter())
        .map(|repo| async { get_repo_status(repo, &mode).await })
        .buffer_unordered(8)
        .filter_map(|status| async move { status.ok().flatten() })
        .collect::<Vec<_>>()
        .await;

    // First, create the plan from all gathered data
    let plan = ExecutionPlan::new(repo_statuses, mode);

    // If the plan is a no-op, we don't need to ask for consent
    if plan.is_noop() {
        cheer::cheer();
        return Ok(());
    }

    // Display the summary and plan
    eprintln!("{plan}");

    // Ask for consent before applying the plan
    eprint!(
        "\nDo you want to proceed? Type {} to continue: ",
        "yes".green()
    );
    io::stdout().flush().wrap_err("Failed to flush stdout")?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .wrap_err("Failed to read input")?;

    if input.trim() != "yes" {
        eprintln!("{}", "Operation cancelled.".red());
        return Ok(());
    }

    // Execute the plan
    plan.execute().await?;

    Ok(())
}

// RULES:
// Things that are non-fatal (return Ok(None))
//   - the directory does not exist
// Things that should be fatal (return an error)
//   - the directory is not a git repo
//   - any of the git gathering commands fail
async fn get_repo_status(path: &Utf8Path, mode: &SyncMode) -> eyre::Result<Option<RepoStatus>> {
    if !path.exists() {
        eprintln!(
            "  {} {} does not exist",
            "⚠️".yellow(),
            path.to_string().bright_cyan()
        );
        return Ok(None);
    }

    if !path.join(".git").is_dir() {
        return Err(eyre::eyre!(
            "{} is not a valid git repository",
            path.to_string().red()
        ));
    }

    let branch = git::run_git_command_quiet(
        path,
        &["rev-parse", "--abbrev-ref", "HEAD"],
        git::GitCommandBehavior::AssertZeroExitCode,
    )
    .await?
    .stdout
    .trim()
    .to_string();

    let remote = git::run_git_command_quiet(
        path,
        &["remote", "get-url", "origin"],
        git::GitCommandBehavior::AssertZeroExitCode,
    )
    .await?
    .stdout
    .trim()
    .to_string();

    let action: Option<RepoAction> = match mode {
        SyncMode::Push => {
            let status_output = git::run_git_command_quiet(
                path,
                &["status", "--porcelain"],
                git::GitCommandBehavior::AssertZeroExitCode,
            )
            .await?;

            let staged_output = git::run_git_command_quiet(
                path,
                &["diff", "--cached", "--quiet"],
                git::GitCommandBehavior::AllowNonZeroExitCode,
            )
            .await?;

            let rev_list_output = git::run_git_command_quiet(
                path,
                &["rev-list", "@{u}..HEAD"],
                git::GitCommandBehavior::AssertZeroExitCode,
            )
            .await?;

            if !status_output.stdout.trim().is_empty() {
                Some(RepoAction::Stage)
            } else if staged_output.status.code() == Some(1) {
                Some(RepoAction::Commit)
            } else if !rev_list_output.stdout.trim().is_empty() {
                Some(RepoAction::Push)
            } else {
                None
            }
        }
        SyncMode::Pull => {
            let fetch_output = git::run_git_command_quiet(
                path,
                &["fetch", "--all"],
                git::GitCommandBehavior::AssertZeroExitCode,
            )
            .await?;

            if !fetch_output.stderr.is_empty() {
                eprintln!("  {} Failed to fetch changes", "⚠️".yellow());
                eprintln!("{}", fetch_output.stderr.red());
            }

            let rev_list_output = git::run_git_command_quiet(
                path,
                &["rev-list", "HEAD..@{u}"],
                git::GitCommandBehavior::AssertZeroExitCode,
            )
            .await?;

            if rev_list_output.stdout.trim().is_empty() {
                None
            } else {
                Some(RepoAction::Pull)
            }
        }
    };

    Ok(Some(RepoStatus {
        path: path.to_owned(),
        branch,
        remote,
        action,
    }))
}

#[derive(Debug)]
pub(crate) struct RepoStatus {
    pub(crate) path: Utf8PathBuf,
    pub(crate) branch: String,
    pub(crate) remote: String,
    pub(crate) action: Option<RepoAction>,
}

/// Defines the mode of synchronization

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RepoAction {
    Stage,
    Commit,
    Push,
    Pull,
}

impl RepoAction {
    pub(crate) fn needs_stage(&self) -> bool {
        matches!(self, RepoAction::Stage)
    }

    pub(crate) fn needs_commit(&self) -> bool {
        matches!(self, RepoAction::Stage | RepoAction::Commit)
    }

    pub(crate) fn needs_push(&self) -> bool {
        matches!(
            self,
            RepoAction::Stage | RepoAction::Commit | RepoAction::Push
        )
    }
}

// In plan.rs, update ActionStep and ExecutionPlan
pub(crate) enum ActionStep {
    Stage(Utf8PathBuf),
    Commit(Utf8PathBuf),
    Push(Utf8PathBuf),
    Pull(Utf8PathBuf),
}

impl ActionStep {
    pub(crate) async fn execute(&self) -> eyre::Result<()> {
        match self {
            ActionStep::Stage(path) => {
                eprintln!("\n📁 {}", path.bright_cyan());
                let output = git::assert_git_command(path, &["add", "."]).await?;
                let status = output.status;
                if status.success() {
                    eprintln!("  {} Changes staged successfully", "✅".green());
                } else {
                    eprintln!("  {} Failed to stage changes", "❌".red());
                    eprintln!("{}", output.stderr);
                }
                Ok(())
            }
            ActionStep::Commit(path) => {
                eprintln!("\n📁 {}", path.bright_cyan());
                eprintln!("  Opening editor for commit message...");

                let status = tokio::process::Command::new("git")
                    .current_dir(path)
                    .arg("commit")
                    .status()
                    .await?;

                if status.success() {
                    eprintln!("  {} Changes committed successfully", "✅".green());
                } else {
                    eprintln!("  {} Failed to commit changes", "❌".red());
                }
                Ok(())
            }
            ActionStep::Push(path) => {
                eprintln!("\n📁 {}", path.bright_cyan());
                let output = git::assert_git_command(path, &["push"]).await?;
                let status = output.status;
                if status.success() {
                    eprintln!("  {} Successfully pushed changes", "✅".green());
                } else {
                    eprintln!("  {} Failed to push changes", "❌".red());
                    eprintln!("{}", output.stderr);
                }
                Ok(())
            }
            ActionStep::Pull(path) => {
                eprintln!("\n📁 {}", path.bright_cyan());
                let output = git::assert_git_command(path, &["pull"]).await?;
                let status = output.status;
                if status.success() {
                    if output.stdout.contains("Already up to date.") {
                        eprintln!("  {} Already up to date", "✅".green());
                    } else {
                        eprintln!("  {} Changes pulled successfully", "✅".green());
                    }
                } else {
                    eprintln!("  {} Failed to pull changes", "❌".red());
                    eprintln!("{}", output.stderr);
                }
                Ok(())
            }
        }
    }
}

impl ExecutionPlan {
    pub(crate) fn new(repo_statuses: Vec<RepoStatus>, mode: SyncMode) -> Self {
        let mut steps = Vec::new();

        for status in &repo_statuses {
            let Some(action) = &status.action else {
                continue;
            };

            match mode {
                SyncMode::Push => {
                    if action.needs_stage() {
                        steps.push(ActionStep::Stage(status.path.clone()));
                    }
                    if action.needs_commit() {
                        steps.push(ActionStep::Commit(status.path.clone()));
                    }
                    if action.needs_push() {
                        steps.push(ActionStep::Push(status.path.clone()));
                    }
                }
                SyncMode::Pull => {
                    if action.needs_push() {
                        steps.push(ActionStep::Pull(status.path.clone()));
                    }
                }
            }
        }

        ExecutionPlan {
            steps,
            mode,
            repo_statuses,
        }
    }
}

pub(crate) struct ExecutionPlan {
    pub(crate) steps: Vec<ActionStep>,
    pub(crate) mode: SyncMode,
    pub(crate) repo_statuses: Vec<RepoStatus>,
}

impl ExecutionPlan {
    pub(crate) fn is_noop(&self) -> bool {
        self.steps.is_empty()
    }

    pub(crate) async fn execute(&self) -> eyre::Result<()> {
        for step in &self.steps {
            step.execute().await?;
        }
        Ok(())
    }
}

impl fmt::Display for ExecutionPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "\n{} Plan:",
            match self.mode {
                SyncMode::Pull => "Pull",
                SyncMode::Push => "Push",
            }
            .bright_cyan()
        )?;

        for status in &self.repo_statuses {
            writeln!(f, "\n📁 {}", status.path.bright_cyan())?;
            writeln!(
                f,
                "  {} @ {}",
                status.branch.bright_yellow(),
                status.remote.bright_yellow()
            )?;
            writeln!(
                f,
                "  Status: {}",
                match status.action {
                    RepoAction::Stage => "Needs staging".style(Style::new().bright_red()),
                    RepoAction::Commit => "Needs commit".style(Style::new().bright_yellow()),
                    RepoAction::Push => "Needs push".style(Style::new().bright_blue()),
                    RepoAction::Pull => "Needs pull".style(Style::new().bright_magenta()),
                }
            )?;

            match status.action {
                RepoAction::Stage => {
                    writeln!(f, "  {}: git add .", "Will execute".bright_blue())?;
                    writeln!(f, "  {}: git commit", "Will execute".bright_blue())?;
                    writeln!(f, "  {}: git push", "Will execute".bright_blue())?;
                }
                RepoAction::Commit => {
                    writeln!(f, "  {}: git commit", "Will execute".bright_blue())?;
                    writeln!(f, "  {}: git push", "Will execute".bright_blue())?;
                }
                RepoAction::Push => {
                    writeln!(f, "  {}: git push", "Will execute".bright_blue())?;
                }
                RepoAction::Pull => {
                    writeln!(f, "  {}: git pull", "Will execute".bright_blue())?;
                }
            }
        }

        Ok(())
    }
}
