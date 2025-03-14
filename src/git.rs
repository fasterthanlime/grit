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
    select,
};

#[derive(Debug)]
pub struct GitCommandOutput {
    pub stdout: String,
    pub stderr: String,
}

pub(crate) async fn run_git_command(
    path: &Utf8Path,
    args: &[&str],
) -> eyre::Result<GitCommandOutput> {
    let mut cmd = Command::new("git");
    cmd.current_dir(path).args(args);

    // Print the full git command
    eprintln!(
        "🚀 Running: {} {} {}",
        "git".bright_green(),
        args.join(" ").bright_cyan(),
        format!("(in {})", path).bright_blue()
    );

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

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let mut stdout_output = String::new();
    let mut stderr_output = String::new();

    loop {
        select! {
            line = stdout_reader.next_line() => {
                match line {
                    Ok(Some(l)) => {
                        eprintln!("  {}", l.bright_green());
                        stdout_output.push_str(&l);
                        stdout_output.push('\n');
                    },
                    Ok(None) => break,
                    Err(e) => return Err(eyre::eyre!("Error reading stdout: {}", e.to_string().red())),
                }
            }
            line = stderr_reader.next_line() => {
                match line {
                    Ok(Some(l)) => {
                        eprintln!("  {}", l.yellow());
                        stderr_output.push_str(&l);
                        stderr_output.push('\n');
                    },
                    Ok(None) => break,
                    Err(e) => return Err(eyre::eyre!("Error reading stderr: {}", e.to_string().red())),
                }
            }
            result = child.wait() => {
                result.wrap_err("Failed to wait on git command")?;
                break;
            }
        }
    }

    Ok(GitCommandOutput {
        stdout: stdout_output,
        stderr: stderr_output,
    })
}
