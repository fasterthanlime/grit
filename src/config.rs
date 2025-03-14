// Rules:
// 1. Always use eprintln!(), not println!()
// 2. Be friendly with colors and emojis but not too uppity
// 3. FIRST come up with a plan, gathering all the data, THEN apply it
// 4. Ask for consent before applying the plan, showing the exact commands to run
// 5. When skipping a repo, explain why (couldn't parse git-rev, etc.)
// 6. Better to panic if git output isn't as expected than to do harmful things
// 7. When printing specific values, like paths, numbers, keywords like "yes" and "no", use colors suited to the theme

use camino::{Utf8Path, Utf8PathBuf};
use eyre::WrapErr;
use owo_colors::OwoColorize;
use std::io::{self, Write};
use std::process::Command;

/// Returns the path to the grit configuration file.
pub fn get_config_path() -> String {
    shellexpand::tilde("~/.config/grit.conf").to_string()
}

/// Reads and parses the repositories from the given configuration file path.
///
/// # Arguments
///
/// * `config_path` - A string slice that holds the path to the configuration file
///
/// # Returns
///
/// A Result containing a vector of Utf8PathBuf representing the repository paths
fn read_repos_from_config(config_path: &str) -> eyre::Result<Vec<Utf8PathBuf>> {
    let config_file = Utf8PathBuf::from(config_path);

    if !config_file.exists() {
        eprintln!("Config file not found at {}", config_path.bright_cyan());
        eprint!(
            "Do you want to create a default config file? ({}/{}): ",
            "yes".green(),
            "no".red()
        );
        io::stdout().flush().wrap_err("Failed to flush stdout")?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .wrap_err("Failed to read input")?;

        if input.trim().to_lowercase() != "yes" {
            eprintln!("Exiting without creating config file.");
            std::process::exit(0);
        }

        create_default_config(&config_file)?;
        eprintln!(
            "Default config file created at {}",
            config_path.bright_cyan()
        );

        eprint!("Enter your favorite text editor to open the config file: ");
        io::stdout().flush().wrap_err("Failed to flush stdout")?;

        let mut editor = String::new();
        io::stdin()
            .read_line(&mut editor)
            .wrap_err("Failed to read input")?;
        let editor = editor.trim();

        eprintln!(
            "Opening config file with {}. Press Ctrl+C to quit now if you don't want to proceed.",
            editor
        );

        Command::new(editor)
            .arg(&config_file)
            .status()
            .wrap_err("Failed to open editor")?;

        eprintln!("Config file has been opened. The program will now exit. Please run the command again after editing the config file.");
        std::process::exit(0);
    }

    let content = std::fs::read_to_string(&config_file).wrap_err_with(|| {
        format!(
            "Failed to read config file at {}",
            config_path.bright_cyan()
        )
    })?;
    parse_config_content(&content)
}

/// Parses the content of the configuration file.
///
/// # Arguments
///
/// * `content` - A string slice containing the configuration file content
///
/// # Returns
///
/// A Result containing a vector of Utf8PathBuf representing the repository paths
fn parse_config_content(content: &str) -> eyre::Result<Vec<Utf8PathBuf>> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let parts: Vec<&str> = trimmed.splitn(2, '#').collect();
            let path = parts[0].trim();
            if path.is_empty() {
                None
            } else {
                Some(Ok(Utf8PathBuf::from(shellexpand::tilde(path).to_string())))
            }
        })
        .collect()
}

/// Creates a default configuration file at the specified path.
///
/// # Arguments
///
/// * `config_file` - A reference to a Utf8Path where the default config should be created
///
/// # Returns
///
/// A Result indicating success or failure of the file creation
fn create_default_config(config_file: &Utf8Path) -> eyre::Result<()> {
    let example_config = r#"# Grit configuration file
# List one repository path per line, e.g.:
# /home/user/projects/repo1
# /home/user/projects/repo2
# ~/Documents/github/my-project
"#;

    std::fs::write(config_file, example_config)?;
    Ok(())
}

pub(crate) fn read_repos_from_default_config() -> eyre::Result<Vec<Utf8PathBuf>> {
    let config_path = get_config_path();
    read_repos_from_config(&config_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config_content_with_comments_and_empty_lines() -> eyre::Result<()> {
        let content = r#"
# This is a comment
/path/to/repo1 # with comment

/path/to/repo2
~/path/to/repo3
"#;
        let repos = parse_config_content(content)?;

        assert_eq!(repos.len(), 3);
        assert_eq!(repos[0], Utf8PathBuf::from("/path/to/repo1"));
        assert_eq!(repos[1], Utf8PathBuf::from("/path/to/repo2"));
        assert_eq!(
            repos[2],
            Utf8PathBuf::from(shellexpand::tilde("~/path/to/repo3").to_string())
        );
        Ok(())
    }

    #[test]
    fn test_parse_config_content_empty_file() -> eyre::Result<()> {
        let content = "";
        let repos = parse_config_content(content)?;
        assert!(repos.is_empty());
        Ok(())
    }

    #[test]
    fn test_parse_config_content_only_comments_and_empty_lines() -> eyre::Result<()> {
        let content = r#"
# This is a comment

# Another comment
"#;
        let repos = parse_config_content(content)?;
        assert!(repos.is_empty());
        Ok(())
    }
}
