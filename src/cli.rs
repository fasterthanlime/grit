use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};

/// Program to keep git repositories in sync between computers
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Args {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

/// Commands available for the sync operation
#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Pull latest changes for all repositories
    Pull,
    /// Push local changes for all repositories
    Push,
}

/// Represents whether a repository exists or not
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Existence {
    Exists,
    DoesNotExist,
}

/// Indicates whether a repository has local changes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChangeStatus {
    HasChanges,
    NoChanges,
}

/// Represents the status of pulling changes from remote
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PullStatus {
    NeedsPull,
    UpToDate,
}

/// Represents the status of pushing changes to remote
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PushStatus {
    NeedsPush,
    UpToDate,
}

#[derive(Debug)]
pub(crate) struct RepoStatus {
    /// Path to the repository
    pub(crate) path: Utf8PathBuf,
    /// Whether the repository exists or not
    pub(crate) existence: Existence,
    /// Current branch of the repository
    pub(crate) branch: String,
    /// Remote URL of the repository
    pub(crate) remote: String,
    /// Status of local changes in the repository
    pub(crate) change_status: ChangeStatus,
    /// Status of pulling changes from remote
    pub(crate) pull_status: PullStatus,
    /// Status of pushing changes to remote
    pub(crate) push_status: PushStatus,
}

/// Defines the mode of synchronization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SyncMode {
    Pull,
    Push,
}
