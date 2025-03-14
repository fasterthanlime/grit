# grit

A simple command-line tool to keep multiple git repositories in sync across different computers.

## Features

- Pull the latest changes from all your repositories with one command
- Push local changes to multiple repositories
- Interactive workflow for staging, committing, and pushing changes
- Color-coded output for better readability

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Pull the latest changes from all repositories
grit pull

# Push local changes to all repositories
grit push
```

## Configuration

Edit the `REPOS` constant in `src/main.rs` to include your repositories:

```rust
static REPOS: &[&str] = &[
    "~/projects/main-project",
    "~/projects/utils",
    "~/documents/notes",
    "~/configs",
];
```
