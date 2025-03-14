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

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Pull => pull_repos(),
        Commands::Push => push_repos(),
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

fn pull_repos() {
    let repos = read_repos();
    for repo in repos {
        let path = shellexpand::tilde(&repo);
        eprintln!("{repo}:");

        if !Path::new(&*path).exists() {
            eprintln!(
                "  {} {}",
                "ERROR:".bright_red().bold(),
                "Directory does not exist".dimmed()
            );
            continue;
        }

        // Check for local changes
        let status = run_git_command(&path, &["status", "--porcelain"]);
        if !status.stdout.is_empty() {
            eprintln!("  {} Local changes detected", "WARNING:".yellow().bold());
            eprintln!("  Consider committing your changes first:");
            eprintln!("    {} git add .", "→".blue());
            eprintln!("    {} git commit -m \"Your message\"", "→".blue());
            continue;
        }

        // Pull changes
        let output = run_git_command(&path, &["pull"]);
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.contains("Already up to date") {
                eprintln!("  {} Already up to date", "INFO:".bright_blue().bold());
            } else {
                eprintln!(
                    "  {} Successfully pulled changes",
                    "SUCCESS:".green().bold()
                );
                eprintln!("{output_str}");
            }
        } else {
            eprintln!("  {} Failed to pull changes", "ERROR:".bright_red().bold());
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }
        eprintln!();
    }
}

fn push_repos() {
    let repos = read_repos();
    for repo in repos {
        let path = shellexpand::tilde(&repo);
        eprintln!("{repo}:");

        if !Path::new(&*path).exists() {
            eprintln!(
                "  {} {}",
                "ERROR:".bright_red().bold(),
                "Directory does not exist".dimmed()
            );
            continue;
        }

        // Check for changes
        let status = run_git_command(&path, &["status", "--porcelain"]);
        if !status.stdout.is_empty() {
            eprintln!("  {} Changes detected:", "INFO:".bright_blue().bold());
            eprintln!("{}", String::from_utf8_lossy(&status.stdout));

            print!("  Would you like to stage these changes? [y/N]: ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();

            if input.trim().to_lowercase() == "y" {
                // Stage changes
                let add_output = run_git_command(&path, &["add", "."]);
                if !add_output.status.success() {
                    eprintln!("  {} Failed to stage changes", "ERROR:".bright_red().bold());
                    eprintln!("{}", String::from_utf8_lossy(&add_output.stderr));
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
                    eprintln!(
                        "  {} Failed to commit changes",
                        "ERROR:".bright_red().bold()
                    );
                    eprintln!("{}", String::from_utf8_lossy(&commit_output.stderr));
                    continue;
                }

                eprintln!("  {} Changes committed", "SUCCESS:".green().bold());
            } else {
                eprintln!("  Skipping repository");
                continue;
            }
        } else {
            eprintln!("  {} No changes to commit", "INFO:".bright_blue().bold());
        }

        // Push changes
        print!("  Push changes to remote? [y/N]: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if input.trim().to_lowercase() == "y" {
            let push_output = run_git_command(&path, &["push"]);
            if push_output.status.success() {
                eprintln!(
                    "  {} Successfully pushed changes",
                    "SUCCESS:".green().bold()
                );
            } else {
                eprintln!("  {} Failed to push changes", "ERROR:".bright_red().bold());
                eprintln!("{}", String::from_utf8_lossy(&push_output.stderr));
            }
        } else {
            eprintln!("  Skipping push");
        }
        eprintln!();
    }
}

fn run_git_command(path: &str, args: &[&str]) -> Output {
    Command::new("git")
        .current_dir(path)
        .args(args)
        .output()
        .expect("Failed to execute git command")
}
