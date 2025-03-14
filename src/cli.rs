use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RepoAction {
    NeedsStage,
    NeedsCommit,
    NeedsPush,
    UpToDate,
}

impl RepoAction {
    pub(crate) fn needs_stage(&self) -> bool {
        matches!(self, RepoAction::NeedsStage)
    }

    pub(crate) fn needs_commit(&self) -> bool {
        matches!(self, RepoAction::NeedsStage | RepoAction::NeedsCommit)
    }

    pub(crate) fn needs_push(&self) -> bool {
        matches!(
            self,
            RepoAction::NeedsStage | RepoAction::NeedsCommit | RepoAction::NeedsPush
        )
    }
}

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

#[derive(Debug)]
pub(crate) struct RepoStatus {
    pub(crate) path: Utf8PathBuf,
    pub(crate) existence: Existence,
    pub(crate) branch: String,
    pub(crate) remote: String,
    pub(crate) action: RepoAction,
}

/// Defines the mode of synchronization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SyncMode {
    Pull,
    Push,
}
