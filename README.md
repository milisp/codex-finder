# Codex Finder

A Rust library to locate the `coder` and `codex` CLI binaries on your system, including Windows, macOS, Linux, and WSL distributions.  
Useful for tools that need to programmatically find and use these binaries.

## Features

- Detect binaries in common paths (npm, bun, cargo, system PATH)
- Platform-specific support
- WSL distribution handling on Windows
- Environment variable overrides: CODER_PATH, CODEX_PATH

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
codex-finder = { git = "https://github.com/milisp/codex-finder" } # or use crates.io version when published
```

## Usage

### Rust Example

You can directly use the library in your Rust code:

```rust
use codex_finder::{discover_coder_command, discover_codex_command};

fn main() -> Result<(), String> {
    // Discover the coder binary
    if let Some(coder_path) = discover_coder_command() {
        println!("Found coder binary at {:?}", coder_path);
    } else {
        println!("Coder binary not found. Please install it or set CODER_PATH.");
    }

    // Discover the codex binary
    if let Some(codex_path) = discover_codex_command() {
        println!("Found codex binary at {:?}", codex_path);
    } else {
        println!("Codex binary not found. Please install it or set CODEX_PATH.");
    }

    Ok(())
}
```

### CLI Example

Run the provided example:

```bash
cargo run --example find_bin
```

This demonstrates how to locate coder and codex binaries on your system.

## Logging

Enable debug logging with the `log` crate and a logger, for example:

```rust
env_logger::init();
```

This prints helpful information about which paths were checked.
