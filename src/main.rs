use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::io::{self, Write};
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

/// Repositories to keep in sync
static REPOS: &[&str] = &[
    "~/dotfiles",
    "~/bearcove/bearcove.eu",
    "~/bearcove/snug",
    "~/bearcove/snugkit",
    "~/bearcove/sdr-podcast.com",
    "~/bearcove/fasterthanli.me",
];

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Pull => pull_repos(),
        Commands::Push => push_repos(),
    }
}

fn pull_repos() {
    for repo in REPOS {
        let path = shellexpand::tilde(repo);
        println!("{}:", repo.green().bold());

        if !Path::new(&*path).exists() {
            println!(
                "  {} {}",
                "ERROR:".bright_red().bold(),
                "Directory does not exist"
            );
            continue;
        }

        // Check for local changes
        let status = run_git_command(&path, &["status", "--porcelain"]);
        if !status.stdout.is_empty() {
            println!("  {} Local changes detected", "WARNING:".yellow().bold());
            println!("  Consider committing your changes first:");
            println!("    {} git add .", "→".blue());
            println!("    {} git commit -m \"Your message\"", "→".blue());
            continue;
        }

        // Pull changes
        let output = run_git_command(&path, &["pull"]);
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.contains("Already up to date") {
                println!("  {} Already up to date", "INFO:".bright_blue().bold());
            } else {
                println!(
                    "  {} Successfully pulled changes",
                    "SUCCESS:".green().bold()
                );
                println!("{}", String::from_utf8_lossy(&output.stdout));
            }
        } else {
            println!("  {} Failed to pull changes", "ERROR:".bright_red().bold());
            println!("{}", String::from_utf8_lossy(&output.stderr));
        }
        println!();
    }
}

fn push_repos() {
    for repo in REPOS {
        let path = shellexpand::tilde(repo);
        println!("{}:", repo.green().bold());

        if !Path::new(&*path).exists() {
            println!(
                "  {} {}",
                "ERROR:".bright_red().bold(),
                "Directory does not exist"
            );
            continue;
        }

        // Check for changes
        let status = run_git_command(&path, &["status", "--porcelain"]);
        if !status.stdout.is_empty() {
            println!("  {} Changes detected:", "INFO:".bright_blue().bold());
            println!("{}", String::from_utf8_lossy(&status.stdout));

            print!("  Would you like to stage these changes? [y/N]: ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();

            if input.trim().to_lowercase() == "y" {
                // Stage changes
                let add_output = run_git_command(&path, &["add", "."]);
                if !add_output.status.success() {
                    println!("  {} Failed to stage changes", "ERROR:".bright_red().bold());
                    println!("{}", String::from_utf8_lossy(&add_output.stderr));
                    continue;
                }

                // Get commit message
                print!("  Enter commit message: ");
                io::stdout().flush().unwrap();

                let mut commit_msg = String::new();
                io::stdin().read_line(&mut commit_msg).unwrap();

                // Commit changes
                let commit_output = run_git_command(&path, &["commit", "-m", commit_msg.trim()]);
                if !commit_output.status.success() {
                    println!(
                        "  {} Failed to commit changes",
                        "ERROR:".bright_red().bold()
                    );
                    println!("{}", String::from_utf8_lossy(&commit_output.stderr));
                    continue;
                }

                println!("  {} Changes committed", "SUCCESS:".green().bold());
            } else {
                println!("  Skipping repository");
                continue;
            }
        } else {
            println!("  {} No changes to commit", "INFO:".bright_blue().bold());
        }

        // Push changes
        print!("  Push changes to remote? [y/N]: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if input.trim().to_lowercase() == "y" {
            let push_output = run_git_command(&path, &["push"]);
            if push_output.status.success() {
                println!(
                    "  {} Successfully pushed changes",
                    "SUCCESS:".green().bold()
                );
            } else {
                println!("  {} Failed to push changes", "ERROR:".bright_red().bold());
                println!("{}", String::from_utf8_lossy(&push_output.stderr));
            }
        } else {
            println!("  Skipping push");
        }
        println!();
    }
}

fn run_git_command(path: &str, args: &[&str]) -> Output {
    Command::new("git")
        .current_dir(path)
        .args(args)
        .output()
        .expect("Failed to execute git command")
}
