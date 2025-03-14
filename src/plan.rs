use camino::Utf8PathBuf;
use owo_colors::{OwoColorize, Style};
use std::fmt;

use crate::cli::{RepoStatus, SyncMode};
use crate::git;

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
                if output.stderr.is_empty() {
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
                if output.stderr.is_empty() || output.stderr.contains("Everything up-to-date") {
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
                if output.stdout.contains("Already up to date.") {
                    eprintln!("  {} Already up to date", "✅".green());
                } else if output.stderr.is_empty() {
                    eprintln!("  {} Changes pulled successfully", "✅".green());
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
            match mode {
                SyncMode::Push => {
                    if status.action.needs_stage() {
                        steps.push(ActionStep::Stage(status.path.clone()));
                    }
                    if status.action.needs_commit() {
                        steps.push(ActionStep::Commit(status.path.clone()));
                    }
                    if status.action.needs_push() {
                        steps.push(ActionStep::Push(status.path.clone()));
                    }
                }
                SyncMode::Pull => {
                    if status.action.needs_push() {
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
