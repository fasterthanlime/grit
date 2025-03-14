use camino::{Utf8Path, Utf8PathBuf};
use cli::{Args, ChangeStatus, Commands, Existence, PullStatus, PushStatus, RepoStatus, SyncMode};
use eyre::Context;
use owo_colors::OwoColorize;
use std::fs::File;
use std::io::{self, BufRead, Write};

mod cli;
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

fn read_repos() -> eyre::Result<Vec<Utf8PathBuf>> {
    // Read repository list from ~/.config/grit.conf
    // Format: one repository path per line
    let config_path = shellexpand::tilde("~/.config/grit.conf").to_string();
    let file = File::open(config_path).wrap_err("Failed to open config file")?;
    let reader = io::BufReader::new(file);
    reader
        .lines()
        .map(|line| {
            let line = line.wrap_err("Failed to read line")?;
            Ok(Utf8PathBuf::from(shellexpand::tilde(&line).to_string()))
        })
        .collect()
}

async fn sync_repos(mode: SyncMode) -> eyre::Result<()> {
    let repos = read_repos()?;
    let mut repo_statuses = Vec::new();

    for repo in repos {
        let status = get_repo_status(&repo, &mode).await?;
        repo_statuses.push(status);
    }

    print_summary(&repo_statuses, &mode);

    print!("\nDo you want to proceed? Type 'yes' to continue: ");
    io::stdout().flush().wrap_err("Failed to flush stdout")?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .wrap_err("Failed to read input")?;

    if input.trim() != "yes" {
        println!("Operation cancelled.");
        return Ok(());
    }

    execute_plan(&repo_statuses, &mode).await?;
    print_final_summary(&repo_statuses, &mode);

    Ok(())
}

async fn get_repo_status(path: &Utf8Path, mode: &SyncMode) -> eyre::Result<RepoStatus> {
    let existence = if path.exists() {
        Existence::Exists
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

fn print_summary(repo_statuses: &[RepoStatus], mode: &SyncMode) {
    println!(
        "\n{} Summary:",
        match mode {
            SyncMode::Pull => "Pull",
            SyncMode::Push => "Push",
        }
    );
    for status in repo_statuses {
        println!("\n📁 {}", status.path.bright_cyan());
        match status.existence {
            Existence::DoesNotExist => {
                println!("  {} Directory does not exist", "⚠️".yellow());
                continue;
            }
            Existence::Exists => {}
        }
        println!("  Branch: {}", status.branch.bright_magenta());
        println!("  Remote: {}", status.remote.bright_blue());
        if status.branch != "main" && status.branch != "master" {
            println!("  {} Not on main branch", "⚠️".yellow());
        }
        match status.change_status {
            ChangeStatus::HasChanges => println!("  {} Local changes detected", "📝".yellow()),
            ChangeStatus::NoChanges => {}
        }
        match (mode, &status.pull_status, &status.push_status) {
            (SyncMode::Pull, PullStatus::NeedsPull, _) => {
                println!("  {} Changes to pull", "⬇️".green())
            }
            (SyncMode::Push, _, PushStatus::NeedsPush) => {
                println!("  {} Changes to push", "⬆️".green())
            }
            _ => println!("  {} Up to date", "✅".green()),
        }
    }
}

async fn execute_plan(repo_statuses: &[RepoStatus], mode: &SyncMode) -> eyre::Result<()> {
    for status in repo_statuses {
        match status.existence {
            Existence::DoesNotExist => continue,
            Existence::Exists => {}
        }

        println!("\n📁 {}", status.path.bright_cyan());

        match (
            mode,
            &status.pull_status,
            &status.push_status,
            &status.change_status,
        ) {
            (SyncMode::Pull, PullStatus::NeedsPull, _, _) => {
                let output = git::run_git_command(&status.path, &["pull"]).await?;
                if output.stdout.contains("Already up to date.") {
                    println!("  {} Successfully pulled changes", "✅".green());
                } else {
                    println!("  {} Failed to pull changes", "❌".red());
                    println!("{}", output.stderr);
                }
            }
            (SyncMode::Push, _, PushStatus::NeedsPush, _)
            | (SyncMode::Push, _, _, ChangeStatus::HasChanges) => {
                match status.change_status {
                    ChangeStatus::HasChanges => {
                        let add_output = git::run_git_command(&status.path, &["add", "."]).await?;
                        if !add_output.stderr.is_empty() {
                            println!("  {} Failed to stage changes", "❌".red());
                            println!("{}", add_output.stderr);
                            continue;
                        }

                        print!("  Enter commit message: ");
                        io::stdout().flush().unwrap();
                        let mut commit_msg = String::new();
                        io::stdin().read_line(&mut commit_msg).unwrap();

                        let commit_output = git::run_git_command(
                            &status.path,
                            &["commit", "-m", commit_msg.trim()],
                        )
                        .await?;
                        if !commit_output.stderr.is_empty() {
                            println!("  {} Failed to commit changes", "❌".red());
                            println!("{}", commit_output.stderr);
                            continue;
                        }
                        println!("  {} Changes committed", "✅".green());
                    }
                    ChangeStatus::NoChanges => {}
                }

                let push_output = git::run_git_command(&status.path, &["push"]).await?;
                if push_output.stderr.is_empty() {
                    println!("  {} Successfully pushed changes", "✅".green());
                } else {
                    println!("  {} Failed to push changes", "❌".red());
                    println!("{}", push_output.stderr);
                }
            }
            _ => {
                println!("  {} No action needed", "ℹ️".blue());
            }
        }
    }
    Ok(())
}

fn print_final_summary(repo_statuses: &[RepoStatus], mode: &SyncMode) {
    println!(
        "\n{} Final Summary:",
        match mode {
            SyncMode::Pull => "Pull",
            SyncMode::Push => "Push",
        }
    );
    for status in repo_statuses {
        match status.existence {
            Existence::DoesNotExist => continue,
            Existence::Exists => {}
        }
        println!("\n📁 {}", status.path.bright_cyan());
        println!("  Branch: {}", status.branch.bright_magenta());
        println!("  Remote: {}", status.remote.bright_blue());
        match mode {
            SyncMode::Pull => {
                println!(
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
                println!(
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
