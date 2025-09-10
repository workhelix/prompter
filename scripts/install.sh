#!/usr/bin/env bash
set -euo pipefail

# Install helper for prompter
# - Builds the release binary
# - Copies it to ~/.local/bin (or DEST if provided)
# Usage:
#   ./scripts/install.sh [DEST]
# Defaults:
#   DEST=$HOME/.local/bin

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${SCRIPT_DIR%/scripts}"
DEST="${1:-"$HOME/.local/bin"}"
BIN_NAME="prompter"

echo "[prompter] Building release binary..."
( cd "$REPO_ROOT" && cargo build --release )

SRC="$REPO_ROOT/target/release/$BIN_NAME"
if [[ ! -f "$SRC" ]]; then
  echo "[prompter] Build failed or binary missing at $SRC" >&2
  exit 1
fi

mkdir -p "$DEST"
cp "$SRC" "$DEST/$BIN_NAME"
chmod 0755 "$DEST/$BIN_NAME"

# PATH check
IFS=':' read -r -a PATH_ENTRIES <<<"${PATH:-}"
IN_PATH=false
for d in "${PATH_ENTRIES[@]}"; do
  if [[ "$d" == "$DEST" ]]; then IN_PATH=true; break; fi
done

echo "[prompter] Installed to $DEST/$BIN_NAME"
if ! $IN_PATH; then
  echo "[prompter] Note: $DEST is not in PATH. Add this to your shell rc:"
  echo "  export PATH=\"$DEST:\$PATH\""
fi

# Quick smoke: --version not implemented; run --list if config exists
if [[ -f "$HOME/.config/prompter/config.toml" ]]; then
  echo "[prompter] Quick smoke: running --list"
  if ! "$DEST/$BIN_NAME" --list >/dev/null 2>&1; then
    echo "[prompter] Smoke test failed (likely missing config/library). Try: $BIN_NAME --init" >&2
  fi
else
  echo "[prompter] Tip: initialize defaults with: $BIN_NAME --init"
fi
