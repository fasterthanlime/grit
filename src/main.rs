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
use cli::{Args, ChangeStatus, Commands, Existence, PullStatus, PushStatus, RepoStatus, SyncMode};
use config::read_repos_from_default_config;
use eyre::Context;
use owo_colors::OwoColorize;
use plan::ExecutionPlan;
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
    let mut repo_statuses = Vec::new();

    for repo in &repos {
        let status = get_repo_status(repo, &mode).await?;
        repo_statuses.push(status);
    }

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
