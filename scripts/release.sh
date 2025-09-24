#!/usr/bin/env bash
set -euo pipefail

# Automated release script for prompter
# Usage: ./scripts/release.sh [patch|minor|major]
#
# This script provides a complete automated release workflow:
# 1. Validates current state (clean working directory, up-to-date with remote)
# 2. Runs quality checks (tests, lints, security audits)
# 3. Bumps version atomically using versioneer
# 4. Creates commit with version bump
# 5. Creates matching git tag
# 6. Pushes commits and tags to trigger GitHub Actions release

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${SCRIPT_DIR%/scripts}"
BUMP_TYPE="${1:-patch}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Validate bump type
case "$BUMP_TYPE" in
    patch|minor|major)
        log_info "Preparing $BUMP_TYPE version release..."
        ;;
    *)
        log_error "Invalid bump type: $BUMP_TYPE"
        echo "Usage: $0 [patch|minor|major]"
        exit 1
        ;;
esac

# Change to repo root
cd "$REPO_ROOT"

# Validate prerequisites
log_info "Validating prerequisites..."

if ! command -v versioneer >/dev/null 2>&1; then
    log_error "versioneer is required but not installed"
    log_info "Install it with: cargo install versioneer"
    exit 1
fi

if ! git rev-parse --git-dir >/dev/null 2>&1; then
    log_error "Not in a git repository"
    exit 1
fi

# Check working directory is clean
if ! git diff-index --quiet HEAD --; then
    log_error "Working directory is not clean. Please commit or stash changes."
    git status --short
    exit 1
fi

# Check we're on main branch
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "main" ]; then
    log_error "Must be on main branch for release (currently on: $CURRENT_BRANCH)"
    exit 1
fi

# Check we're up to date with remote
log_info "Checking remote sync status..."
git fetch origin main
LOCAL=$(git rev-parse HEAD)
REMOTE=$(git rev-parse origin/main)
if [ "$LOCAL" != "$REMOTE" ]; then
    log_error "Local main branch is not up-to-date with origin/main"
    log_info "Run: git pull origin main"
    exit 1
fi

# Verify version files are synchronized
log_info "Verifying version synchronization..."
if ! versioneer verify >/dev/null 2>&1; then
    log_error "Version files are not synchronized"
    log_info "Run: versioneer sync"
    exit 1
fi

# Get current version
CURRENT_VERSION=$(versioneer show)
log_info "Current version: $CURRENT_VERSION"

# Run quality checks
log_info "Running quality checks..."

log_info "Running tests..."
cargo test --all --verbose

log_info "Running lints..."
cargo clippy --all-targets --all-features -- -D warnings

log_info "Running security audit..."
cargo audit

log_info "Checking dependencies..."
cargo deny check

log_success "All quality checks passed"

# Bump version
log_info "Bumping $BUMP_TYPE version..."
versioneer "$BUMP_TYPE"

NEW_VERSION=$(versioneer show)
log_success "Version bumped: $CURRENT_VERSION → $NEW_VERSION"

# Verify the bump worked correctly
if ! versioneer verify >/dev/null 2>&1; then
    log_error "Version synchronization failed after bump"
    exit 1
fi

# Create commit
log_info "Creating version bump commit..."
git add Cargo.toml Cargo.lock VERSION
git commit -m "chore: bump version to $NEW_VERSION

Release $NEW_VERSION with automated version management workflow.

🤖 Generated with automated release script"

# Create tag
log_info "Creating git tag..."
versioneer tag

# Verify tag matches version
TAG_VERSION=$(git describe --exact-match --tags HEAD | sed "s/prompter-v//")
if [ "$TAG_VERSION" != "$NEW_VERSION" ]; then
    log_error "Created tag version ($TAG_VERSION) doesn't match expected version ($NEW_VERSION)"
    exit 1
fi

log_success "Created tag: prompter-v$NEW_VERSION"

# Final confirmation
echo
log_info "Ready to push release:"
log_info "  Version: $NEW_VERSION"
log_info "  Tag: prompter-v$NEW_VERSION"
log_info "  This will trigger GitHub Actions to create the release"
echo

if [ -t 0 ]; then  # Check if we have a TTY (interactive)
    echo -n "Push release to GitHub? [y/N]: "
    read -r response
    case "$response" in
        [yY]|[yY][eE][sS])
            ;;
        *)
            log_info "Release preparation complete but not pushed"
            log_info "To push manually: git push origin main && git push --tags"
            exit 0
            ;;
    esac
fi

# Push commits and tags
log_info "Pushing to GitHub..."
git push origin main
git push --tags

log_success "Release $NEW_VERSION pushed successfully!"
log_info "Monitor the release at: https://github.com/workhelix/prompter/actions"
log_info "Release will be available at: https://github.com/workhelix/prompter/releases/tag/prompter-v$NEW_VERSION"