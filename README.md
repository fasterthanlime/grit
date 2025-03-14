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

Create a configuration file at `~/.config/grit.conf` with your repositories:

You can use hash (`#`) as comment lines in the configuration file, just like in Bash. Here's a sample configuration:

```bash
# Personal projects
~/projects/main-project
~/projects/utils

# Work-related
~/work/client-project
~/work/internal-tools

# Documentation and notes
~/documents/notes
~/configs
```
