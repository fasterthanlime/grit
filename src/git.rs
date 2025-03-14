// Rules:
// 1. Always use eprintln!(), not println!()
// 2. Be friendly with colors and emojis but not too uppity
// 3. FIRST come up with a plan, gathering all the data, THEN apply it
// 4. Ask for consent before applying the plan, showing the exact commands to run
// 5. When skipping a repo, explain why (couldn't parse git-rev, etc.)
// 6. Better to panic if git output isn't as expected than to do harmful things
// 7. When printing specific values, like paths, numbers, keywords like "yes" and "no", use colors suited to the theme

use std::process::Stdio;

use camino::Utf8Path;
use eyre::Context;
use owo_colors::OwoColorize;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

#[derive(Debug)]
pub struct GitCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: std::process::ExitStatus,
}

#[derive(Debug)]
pub enum GitCommandBehavior {
    AssertZeroExitCode,
    AllowNonZeroExitCode,
}

#[derive(Debug)]
pub enum GitCommandVerbosity {
    Verbose,
    Quiet,
}

pub(crate) async fn run_git_command(
    path: &Utf8Path,
    args: &[&str],
    behavior: GitCommandBehavior,
    verbosity: GitCommandVerbosity,
) -> eyre::Result<GitCommandOutput> {
    let mut cmd = Command::new("git");
    cmd.current_dir(path).args(args);

    if let GitCommandVerbosity::Verbose = verbosity {
        // Print the full git command
        eprintln!(
            "🚀 Running: {} {} {}",
            "git".bright_green(),
            args.join(" ").bright_cyan(),
            format!("(in {path})").bright_blue()
        );
    }

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .wrap_err("Failed to spawn git command")?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| eyre::eyre!("Failed to open stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| eyre::eyre!("Failed to open stderr"))?;

    let stdout_reader = BufReader::new(stdout).lines();
    let stderr_reader = BufReader::new(stderr).lines();

    let mut stdout_output = String::new();
    let mut stderr_output = String::new();

    let stdout_future = async {
        let mut reader = stdout_reader;
        while let Ok(Some(line)) = reader.next_line().await {
            if let GitCommandVerbosity::Verbose = verbosity {
                eprintln!("  {}", line.bright_green());
            }
            stdout_output.push_str(&line);
            stdout_output.push('\n');
        }
    };

    let stderr_future = async {
        let mut reader = stderr_reader;
        while let Ok(Some(line)) = reader.next_line().await {
            if let GitCommandVerbosity::Verbose = verbosity {
                eprintln!("  {}", line.yellow());
            }
            stderr_output.push_str(&line);
            stderr_output.push('\n');
        }
    };

    let wait_future = child.wait();

    let ((), (), result) = tokio::join!(stdout_future, stderr_future, wait_future);
    let status = result.wrap_err("Failed to wait on git command")?;

    let output = GitCommandOutput {
        stdout: stdout_output,
        stderr: stderr_output,
        status,
    };

    match behavior {
        GitCommandBehavior::AssertZeroExitCode => {
            if !output.status.success() {
                return Err(eyre::eyre!("Git command failed with non-zero exit code"));
            }
        }
        GitCommandBehavior::AllowNonZeroExitCode => {}
    }

    Ok(output)
}

pub(crate) async fn assert_git_command(
    path: &Utf8Path,
    args: &[&str],
) -> eyre::Result<GitCommandOutput> {
    run_git_command(
        path,
        args,
        GitCommandBehavior::AssertZeroExitCode,
        GitCommandVerbosity::Verbose,
    )
    .await
}

pub(crate) async fn run_git_command_quiet(
    path: &Utf8Path,
    args: &[&str],
    behavior: GitCommandBehavior,
) -> eyre::Result<GitCommandOutput> {
    run_git_command(path, args, behavior, GitCommandVerbosity::Quiet).await
}
