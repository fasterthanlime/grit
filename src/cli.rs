use clap::{Parser, Subcommand};

/// Program to keep git repositories in sync between computers
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Args {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Pull latest changes for all repositories
    Pull,
    /// Push local changes for all repositories
    Push,
}

#[derive(Debug)]
pub(crate) enum Existence {
    Exists,
    DoesNotExist,
}

#[derive(Debug)]
pub(crate) enum ChangeStatus {
    HasChanges,
    NoChanges,
}

#[derive(Debug)]
pub(crate) enum PullStatus {
    NeedsPull,
    UpToDate,
}

#[derive(Debug)]
pub(crate) enum PushStatus {
    NeedsPush,
    UpToDate,
}

#[derive(Debug)]
pub(crate) struct RepoStatus {
    pub(crate) path: Utf8PathBuf,
    pub(crate) existence: Existence,
    pub(crate) branch: String,
    pub(crate) remote: String,
    pub(crate) change_status: ChangeStatus,
    pub(crate) pull_status: PullStatus,
    pub(crate) push_status: PushStatus,
}

#[derive(Debug)]
pub(crate) enum SyncMode {
    Pull,
    Push,
}

#[tokio::main(flavor = "current_thread")]
