use camino::{Utf8Path, Utf8PathBuf};
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::process::{Command, Output};

/// Program to keep git repositories in sync between computers
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Pull latest changes for all repositories
    Pull,
    /// Push local changes for all repositories
    Push,
}

#[derive(Debug)]
enum Existence {
    Exists,
    DoesNotExist,
}

#[derive(Debug)]
enum ChangeStatus {
    HasChanges,
    NoChanges,
}

#[derive(Debug)]
enum PullStatus {
    NeedsPull,
    UpToDate,
}

#[derive(Debug)]
enum PushStatus {
    NeedsPush,
    UpToDate,
}

#[derive(Debug)]
struct RepoStatus {
    path: Utf8PathBuf,
    existence: Existence,
    branch: String,
    remote: String,
    change_status: ChangeStatus,
    pull_status: PullStatus,
    push_status: PushStatus,
}

#[derive(Debug)]
enum SyncMode {
    Pull,
    Push,
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Pull => sync_repos(SyncMode::Pull),
        Commands::Push => sync_repos(SyncMode::Push),
    }
}

fn read_repos() -> Vec<Utf8PathBuf> {
    // Read repository list from ~/.config/grit.conf
    // Format: one repository path per line
    let config_path = shellexpand::tilde("~/.config/grit.conf").to_string();
    let file = File::open(config_path).expect("Failed to open config file");
    let reader = io::BufReader::new(file);
    reader
        .lines()
        .map(|line| {
            Utf8PathBuf::from(shellexpand::tilde(&line.expect("Failed to read line")).to_string())
        })
        .collect()
}

fn sync_repos(mode: SyncMode) {
    let repos = read_repos();
    let mut repo_statuses = Vec::new();

    for repo in repos {
        let status = get_repo_status(&repo, &mode);
        repo_statuses.push(status);
    }

    print_summary(&repo_statuses, &mode);

    print!("\nDo you want to proceed? Type 'yes' to continue: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    if input.trim() != "yes" {
        println!("Operation cancelled.");
        return;
    }

    execute_plan(&repo_statuses, &mode);
    print_final_summary(&repo_statuses, &mode);
}

fn get_repo_status(path: &Utf8Path, mode: &SyncMode) -> RepoStatus {
    let existence = if path.exists() {
        Existence::Exists
    } else {
        Existence::DoesNotExist
    };
    let branch = match existence {
        Existence::Exists => String::from_utf8_lossy(
            &run_git_command(path, &["rev-parse", "--abbrev-ref", "HEAD"]).stdout,
        )
        .trim()
        .to_string(),
        Existence::DoesNotExist => String::new(),
    };
    let remote = match existence {
        Existence::Exists => {
            String::from_utf8_lossy(&run_git_command(path, &["remote", "get-url", "origin"]).stdout)
                .trim()
                .to_string()
        }
        Existence::DoesNotExist => String::new(),
    };
    let change_status = match existence {
        Existence::Exists => {
            if run_git_command(path, &["status", "--porcelain"])
                .stdout
                .is_empty()
            {
                ChangeStatus::NoChanges
            } else {
                ChangeStatus::HasChanges
            }
        }
        Existence::DoesNotExist => ChangeStatus::NoChanges,
    };
    let pull_status = match (mode, existence) {
        (SyncMode::Pull, Existence::Exists) => {
            if String::from_utf8_lossy(&run_git_command(path, &["rev-list", "HEAD..@{u}"]).stdout)
                .trim()
                .is_empty()
            {
                PullStatus::UpToDate
            } else {
                PullStatus::NeedsPull
            }
        }
        _ => PullStatus::UpToDate,
    };
    let push_status = match (mode, existence) {
        (SyncMode::Push, Existence::Exists) => {
            if String::from_utf8_lossy(&run_git_command(path, &["rev-list", "@{u}..HEAD"]).stdout)
                .trim()
                .is_empty()
            {
                PushStatus::UpToDate
            } else {
                PushStatus::NeedsPush
            }
        }
        _ => PushStatus::UpToDate,
    };

    RepoStatus {
        path: path.to_owned(),
        existence,
        branch,
        remote,
        change_status,
        pull_status,
        push_status,
    }
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

fn execute_plan(repo_statuses: &[RepoStatus], mode: &SyncMode) {
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
                let output = run_git_command(&status.path, &["pull"]);
                if output.status.success() {
                    println!("  {} Successfully pulled changes", "✅".green());
                } else {
                    println!("  {} Failed to pull changes", "❌".red());
                    println!("{}", String::from_utf8_lossy(&output.stderr));
                }
            }
            (SyncMode::Push, _, PushStatus::NeedsPush, _)
            | (SyncMode::Push, _, _, ChangeStatus::HasChanges) => {
                match status.change_status {
                    ChangeStatus::HasChanges => {
                        let add_output = run_git_command(&status.path, &["add", "."]);
                        if !add_output.status.success() {
                            println!("  {} Failed to stage changes", "❌".red());
                            println!("{}", String::from_utf8_lossy(&add_output.stderr));
                            continue;
                        }

                        print!("  Enter commit message: ");
                        io::stdout().flush().unwrap();
                        let mut commit_msg = String::new();
                        io::stdin().read_line(&mut commit_msg).unwrap();

                        let commit_output =
                            run_git_command(&status.path, &["commit", "-m", commit_msg.trim()]);
                        if !commit_output.status.success() {
                            println!("  {} Failed to commit changes", "❌".red());
                            println!("{}", String::from_utf8_lossy(&commit_output.stderr));
                            continue;
                        }
                        println!("  {} Changes committed", "✅".green());
                    }
                    ChangeStatus::NoChanges => {}
                }

                let push_output = run_git_command(&status.path, &["push"]);
                if push_output.status.success() {
                    println!("  {} Successfully pushed changes", "✅".green());
                } else {
                    println!("  {} Failed to push changes", "❌".red());
                    println!("{}", String::from_utf8_lossy(&push_output.stderr));
                }
            }
            _ => {
                println!("  {} No action needed", "ℹ️".blue());
            }
        }
    }
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

fn run_git_command(path: &Utf8Path, args: &[&str]) -> Output {
    Command::new("git")
        .current_dir(path)
        .args(args)
        .output()
        .expect("Failed to execute git command")
}
