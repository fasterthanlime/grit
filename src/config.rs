use camino::{Utf8Path, Utf8PathBuf};
use eyre::WrapErr;
use owo_colors::OwoColorize;

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
        create_default_config(&config_file)?;
        eprintln!(
            "Default config file created at {}",
            config_path.bright_cyan()
        );
        eprintln!("Please edit the file and run the command again.");
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
