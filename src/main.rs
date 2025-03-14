use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;
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
struct RepoStatus {
    path: String,
    exists: bool,
    branch: String,
    remote: String,
    has_changes: bool,
    needs_pull: bool,
    needs_push: bool,
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Pull => sync_repos(true),
        Commands::Push => sync_repos(false),
    }
}

fn read_repos() -> Vec<String> {
    // Read repository list from ~/.config/grit.conf
    // Format: one repository path per line
    let config_path = shellexpand::tilde("~/.config/grit.conf").to_string();
    let file = File::open(config_path).expect("Failed to open config file");
    let reader = io::BufReader::new(file);
    reader
        .lines()
        .map(|line| line.expect("Failed to read line"))
        .collect()
}

fn sync_repos(is_pull: bool) {
    let repos = read_repos();
    let mut repo_statuses = Vec::new();

    for repo in repos {
        let path = shellexpand::tilde(&repo).to_string();
        let status = get_repo_status(&path, is_pull);
        repo_statuses.push(status);
    }

    print_summary(&repo_statuses, is_pull);

    print!("\nDo you want to proceed? Type 'yes' to continue: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    if input.trim() != "yes" {
        println!("Operation cancelled.");
        return;
    }

    execute_plan(&repo_statuses, is_pull);
    print_final_summary(&repo_statuses, is_pull);
}

fn get_repo_status(path: &str, is_pull: bool) -> RepoStatus {
    let exists = Path::new(path).exists();
    let branch = if exists {
        String::from_utf8_lossy(
            &run_git_command(path, &["rev-parse", "--abbrev-ref", "HEAD"]).stdout,
        )
        .trim()
        .to_string()
    } else {
        String::new()
    };
    let remote = if exists {
        String::from_utf8_lossy(&run_git_command(path, &["remote", "get-url", "origin"]).stdout)
            .trim()
            .to_string()
    } else {
        String::new()
    };
    let has_changes = exists
        && !run_git_command(path, &["status", "--porcelain"])
            .stdout
            .is_empty();
    let needs_pull = is_pull
        && exists
        && !String::from_utf8_lossy(&run_git_command(path, &["rev-list", "HEAD..@{u}"]).stdout)
            .trim()
            .is_empty();
    let needs_push = !is_pull
        && exists
        && !String::from_utf8_lossy(&run_git_command(path, &["rev-list", "@{u}..HEAD"]).stdout)
            .trim()
            .is_empty();

    RepoStatus {
        path: path.to_string(),
        exists,
        branch,
        remote,
        has_changes,
        needs_pull,
        needs_push,
    }
}

fn print_summary(repo_statuses: &[RepoStatus], is_pull: bool) {
    println!("\n{} Summary:", if is_pull { "Pull" } else { "Push" });
    for status in repo_statuses {
        println!("\n📁 {}", status.path.bright_cyan());
        if !status.exists {
            println!("  {} Directory does not exist", "⚠️".yellow());
            continue;
        }
        println!("  Branch: {}", status.branch.bright_magenta());
        println!("  Remote: {}", status.remote.bright_blue());
        if status.branch != "main" && status.branch != "master" {
            println!("  {} Not on main branch", "⚠️".yellow());
        }
        if status.has_changes {
            println!("  {} Local changes detected", "📝".yellow());
        }
        if is_pull && status.needs_pull {
            println!("  {} Changes to pull", "⬇️".green());
        } else if !is_pull && status.needs_push {
            println!("  {} Changes to push", "⬆️".green());
        } else {
            println!("  {} Up to date", "✅".green());
        }
    }
}

fn execute_plan(repo_statuses: &[RepoStatus], is_pull: bool) {
    for status in repo_statuses {
        if !status.exists {
            continue;
        }

        println!("\n📁 {}", status.path.bright_cyan());

        if is_pull && status.needs_pull {
            let output = run_git_command(&status.path, &["pull"]);
            if output.status.success() {
                println!("  {} Successfully pulled changes", "✅".green());
            } else {
                println!("  {} Failed to pull changes", "❌".red());
                println!("{}", String::from_utf8_lossy(&output.stderr));
            }
        } else if !is_pull && (status.has_changes || status.needs_push) {
            if status.has_changes {
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

            let push_output = run_git_command(&status.path, &["push"]);
            if push_output.status.success() {
                println!("  {} Successfully pushed changes", "✅".green());
            } else {
                println!("  {} Failed to push changes", "❌".red());
                println!("{}", String::from_utf8_lossy(&push_output.stderr));
            }
        } else {
            println!("  {} No action needed", "ℹ️".blue());
        }
    }
}

fn print_final_summary(repo_statuses: &[RepoStatus], is_pull: bool) {
    println!("\n{} Final Summary:", if is_pull { "Pull" } else { "Push" });
    for status in repo_statuses {
        if !status.exists {
            continue;
        }
        println!("\n📁 {}", status.path.bright_cyan());
        println!("  Branch: {}", status.branch.bright_magenta());
        println!("  Remote: {}", status.remote.bright_blue());
        if is_pull {
            println!(
                "  {} {}",
                if status.needs_pull { "⬇️" } else { "✅" },
                if status.needs_pull {
                    "Changes pulled"
                } else {
                    "Already up to date"
                }
            );
        } else {
            println!(
                "  {} {}",
                if status.needs_push || status.has_changes {
                    "⬆️"
                } else {
                    "✅"
                },
                if status.needs_push || status.has_changes {
                    "Changes pushed"
                } else {
                    "No changes to push"
                }
            );
        }
    }
}

fn run_git_command(path: &str, args: &[&str]) -> Output {
    Command::new("git")
        .current_dir(path)
        .args(args)
        .output()
        .expect("Failed to execute git command")
}
