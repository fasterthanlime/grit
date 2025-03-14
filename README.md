# grit

A simple command-line tool to keep multiple git repositories in sync across different computers.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Might eat your dog](https://img.shields.io/badge/Might%20eat%20your%20dog-yes-red.svg)](https://shields.io/)
[![Maintenance](https://img.shields.io/badge/Maintained%3F-yes-green.svg)](https://GitHub.com/Naereen/StrapDown.js/graphs/commit-activity)
[![Made with Rust](https://img.shields.io/badge/Made%20with-Rust-1f425f.svg)](https://www.rust-lang.org/)

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

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
