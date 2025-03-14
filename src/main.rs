// Rules:
// 1. Always use eprintln!(), not println!()
// 2. Be friendly with colors and emojis but not too uppity
// 3. FIRST come up with a plan, gathering all the data, THEN apply it
// 4. Ask for consent before applying the plan, showing the exact commands to run
// 5. When skipping a repo, explain why (couldn't parse git-rev, etc.)
// 6. Better to panic if git output isn't as expected than to do harmful things
// 7. When printing specific values, like paths, numbers, keywords like "yes" and "no", use colors suited to the theme

use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use cli::{Args, ChangeStatus, Commands, Existence, PullStatus, PushStatus, RepoStatus, SyncMode};
use eyre::Context;
use owo_colors::OwoColorize;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, Write};

mod cli;
mod git;

#[derive(Debug)]
enum ActionStep {
    Pull(Utf8PathBuf),
    AddCommitPush {
        path: Utf8PathBuf,
        has_changes: bool,
    },
    Skip(Utf8PathBuf, String),
    NoAction(Utf8PathBuf),
}

impl ActionStep {
    async fn execute(&self) -> eyre::Result<()> {
        match self {
            ActionStep::Pull(path) => {
                eprintln!("\n📁 {}", path.bright_cyan());
                let output = git::run_git_command(path, &["pull"]).await?;
                if output.stdout.contains("Already up to date.") {
                    eprintln!("  {} Successfully pulled changes", "✅".green());
                } else if output.stderr.is_empty() {
                    eprintln!("  {} Changes pulled successfully", "✅".green());
                } else {
                    eprintln!("  {} Failed to pull changes", "❌".red());
                    eprintln!("{}", output.stderr);
                }
                Ok(())
            }
            ActionStep::AddCommitPush { path, has_changes } => {
                eprintln!("\n📁 {}", path.bright_cyan());

                if *has_changes {
                    let add_output = git::run_git_command(path, &["add", "."]).await?;
                    if !add_output.stderr.is_empty() {
                        eprintln!("  {} Failed to stage changes", "❌".red());
                        eprintln!("{}", add_output.stderr);
                        return Ok(());
                    }

                    eprint!("  Enter commit message: ");
                    io::stdout().flush().wrap_err("Failed to flush stdout")?;
                    let mut commit_msg = String::new();
                    io::stdin()
                        .read_line(&mut commit_msg)
                        .wrap_err("Failed to read input")?;

                    let commit_output =
                        git::run_git_command(path, &["commit", "-m", commit_msg.trim()]).await?;

                    if !commit_output.stderr.is_empty()
                        && !commit_output.stderr.contains("nothing to commit")
                    {
                        eprintln!("  {} Failed to commit changes", "❌".red());
                        eprintln!("{}", commit_output.stderr);
                        return Ok(());
                    }
                    eprintln!("  {} Changes committed", "✅".green());
                }

                let push_output = git::run_git_command(path, &["push"]).await?;
                if push_output.stderr.is_empty()
                    || push_output.stderr.contains("Everything up-to-date")
                {
                    eprintln!("  {} Successfully pushed changes", "✅".green());
                } else {
                    eprintln!("  {} Failed to push changes", "❌".red());
                    eprintln!("{}", push_output.stderr);
                }

                Ok(())
            }
            ActionStep::Skip(path, reason) => {
                eprintln!("\n📁 {}", path.bright_cyan());
                eprintln!("  {} {reason}", "⚠️".yellow());
                Ok(())
            }
            ActionStep::NoAction(path) => {
                eprintln!("\n📁 {}", path.bright_cyan());
                eprintln!("  {} No action needed", "ℹ️".blue());
                Ok(())
            }
        }
    }
}

struct ExecutionPlan {
    steps: Vec<ActionStep>,
    mode: SyncMode,
    repo_statuses: Vec<RepoStatus>,
}

impl ExecutionPlan {
    fn new(repo_statuses: Vec<RepoStatus>, mode: SyncMode) -> Self {
        let mut steps = Vec::new();

        for status in &repo_statuses {
            match status.existence {
                Existence::DoesNotExist => {
                    steps.push(ActionStep::Skip(
                        status.path.clone(),
                        "Directory does not exist or is not a git repository".to_string(),
                    ));
                }
                Existence::Exists => {
                    match (
                        &mode,
                        &status.pull_status,
                        &status.push_status,
                        &status.change_status,
                    ) {
                        (SyncMode::Pull, PullStatus::NeedsPull, _, _) => {
                            steps.push(ActionStep::Pull(status.path.clone()));
                        }
                        (SyncMode::Push, _, PushStatus::NeedsPush, _)
                        | (SyncMode::Push, _, _, ChangeStatus::HasChanges) => {
                            steps.push(ActionStep::AddCommitPush {
                                path: status.path.clone(),
                                has_changes: matches!(
                                    status.change_status,
                                    ChangeStatus::HasChanges
                                ),
                            });
                        }
                        _ => {
                            steps.push(ActionStep::NoAction(status.path.clone()));
                        }
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

    async fn execute(&self) -> eyre::Result<()> {
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
        )?;

        for step in &self.steps {
            match step {
                ActionStep::Pull(path) => {
                    writeln!(f, "\n📁 {}", path)?;
                    writeln!(f, "  Will execute: git pull")?;
                }
                ActionStep::AddCommitPush { path, has_changes } => {
                    writeln!(f, "\n📁 {}", path)?;
                    if *has_changes {
                        writeln!(f, "  Will execute: git add .")?;
                        writeln!(f, "  Will prompt for commit message")?;
                        writeln!(f, "  Will execute: git commit -m <message>")?;
                    }
                    writeln!(f, "  Will execute: git push")?;
                }
                ActionStep::Skip(path, reason) => {
                    writeln!(f, "\n📁 {}", path)?;
                    writeln!(f, "  Will skip: {}", reason)?;
                }
                ActionStep::NoAction(path) => {
                    writeln!(f, "\n📁 {}", path)?;
                    writeln!(f, "  No action needed")?;
                }
            }
        }

        Ok(())
    }
}

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

fn read_repos() -> eyre::Result<Vec<Utf8PathBuf>> {
    let config_path = shellexpand::tilde("~/.config/grit.conf").to_string();
    let config_file = Utf8PathBuf::from(&config_path);

    if !config_file.exists() {
        eprintln!("Config file not found at {}", config_path.bright_cyan());
        eprintln!(
            "Would you like to create an empty config file? ({}/{})",
            "yes".green(),
            "no".red()
        );

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() == "yes" {
            let example_config = r#"# Grit configuration file
# List one repository path per line, e.g.:
# /home/user/projects/repo1
# /home/user/projects/repo2
# ~/Documents/github/my-project
"#;

            std::fs::write(&config_file, example_config)?;

            eprintln!("Empty config file created at {}", config_path.bright_cyan());
            eprintln!("What's your preferred text editor?");

            let mut editor = String::new();
            io::stdin().read_line(&mut editor)?;
            editor = editor.trim().to_string();

            if !editor.is_empty() {
                std::process::Command::new(&editor)
                    .arg(&config_path)
                    .status()?;
            }
        } else {
            return Ok(Vec::new());
        }
    }

    let file = File::open(&config_file).wrap_err_with(|| {
        format!(
            "Failed to open config file at {}",
            config_path.bright_cyan()
        )
    })?;
    let reader = io::BufReader::new(file);
    reader
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let trimmed = line.trim();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                None
            } else {
                Some(Ok(Utf8PathBuf::from(
                    shellexpand::tilde(trimmed).to_string(),
                )))
            }
        })
        .collect()
}

async fn sync_repos(mode: SyncMode) -> eyre::Result<()> {
    let repos = read_repos()?;
    let mut repo_statuses = Vec::new();

    for repo in &repos {
        let status = get_repo_status(repo, &mode).await?;
        repo_statuses.push(status);
    }

    // First, create the plan from all gathered data
    let plan = ExecutionPlan::new(repo_statuses, mode);

    // Display the summary and plan
    print_summary(&plan);
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

    // Print final summary
    print_final_summary(&plan);

    Ok(())
}

async fn get_repo_status(path: &Utf8Path, mode: &SyncMode) -> eyre::Result<RepoStatus> {
    let existence = if path.exists() {
        if path.join(".git").is_dir() {
            Existence::Exists
        } else {
            eprintln!(
                "  {} Directory exists but is not a git repository",
                "⚠️".yellow()
            );
            Existence::DoesNotExist
        }
    } else {
        Existence::DoesNotExist
    };

    let branch = match existence {
        Existence::Exists => {
            let output = git::run_git_command(path, &["rev-parse", "--abbrev-ref", "HEAD"]).await?;
            output.stdout.trim().to_string()
        }
        Existence::DoesNotExist => String::new(),
    };

    let remote = match existence {
        Existence::Exists => {
            let output = git::run_git_command(path, &["remote", "get-url", "origin"]).await?;
            output.stdout.trim().to_string()
        }
        Existence::DoesNotExist => String::new(),
    };

    let change_status = match existence {
        Existence::Exists => {
            let output = git::run_git_command(path, &["status", "--porcelain"]).await?;
            if output.stdout.is_empty() {
                ChangeStatus::NoChanges
            } else {
                ChangeStatus::HasChanges
            }
        }
        Existence::DoesNotExist => ChangeStatus::NoChanges,
    };

    let pull_status = match (mode, existence) {
        (SyncMode::Pull, Existence::Exists) => {
            // First, fetch all changes
            let fetch_output = git::run_git_command(path, &["fetch", "--all"]).await?;
            if !fetch_output.stderr.is_empty() {
                eprintln!("  {} Failed to fetch changes", "⚠️".yellow());
                eprintln!("{}", fetch_output.stderr);
            }

            // Then check if there are changes to pull
            let output = git::run_git_command(path, &["rev-list", "HEAD..@{u}"]).await?;
            if output.stdout.trim().is_empty() {
                PullStatus::UpToDate
            } else {
                PullStatus::NeedsPull
            }
        }
        _ => PullStatus::UpToDate,
    };

    let push_status = match (mode, existence) {
        (SyncMode::Push, Existence::Exists) => {
            let output = git::run_git_command(path, &["rev-list", "@{u}..HEAD"]).await?;
            if output.stdout.trim().is_empty() {
                PushStatus::UpToDate
            } else {
                PushStatus::NeedsPush
            }
        }
        _ => PushStatus::UpToDate,
    };

    Ok(RepoStatus {
        path: path.to_owned(),
        existence,
        branch,
        remote,
        change_status,
        pull_status,
        push_status,
    })
}

fn print_summary(plan: &ExecutionPlan) {
    eprintln!(
        "\n{} Summary:",
        match plan.mode {
            SyncMode::Pull => "Pull",
            SyncMode::Push => "Push",
        }
    );

    for status in &plan.repo_statuses {
        eprintln!("\n📁 {}", status.path.bright_cyan());

        match status.existence {
            Existence::DoesNotExist => {
                eprintln!(
                    "  {} Directory does not exist or is not a git repository",
                    "⚠️".yellow()
                );
                continue;
            }
            Existence::Exists => {}
        }

        eprintln!("  Branch: {}", status.branch.bright_magenta());
        eprintln!("  Remote: {}", status.remote.bright_blue());

        if status.branch != "main" && status.branch != "master" {
            eprintln!("  {} Not on main branch", "⚠️".yellow());
        }

        match status.change_status {
            ChangeStatus::HasChanges => eprintln!("  {} Local changes detected", "📝".yellow()),
            ChangeStatus::NoChanges => {}
        }

        match (plan.mode, &status.pull_status, &status.push_status) {
            (SyncMode::Pull, PullStatus::NeedsPull, _) => {
                eprintln!("  {} Changes to pull", "⬇️".green())
            }
            (SyncMode::Push, _, PushStatus::NeedsPush) => {
                eprintln!("  {} Changes to push", "⬆️".green())
            }
            _ => eprintln!("  {} Up to date", "✅".green()),
        }
    }
}

fn print_final_summary(plan: &ExecutionPlan) {
    eprintln!(
        "\n{} Final Summary:",
        match plan.mode {
            SyncMode::Pull => "Pull",
            SyncMode::Push => "Push",
        }
    );

    for status in &plan.repo_statuses {
        match status.existence {
            Existence::DoesNotExist => continue,
            Existence::Exists => {}
        }

        eprintln!("\n📁 {}", status.path.bright_cyan());
        eprintln!("  Branch: {}", status.branch.bright_magenta());
        eprintln!("  Remote: {}", status.remote.bright_blue());

        match plan.mode {
            SyncMode::Pull => {
                eprintln!(
                    "  {} {}",
                    match status.pull_status {
                        PullStatus::NeedsPull => "⬇️",
                        PullStatus::UpToDate => "✅",
                    },
                    match status.pull_status {
                        PullStatus::NeedsPull => "Changes pulled",
                        PullStatus::UpToDate => "Already up to date",
                    }
                );
            }
            SyncMode::Push => {
                eprintln!(
                    "  {} {}",
                    match (&status.push_status, &status.change_status) {
                        (PushStatus::NeedsPush, _) | (_, ChangeStatus::HasChanges) => "⬆️",
                        (PushStatus::UpToDate, ChangeStatus::NoChanges) => "✅",
                    },
                    match (&status.push_status, &status.change_status) {
                        (PushStatus::NeedsPush, _) | (_, ChangeStatus::HasChanges) =>
                            "Changes pushed",
                        (PushStatus::UpToDate, ChangeStatus::NoChanges) => "No changes to push",
                    }
                );
            }
        }
    }
}
