# prompter - Development Workflow
# Requires: just, peter-hook, versioneer

# Default recipe to display available commands
default:
    @just --list

# Setup development environment
setup:
    @echo "Setting up prompter development environment..."
    @just install-hooks
    @echo "✅ Setup complete!"

# Install git hooks using peter-hook
install-hooks:
    @echo "Installing git hooks with peter-hook..."
    @if command -v peter-hook >/dev/null 2>&1; then \
        peter-hook install; \
        echo "✅ Git hooks installed"; \
    else \
        echo "❌ peter-hook not found. Install with: cargo install peter-hook"; \
        exit 1; \
    fi

# Version management
version-show:
    @echo "Current version: $(cat VERSION)"
    @echo "Cargo.toml version: $(grep '^version' Cargo.toml | cut -d'"' -f2)"

# Bump version (patch|minor|major)
bump-version level:
    @echo "Bumping {{ level }} version..."
    @if command -v versioneer >/dev/null 2>&1; then \
        versioneer {{ level }}; \
        echo "✅ Version bumped to: $(cat VERSION)"; \
    else \
        echo "❌ versioneer not found. Install with: cargo install versioneer"; \
        exit 1; \
    fi

# Release workflow with comprehensive validation (replaces scripts/release.sh)
release level:
    #!/usr/bin/env bash
    set -euo pipefail

    # Validate bump type
    case "{{ level }}" in
        patch|minor|major) ;;
        *)
            echo "❌ Invalid bump type: {{ level }}"
            echo "Usage: just release [patch|minor|major]"
            exit 1
            ;;
    esac

    echo "🚀 Starting release workflow for prompter..."
    echo ""

    # Prerequisites validation
    echo "Step 1: Validating prerequisites..."

    if ! command -v versioneer >/dev/null 2>&1; then
        echo "❌ versioneer is required but not installed"
        echo "Install it with: cargo install versioneer"
        exit 1
    fi

    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        echo "❌ Not in a git repository"
        exit 1
    fi

    # Check working directory is clean
    if ! git diff-index --quiet HEAD --; then
        echo "❌ Working directory is not clean. Please commit or stash changes."
        git status --short
        exit 1
    fi

    # Check we're on main branch
    CURRENT_BRANCH=$(git branch --show-current)
    if [ "$CURRENT_BRANCH" != "main" ]; then
        echo "❌ Must be on main branch for release (currently on: $CURRENT_BRANCH)"
        exit 1
    fi

    # Check we're up to date with remote
    git fetch origin main
    LOCAL=$(git rev-parse HEAD)
    REMOTE=$(git rev-parse origin/main)
    if [ "$LOCAL" != "$REMOTE" ]; then
        echo "❌ Local main branch is not up-to-date with origin/main"
        echo "Run: git pull origin main"
        exit 1
    fi

    # Verify version files are synchronized
    if ! versioneer verify >/dev/null 2>&1; then
        echo "❌ Version files are not synchronized"
        echo "Run: versioneer sync"
        exit 1
    fi

    CURRENT_VERSION=$(cat VERSION)
    echo "✅ Prerequisites validated"
    echo "   Current version: $CURRENT_VERSION"
    echo ""

    # Quality gates
    echo "Step 2: Running quality gates..."
    echo "   → Running tests..."
    just test
    echo "   → Running security audit..."
    just audit
    echo "   → Running dependency checks..."
    just deny
    echo "   → Running format + lint checks..."
    just pre-commit
    echo "✅ All quality gates passed"
    echo ""

    # Version management
    echo "Step 3: Bumping {{ level }} version..."
    versioneer {{ level }}
    NEW_VERSION=$(cat VERSION)

    # Verify the bump worked
    if ! versioneer verify >/dev/null 2>&1; then
        echo "❌ Version synchronization failed after bump"
        exit 1
    fi

    echo "✅ Version bumped: $CURRENT_VERSION → $NEW_VERSION"
    echo ""

    # Create commit
    echo "Step 4: Committing changes..."
    git add Cargo.toml Cargo.lock VERSION
    git commit -m "chore: bump version to $NEW_VERSION"
    echo "✅ Changes committed"
    echo ""

    # Create tag
    echo "Step 5: Creating git tag..."
    versioneer tag --tag-format "prompter-v{version}"

    # Verify tag matches version
    TAG_VERSION=$(git describe --exact-match --tags HEAD | sed "s/prompter-v//")
    if [ "$TAG_VERSION" != "$NEW_VERSION" ]; then
        echo "❌ Created tag version ($TAG_VERSION) doesn't match expected version ($NEW_VERSION)"
        exit 1
    fi
    echo "✅ Created tag: prompter-v$NEW_VERSION"
    echo ""

    # Interactive confirmation
    echo "📋 Release Summary:"
    echo "   Version: $NEW_VERSION"
    echo "   Tag: prompter-v$NEW_VERSION"
    echo "   Branch: main"
    echo "   This will trigger GitHub Actions to create the release"
    echo ""

    if [ -t 0 ]; then
        echo -n "Push release to GitHub? [y/N]: "
        read -r response
        case "$response" in
            [yY]|[yY][eE][sS]) ;;
            *)
                echo ""
                echo "ℹ️  Release prepared but not pushed"
                echo "   To push manually: git push origin main && git push --tags"
                exit 0
                ;;
        esac
    fi

    # Push
    echo ""
    echo "Step 6: Pushing to GitHub..."
    git push origin main
    git push --tags
    echo ""
    echo "🎉 Release $NEW_VERSION pushed successfully!"
    echo "   Monitor: https://github.com/workhelix/prompter/actions"
    echo "   Release: https://github.com/workhelix/prompter/releases/tag/prompter-v$NEW_VERSION"

# Clean build artifacts
clean:
    @echo "Cleaning build artifacts..."
    cargo clean
    @rm -rf target/
    @echo "✅ Clean complete!"

# Build in debug mode
build:
    @echo "Building prompter..."
    cargo build
    @echo "✅ Build complete!"

# Build in release mode
build-release:
    @echo "Building prompter in release mode..."
    cargo build --release
    @echo "✅ Release build complete!"

# Run tests
test:
    @echo "Running tests..."
    cargo test --all --verbose
    @echo "✅ Tests complete!"

# Code quality checks
quality: pre-commit pre-push

# Run pre-commit hooks (format-check + clippy-check)
pre-commit:
    @if command -v peter-hook >/dev/null 2>&1; then \
        peter-hook run pre-commit; \
    else \
        echo "❌ peter-hook not found. Install with: cargo install peter-hook"; \
        exit 1; \
    fi

# Run pre-push hooks (test-all + security-audit + version-sync-check + tag-version-check)
pre-push:
    @if command -v peter-hook >/dev/null 2>&1; then \
        peter-hook run pre-push; \
    else \
        echo "❌ peter-hook not found. Install with: cargo install peter-hook"; \
        exit 1; \
    fi

# Format code (requires nightly rustfmt)
format:
    @echo "Formatting code..."
    @if rustup toolchain list | grep -q nightly; then \
        cargo +nightly fmt; \
        echo "✅ Code formatted"; \
    else \
        echo "❌ Nightly toolchain required for formatting"; \
        echo "Install with: rustup install nightly"; \
        exit 1; \
    fi

# Check code formatting
format-check:
    @just pre-commit
    @just pre-push

# Lint code with clippy
lint:
    @just pre-commit
    @just pre-push

# Security audit
audit:
    @echo "Running security audit..."
    @if command -v cargo-audit >/dev/null 2>&1; then \
        cargo audit; \
        echo "✅ Security audit passed"; \
    else \
        echo "❌ cargo-audit not found. Install with: cargo install cargo-audit"; \
        exit 1; \
    fi

# Dependency compliance check
deny:
    @echo "Checking dependency compliance..."
    @if command -v cargo-deny >/dev/null 2>&1; then \
        cargo deny check; \
        echo "✅ Dependency compliance check passed"; \
    else \
        echo "❌ cargo-deny not found. Install with: cargo install cargo-deny"; \
        exit 1; \
    fi

# Full CI pipeline
ci: quality test build-release
    @echo "✅ Full CI pipeline complete!"

# Development workflow - quick checks before commit
dev: format pre-commit test
    @echo "✅ Development checks complete! Ready to commit."

# Run the built binary
run *args:
    cargo run -- {{ args }}

# Run the binary with release optimizations
run-release *args:
    cargo run --release -- {{ args }}
