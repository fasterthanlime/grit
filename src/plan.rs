use camino::Utf8PathBuf;
use eyre::Context;
use owo_colors::OwoColorize;
use std::fmt;
use std::io::{self, Write};

use crate::cli::{ChangeStatus, Existence, PullStatus, PushStatus, RepoStatus, SyncMode};
use crate::git;

#[derive(Debug)]
pub(crate) enum ActionStep {
    Pull(Utf8PathBuf),
    AddCommitPush {
        path: Utf8PathBuf,
        has_changes: bool,
    },
    Skip(Utf8PathBuf, String),
    NoAction(Utf8PathBuf),
}

impl ActionStep {
    pub(crate) async fn execute(&self) -> eyre::Result<()> {
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

pub(crate) struct ExecutionPlan {
    pub(crate) steps: Vec<ActionStep>,
    pub(crate) mode: SyncMode,
    pub(crate) repo_statuses: Vec<RepoStatus>,
}

impl ExecutionPlan {
    pub(crate) fn new(repo_statuses: Vec<RepoStatus>, mode: SyncMode) -> Self {
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
