// Rules:
// 1. Always use eprintln!(), not println!()
// 2. Be friendly with colors and emojis but not too uppity
// 3. FIRST come up with a plan, gathering all the data, THEN apply it
// 4. Ask for consent before applying the plan, showing the exact commands to run
// 5. When skipping a repo, explain why (couldn't parse git-rev, etc.)
// 6. Better to panic if git output isn't as expected than to do harmful things
// 7. When printing specific values, like paths, numbers, keywords like "yes" and "no", use colors suited to the theme

use camino::Utf8Path;
use clap::Parser;
use cli::{Args, Commands, RepoStatus, SyncMode};
use config::read_repos_from_default_config;
use eyre::Context;
use futures_util::StreamExt;
use owo_colors::OwoColorize;
use plan::{ExecutionPlan, RepoAction};
use std::io::{self, Write};

mod cli;
mod config;
mod git;
mod plan;

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
        .filter_map(|status| async move { status })
        .collect::<Vec<_>>()
        .await;

    // First, create the plan from all gathered data
    let plan = ExecutionPlan::new(repo_statuses, mode);

    // If the plan is a no-op, we don't need to ask for consent
    if plan.is_noop() {
        let marine_emojis = ["🐠", "🐡", "🦈", "🐙", "🦀", "🐚", "🐳", "🐬", "🦭", "🐟"];
        let random_emoji = marine_emojis[fastrand::usize(..marine_emojis.len())];

        eprintln!("\n");
        eprintln!("{}", "========================================".cyan());
        eprintln!(
            "{random_emoji} {} {random_emoji}",
            "Everything's up to date!".green().bold(),
        );
        eprintln!("{}", "You're good to go.".blue());
        eprintln!("{}", "Have a nice day!".magenta());
        eprintln!("{}", "========================================".cyan());
        eprintln!("\n");
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

async fn get_repo_status(path: &Utf8Path, mode: &SyncMode) -> Option<RepoStatus> {
    if !path.exists() || !path.join(".git").is_dir() {
        eprintln!(
            "  {} {} is not a valid git repository",
            "⚠️".yellow(),
            path.to_string().bright_cyan()
        );
        return None;
    }

    let branch = match git::assert_git_command(path, &["rev-parse", "--abbrev-ref", "HEAD"]).await {
        Ok(output) => output.stdout.trim().to_string(),
        Err(e) => {
            eprintln!(
                "  {} Failed to get branch for {}: {}",
                "⚠️".yellow(),
                path.to_string().bright_cyan(),
                e
            );
            return None;
        }
    };

    let remote = match git::assert_git_command(path, &["remote", "get-url", "origin"]).await {
        Ok(output) => output.stdout.trim().to_string(),
        Err(e) => {
            eprintln!(
                "  {} Failed to get remote for {}: {}",
                "⚠️".yellow(),
                path.to_string().bright_cyan(),
                e
            );
            return None;
        }
    };

    let action = match mode {
        SyncMode::Push => {
            let status_output =
                match git::assert_git_command(path, &["status", "--porcelain"]).await {
                    Ok(output) => output,
                    Err(e) => {
                        eprintln!(
                            "  {} Failed to get status for {}: {}",
                            "⚠️".yellow(),
                            path.to_string().bright_cyan(),
                            e
                        );
                        return None;
                    }
                };

            let staged_output =
                match git::run_git_command_allow_failure(path, &["diff", "--cached", "--quiet"])
                    .await
                {
                    Ok(output) => output,
                    Err(e) => {
                        eprintln!(
                            "  {} Failed to check staged changes for {}: {}",
                            "⚠️".yellow(),
                            path.to_string().bright_cyan(),
                            e
                        );
                        return None;
                    }
                };

            let rev_list_output =
                match git::assert_git_command(path, &["rev-list", "@{u}..HEAD"]).await {
                    Ok(output) => output,
                    Err(e) => {
                        eprintln!(
                            "  {} Failed to check unpushed commits for {}: {}",
                            "⚠️".yellow(),
                            path.to_string().bright_cyan(),
                            e
                        );
                        return None;
                    }
                };

            if !status_output.stdout.trim().is_empty() {
                RepoAction::Stage
            } else if staged_output.status.code() == Some(1) {
                RepoAction::Commit
            } else if !rev_list_output.stdout.trim().is_empty() {
                RepoAction::Push
            } else {
                return None;
            }
        }
        SyncMode::Pull => {
            let fetch_output = match git::assert_git_command(path, &["fetch", "--all"]).await {
                Ok(output) => output,
                Err(e) => {
                    eprintln!(
                        "  {} Failed to fetch changes for {}: {}",
                        "⚠️".yellow(),
                        path.to_string().bright_cyan(),
                        e
                    );
                    return None;
                }
            };

            if !fetch_output.stderr.is_empty() {
                eprintln!("  {} Failed to fetch changes", "⚠️".yellow());
                eprintln!("{}", fetch_output.stderr);
            }

            let rev_list_output =
                match git::assert_git_command(path, &["rev-list", "HEAD..@{u}"]).await {
                    Ok(output) => output,
                    Err(e) => {
                        eprintln!(
                            "  {} Failed to check for updates in {}: {}",
                            "⚠️".yellow(),
                            path.to_string().bright_cyan(),
                            e
                        );
                        return None;
                    }
                };

            if rev_list_output.stdout.trim().is_empty() {
                return None;
            } else {
                RepoAction::Pull
            }
        }
    };

    Some(RepoStatus {
        path: path.to_owned(),
        branch,
        remote,
        action,
    })
}
