# prompter

A small Rust CLI to compose reusable prompt snippets from a library and a TOML manifest. Designed for fast local use and simple, explicit behavior.

## Features
- Profiles in TOML under `~/.config/prompter/config.toml`
- Markdown snippets under `~/.local/prompter/library`
- Recursive profile composition with cycle detection
- Path-based deduplication (first occurrence wins)
- Deterministic depth-first order respecting `depends_on`
- Optional output separator with escape support (`\n`, `\t`, `\"`, `\\`)
- Utilities: `--list`, `--validate`, `--init`, `--version`

## Install
Prereqs: Rust toolchain with Cargo.

Option A — helper script:

```bash
./scripts/install.sh            # builds and installs to ~/.local/bin
./scripts/install.sh /opt/bin   # custom destination
```

Ensure the install directory is in your PATH, e.g.:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc  # or zshrc/fish equivalent
```

Option B — manual:

```bash
cargo build --release
install -m 0755 target/release/prompter ~/.local/bin/
```

## Initialize
Creates default config and sample library files (non-destructive: only writes if missing):

```bash
prompter --init
```

## Configure
Location: `~/.config/prompter/config.toml`

Example:

```toml
[python.api]
depends_on = ["a/b/c.md", "f/g/h.md"]

[general.testing]
depends_on = ["python.api", "a/b/d.md"]
```

- Any `depends_on` entry ending with `.md` is treated as a library file path relative to `~/.local/prompter/library`.
- Any other entry is treated as another profile and expanded recursively.

## Use

```bash
# List profiles
prompter --list

# Validate config and library references
prompter --validate

# Render a profile (concatenated file contents)
prompter python.api

# Render with a separator between files
prompter -s "\n---\n" python.api
```

## Behavior
- Missing files or unknown profiles: exits non-zero with clear errors.
- Dedup: first path occurrence included, repeats dropped.
- Order: depth-first traversal, preserves provided `depends_on` order.

## Development
- Build: `cargo build --release`
- Unit tests: `cargo test`
- Integration tests: `cargo test --test cli`

## Paths
- Config: `~/.config/prompter/config.toml`
- Library root: `~/.local/prompter/library`

## License
CC0 1.0 Universal (CC0-1.0). See LICENSE for details.
