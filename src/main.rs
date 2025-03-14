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
        .filter_map(|status| async move { status.ok().flatten() })
        .collect::<Vec<_>>()
        .await;

    // First, create the plan from all gathered data
    let plan = ExecutionPlan::new(repo_statuses, mode);

    // If the plan is a no-op, we don't need to ask for consent
    if plan.is_noop() {
        let marine_emojis = ["🐠", "🐡", "🦈", "🐙", "🦀", "🐚", "🐳", "🐬", "🦭", "🐟"];
        let random_emoji = marine_emojis[fastrand::usize(..marine_emojis.len())];

        let cheerful_messages = [
            "Everything's shipshape and Bristol fashion!",
            "You're on top of your game!",
            "Smooth sailing ahead!",
            "You're crushing it!",
            "High five for being up-to-date!",
            "You're a git wizard, Harry!",
            "Code so fresh, it should be illegal!",
            "Repo goals achieved!",
            "You're in sync with the universe!",
            "Git-tastic work!",
            "You've got your ducks in a row!",
            "Cleaner than a whistle!",
            "Your repo game is strong!",
            "Synced and ready to rock!",
            "You're the captain of this ship!",
            "Smooth as butter!",
            "Git-er done? More like git-er already done!",
            "You're firing on all cylinders!",
            "Repo perfection achieved!",
            "You're in the git zone!",
            "Commits so clean, they sparkle!",
            "Your repo is a thing of beauty!",
            "Git-standing work!",
            "You're a lean, mean, syncing machine!",
            "Repository bliss achieved!",
            "You're the git master!",
            "Synced to perfection!",
            "Your repo is a work of art!",
            "Git-cellent job!",
            "You're on fire (in a good way)!",
            "Repo harmony restored!",
            "You've got the Midas touch!",
            "Git-tacular performance!",
            "You're the git whisperer!",
            "Synced and fabulous!",
            "Your repo is a shining example!",
            "Git-credible work!",
            "You're in perfect harmony!",
            "Repo nirvana achieved!",
            "You're a git superhero!",
            "Synced to the nines!",
            "Your repo is poetry in motion!",
            "Git-mazing job!",
            "You're the king/queen of version control!",
            "Repo zen achieved!",
            "You've got git-game!",
            "Synced and sensational!",
            "Your repo is a masterpiece!",
            "Git-errific work!",
            "You're the git guru!",
        ];

        let message1 = cheerful_messages[fastrand::usize(..cheerful_messages.len())];
        let message2 = cheerful_messages[fastrand::usize(..cheerful_messages.len())];

        eprintln!("{}", "========================================".cyan());
        eprintln!("{} {}", random_emoji, message1.green().bold());
        eprintln!("{} {}", random_emoji, message2.blue());
        eprintln!("{}", "========================================".cyan());
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

// RULES:
// Things that are non-fatal (return Ok(None))
//   - the directory does not exist
// Things that should be fatal (return an error)
//   - the directory is not a git repo
//   - any of the git gathering commands fail
async fn get_repo_status(path: &Utf8Path, mode: &SyncMode) -> eyre::Result<Option<RepoStatus>> {
    if !path.exists() {
        eprintln!(
            "  {} {} does not exist",
            "⚠️".yellow(),
            path.to_string().bright_cyan()
        );
        return Ok(None);
    }

    if !path.join(".git").is_dir() {
        return Err(eyre::eyre!(
            "{} is not a valid git repository",
            path.to_string().red()
        ));
    }

    let branch = git::run_git_command_quiet(
        path,
        &["rev-parse", "--abbrev-ref", "HEAD"],
        git::GitCommandBehavior::AssertZeroExitCode,
    )
    .await?
    .stdout
    .trim()
    .to_string();

    let remote = git::run_git_command_quiet(
        path,
        &["remote", "get-url", "origin"],
        git::GitCommandBehavior::AssertZeroExitCode,
    )
    .await?
    .stdout
    .trim()
    .to_string();

    let action: Option<RepoAction> = match mode {
        SyncMode::Push => {
            let status_output = git::run_git_command_quiet(
                path,
                &["status", "--porcelain"],
                git::GitCommandBehavior::AssertZeroExitCode,
            )
            .await?;

            let staged_output = git::run_git_command_quiet(
                path,
                &["diff", "--cached", "--quiet"],
                git::GitCommandBehavior::AllowNonZeroExitCode,
            )
            .await?;

            let rev_list_output = git::run_git_command_quiet(
                path,
                &["rev-list", "@{u}..HEAD"],
                git::GitCommandBehavior::AssertZeroExitCode,
            )
            .await?;

            if !status_output.stdout.trim().is_empty() {
                Some(RepoAction::Stage)
            } else if staged_output.status.code() == Some(1) {
                Some(RepoAction::Commit)
            } else if !rev_list_output.stdout.trim().is_empty() {
                Some(RepoAction::Push)
            } else {
                None
            }
        }
        SyncMode::Pull => {
            let fetch_output = git::run_git_command_quiet(
                path,
                &["fetch", "--all"],
                git::GitCommandBehavior::AssertZeroExitCode,
            )
            .await?;

            if !fetch_output.stderr.is_empty() {
                eprintln!("  {} Failed to fetch changes", "⚠️".yellow());
                eprintln!("{}", fetch_output.stderr.red());
            }

            let rev_list_output = git::run_git_command_quiet(
                path,
                &["rev-list", "HEAD..@{u}"],
                git::GitCommandBehavior::AssertZeroExitCode,
            )
            .await?;

            if rev_list_output.stdout.trim().is_empty() {
                None
            } else {
                Some(RepoAction::Pull)
            }
        }
    };

    Ok(Some(RepoStatus {
        path: path.to_owned(),
        branch,
        remote,
        action,
    }))
}
