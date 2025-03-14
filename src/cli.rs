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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SyncMode {
    Pull,
    Push,
}
