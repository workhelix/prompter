#!/bin/bash
set -euo pipefail

# prompter installation script
# Usage: curl -fsSL https://raw.githubusercontent.com/workhelix/prompter/main/install.sh | sh
# Or with custom install directory: INSTALL_DIR=/usr/local/bin curl ... | sh

TOOL_NAME="prompter"
REPO_OWNER="${REPO_OWNER:-workhelix}"
REPO_NAME="${REPO_NAME:-prompter}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
GITHUB_API_URL="https://api.github.com"
GITHUB_DOWNLOAD_URL="https://github.com"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" >&2
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" >&2
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1" >&2
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

# Detect OS and architecture
detect_platform() {
    local os arch target

    # Detect OS
    case "$(uname -s)" in
        Linux*) os="unknown-linux-gnu" ;;
        Darwin*) os="apple-darwin" ;;
        MINGW*|MSYS*|CYGWIN*) os="pc-windows-msvc" ;;
        *)
            log_error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac

    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *)
            log_error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac

    target="${arch}-${os}"
    echo "$target"
}

# Get latest release version from GitHub API
get_latest_version() {
    local api_url="$GITHUB_API_URL/repos/$REPO_OWNER/$REPO_NAME/releases/latest"

    log_info "Fetching latest release information..."

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$api_url" | grep '"tag_name":' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "$api_url" | grep '"tag_name":' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    else
        log_error "Neither curl nor wget is available. Please install one of them."
        exit 1
    fi
}

# Download and verify checksum if available
download_and_verify() {
    local download_url="$1"
    local filename="$2"
    local temp_dir="$3"

    log_info "Downloading $filename..."

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$download_url" -o "$temp_dir/$filename"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$download_url" -O "$temp_dir/$filename"
    else
        log_error "Neither curl nor wget is available."
        exit 1
    fi

    # Try to download and verify checksum if available
    local checksum_url="${download_url}.sha256"
    local checksum_file="$temp_dir/${filename}.sha256"

    if command -v curl >/dev/null 2>&1; then
        if curl -fsSL "$checksum_url" -o "$checksum_file" 2>/dev/null; then
            log_info "Verifying checksum..."
            if command -v sha256sum >/dev/null 2>&1; then
                (cd "$temp_dir" && sha256sum -c "$checksum_file") || {
                    log_error "Checksum verification failed!"
                    exit 1
                }
                log_success "Checksum verification passed"
            elif command -v shasum >/dev/null 2>&1; then
                (cd "$temp_dir" && shasum -a 256 -c "$checksum_file") || {
                    log_error "Checksum verification failed!"
                    exit 1
                }
                log_success "Checksum verification passed"
            else
                log_warn "No checksum utility available, skipping verification"
            fi
        else
            log_warn "No checksum file available, skipping verification"
        fi
    fi
}

# Extract archive based on file extension
extract_archive() {
    local archive_file="$1"
    local temp_dir="$2"

    case "$archive_file" in
        *.tar.gz|*.tgz)
            log_info "Extracting tar.gz archive..."
            tar -xzf "$temp_dir/$archive_file" -C "$temp_dir"
            ;;
        *.zip)
            log_info "Extracting zip archive..."
            if command -v unzip >/dev/null 2>&1; then
                unzip -q "$temp_dir/$archive_file" -d "$temp_dir"
            else
                log_error "unzip is not available. Please install unzip to extract the archive."
                exit 1
            fi
            ;;
        *)
            log_error "Unsupported archive format: $archive_file"
            exit 1
            ;;
    esac
}

# Check if binary needs to be replaced
check_existing_installation() {
    local install_path="$1"

    if [ -f "$install_path" ]; then
        if [ -t 0 ]; then  # Check if we have a TTY (interactive)
            echo -n "$(basename "$install_path") is already installed at $install_path. Replace it? [y/N]: "
            read -r response
            case "$response" in
                [yY]|[yY][eE][sS])
                    return 0
                    ;;
                *)
                    log_info "Installation cancelled by user"
                    exit 0
                    ;;
            esac
        else
            log_warn "$(basename "$install_path") already exists at $install_path, replacing..."
            return 0
        fi
    fi
}

main() {
    log_info "Installing $TOOL_NAME..."

    # Detect platform
    local target
    target=$(detect_platform)
    log_info "Detected platform: $target"

    # Get latest version
    local version
    version=$(get_latest_version)
    if [ -z "$version" ]; then
        log_error "Failed to get latest version"
        exit 1
    fi
    log_info "Latest version: $version"

    # Construct download URL
    local filename="${TOOL_NAME}-${target}.zip"
    local download_url="$GITHUB_DOWNLOAD_URL/$REPO_OWNER/$REPO_NAME/releases/download/$version/$filename"

    # Create temporary directory
    local temp_dir
    temp_dir=$(mktemp -d)
    trap "rm -rf \"$temp_dir\"" EXIT

    # Download and verify
    download_and_verify "$download_url" "$filename" "$temp_dir"

    # Extract archive
    extract_archive "$filename" "$temp_dir"

    # Find the binary (handle potential directory structure)
    local binary_name="$TOOL_NAME"
    if [ "$(uname -s)" = "MINGW*" ] || [ "$(uname -s)" = "MSYS*" ] || [ "$(uname -s)" = "CYGWIN*" ]; then
        binary_name="${TOOL_NAME}.exe"
    fi

    local binary_path
    if [ -f "$temp_dir/$binary_name" ]; then
        binary_path="$temp_dir/$binary_name"
    else
        # Look for binary in subdirectories
        binary_path=$(find "$temp_dir" -name "$binary_name" -type f | head -1)
        if [ -z "$binary_path" ]; then
            log_error "Could not find $binary_name in the extracted archive"
            exit 1
        fi
    fi

    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"

    # Check for existing installation
    local install_path="$INSTALL_DIR/$TOOL_NAME"
    if [ "$(uname -s)" = "MINGW*" ] || [ "$(uname -s)" = "MSYS*" ] || [ "$(uname -s)" = "CYGWIN*" ]; then
        install_path="${install_path}.exe"
    fi

    check_existing_installation "$install_path"

    # Install binary
    log_info "Installing to $install_path..."
    cp "$binary_path" "$install_path"
    chmod +x "$install_path"

    log_success "$TOOL_NAME installed successfully!"
    log_info "Binary location: $install_path"

    # Check if install directory is in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            log_success "$INSTALL_DIR is already in your PATH"
            ;;
        *)
            log_warn "$INSTALL_DIR is not in your PATH"
            log_info "Add it to your PATH by adding this line to your shell configuration file:"
            log_info "  export PATH=\"$INSTALL_DIR:\$PATH\""
            ;;
    esac

    # Test installation
    if command -v "$TOOL_NAME" >/dev/null 2>&1; then
        log_success "Installation verified: $TOOL_NAME is available"
        log_info "Version: $("$TOOL_NAME" --version 2>/dev/null || "$TOOL_NAME" version 2>/dev/null || echo "unable to determine")"
    else
        log_warn "Installation completed, but $TOOL_NAME is not immediately available"
        log_info "You may need to restart your shell or source your shell configuration"
    fi
}

main "$@"