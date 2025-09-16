# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

- **Build:** `cargo build --release`
- **Run tests:** `cargo test` (unit tests), `cargo test --test cli` (integration tests)
- **Format:** `cargo fmt --all -- --check`
- **Lint:** `cargo clippy --all-targets --all-features -- -D warnings`
- **Security audit:** `cargo audit`
- **Dependency check:** `cargo deny check`
- **Unused deps:** `cargo machete`
- **TOML format:** `taplo format --check`
- **TOML lint:** `taplo check`
- **Coverage:** `cargo tarpaulin --all --out Xml --engine llvm --timeout 300 --fail-under 80` (Linux only)

### Aggressive Linting Setup

**Git Hooks (peter-hook):**
- Pre-commit hooks run comprehensive checks: TOML formatting, Rust formatting, compilation, aggressive clippy, security audit, dependency compliance, documentation, and full test suite
- Commit message validation enforces proper length limits
- All hooks configured in `hooks.toml`

**Quality Standards:**
- Aggressive clippy linting with pedantic and nursery lints enabled
- Comprehensive documentation required for all public APIs
- Security vulnerability scanning on every commit
- License compliance enforcement
- Zero tolerance for unsafe code (except in specifically marked tests)

## Architecture

**prompter** is a CLI tool for composing reusable prompt snippets from a library using TOML profiles.

### Core Components

- **`main.rs`**: CLI entry point, argument parsing, and mode dispatch
- **`lib.rs`**: Core logic split into several key areas:
  - **Config parsing**: Custom TOML parser that handles profiles and dependencies
  - **Profile resolution**: Recursive dependency resolution with cycle detection and deduplication
  - **File rendering**: Concatenation of resolved files with optional separators and system info prefix

### Key Data Flow

1. **Config**: Profiles map to lists of dependencies (`.md` files or other profile names)
2. **Resolution**: Depth-first traversal respects `depends_on` order, deduplicates by path (first occurrence wins)
3. **Output**: Pre-prompt + System prefix + file contents with optional separators

### Output Structure

All rendered profiles follow this format:
```
[Pre-prompt text]

[System info: "Today is YYYY-MM-DD, and you are running on a ARCH/OS system."]

[File 1 content]
[Optional separator]
[File 2 content]
[Optional separator]
...
```

The pre-prompt defaults to LLM coding agent instructions but can be customized or disabled.

### File Locations

- **Config**: `~/.config/prompter/config.toml`
- **Library**: `~/.local/prompter/library/` (markdown snippets)

### Current CLI Design (Compliant with User Standards)

Uses subcommand pattern with clap:
- `prompter <profile>` - render profile (backward compatible)
- `prompter run <profile>` - explicit render command
- `prompter list` - list profiles
- `prompter validate` - validate config
- `prompter init` - create default config/library (with progress spinner)
- `prompter version` - show version
- `prompter help` - show help (built-in)
- `prompter -s <sep> <profile>` - render with separator
- `prompter -p <text> <profile>` - render with custom pre-prompt

**Features**:
- ✅ Subcommand pattern using clap
- ✅ Built-in help and version subcommands
- ✅ PTY detection with `is-terminal` crate
- ✅ Terminal effects using `colored` and `indicatif`
- ✅ Colorful output with emojis when interactive
- ✅ Clean output when piped/redirected
- ✅ Progress spinners during operations
- ✅ Pre-prompt injection (defaults to LLM coding agent instructions)

### Testing Strategy

- **Unit tests**: Embedded in `lib.rs` with `#[cfg(test)]`, use temporary directories
- **Integration tests**: `tests/cli.rs` tests full binary with `Command::new(bin_path())`
- **Coverage**: CI enforces 80% minimum via tarpaulin

### Profile Resolution Logic

Recursive expansion with these behaviors:
- **Cycle detection**: Maintains stack to detect circular dependencies
- **Deduplication**: `HashSet<PathBuf>` ensures first occurrence wins
- **Error handling**: Clear messages for missing files, unknown profiles, cycles
- **Order preservation**: Depth-first maintains `depends_on` sequence