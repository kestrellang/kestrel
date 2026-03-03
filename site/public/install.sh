#!/bin/sh
# Jessup installer - Kestrel version manager
#
# Usage:
#   curl --proto '=https' --tlsv1.2 -sSf https://kestrel-lang.com/install.sh | sh
#
# Options:
#   -y              Skip confirmation prompts
#   JESSUP_HOME     Override install directory (default: ~/.jessup)

set -eu

# ============================================================================
# CONFIGURATION
# ============================================================================

JESSUP_HOME="${JESSUP_HOME:-$HOME/.jessup}"
JESSUP_BIN="$JESSUP_HOME/bin"
REPO="jkpdino/kestrel"
SKIP_CONFIRM="${1:-}"

# ============================================================================
# COLORS
# ============================================================================

if [ -t 1 ]; then
    BOLD='\033[1m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    RED='\033[0;31m'
    CYAN='\033[0;36m'
    RESET='\033[0m'
else
    BOLD=''
    GREEN=''
    YELLOW=''
    RED=''
    CYAN=''
    RESET=''
fi

# ============================================================================
# HELPERS
# ============================================================================

info() {
    printf "${GREEN}info:${RESET} %s\n" "$1"
}

warn() {
    printf "${YELLOW}warn:${RESET} %s\n" "$1"
}

err() {
    printf "${RED}error:${RESET} %s\n" "$1" >&2
    exit 1
}

need_cmd() {
    if ! command -v "$1" > /dev/null 2>&1; then
        err "need '$1' (command not found)"
    fi
}

# ============================================================================
# PLATFORM DETECTION
# ============================================================================

detect_platform() {
    local _os _arch _target

    _os="$(uname -s)"
    _arch="$(uname -m)"

    case "$_os" in
        Darwin)
            _os="apple-darwin"
            ;;
        Linux)
            _os="unknown-linux"
            ;;
        *)
            err "unsupported operating system: $_os"
            ;;
    esac

    case "$_arch" in
        arm64 | aarch64)
            _arch="aarch64"
            ;;
        x86_64)
            _arch="x86_64"
            ;;
        *)
            err "unsupported architecture: $_arch"
            ;;
    esac

    PLATFORM="${_arch}-${_os}"
}

# ============================================================================
# MAIN
# ============================================================================

main() {
    need_cmd curl
    need_cmd tar
    need_cmd uname
    need_cmd mkdir
    need_cmd chmod

    detect_platform

    printf "\n"
    printf "${BOLD}${CYAN}  Jessup - Kestrel Version Manager${RESET}\n"
    printf "\n"
    printf "  This will install jessup and the latest stable kestrel toolchain.\n"
    printf "\n"
    printf "  Install location: ${BOLD}%s${RESET}\n" "$JESSUP_HOME"
    printf "  Platform:         ${BOLD}%s${RESET}\n" "$PLATFORM"
    printf "\n"

    # Confirmation prompt
    if [ "$SKIP_CONFIRM" != "-y" ]; then
        printf "Proceed with installation? [Y/n] "
        read -r _confirm
        case "$_confirm" in
            [nN]*)
                info "Installation cancelled"
                exit 0
                ;;
        esac
    fi

    # Create directories
    info "Creating $JESSUP_HOME..."
    mkdir -p "$JESSUP_BIN"
    mkdir -p "$JESSUP_HOME/toolchains"

    # Download jessup binary
    info "Downloading jessup for $PLATFORM..."

    local _url _tmpdir _archive
    _tmpdir="$(mktemp -d)"
    _archive="$_tmpdir/jessup.tar.gz"

    # Get the latest release info and find the jessup asset
    _url="https://github.com/$REPO/releases/latest/download/jessup-${PLATFORM}.tar.gz"

    if ! curl -sL -o "$_archive" "$_url"; then
        rm -rf "$_tmpdir"
        err "failed to download jessup from $_url"
    fi

    # Extract jessup binary (strip the top-level directory)
    tar xzf "$_archive" -C "$_tmpdir" --strip-components=1 2>/dev/null || {
        rm -rf "$_tmpdir"
        err "failed to extract jessup archive"
    }

    # Install jessup binary
    if [ -f "$_tmpdir/jessup" ]; then
        mv "$_tmpdir/jessup" "$JESSUP_BIN/jessup"
    else
        rm -rf "$_tmpdir"
        err "jessup binary not found in archive"
    fi

    chmod +x "$JESSUP_BIN/jessup"
    rm -rf "$_tmpdir"

    info "Installed jessup to $JESSUP_BIN/jessup"

    # Add to PATH
    add_to_path

    # Install latest stable toolchain
    info "Installing latest stable toolchain..."
    "$JESSUP_BIN/jessup" install stable

    printf "\n"
    printf "${BOLD}${GREEN}  Jessup installed successfully!${RESET}\n"
    printf "\n"
    printf "  To get started, either restart your shell or run:\n"
    printf "\n"
    printf "    ${BOLD}export PATH=\"%s:\$PATH\"${RESET}\n" "$JESSUP_BIN"
    printf "\n"
    printf "  Then:\n"
    printf "\n"
    printf "    ${BOLD}kestrel --help${RESET}      Run the Kestrel compiler\n"
    printf "    ${BOLD}jessup list${RESET}         Show installed toolchains\n"
    printf "    ${BOLD}jessup --help${RESET}       Show jessup commands\n"
    printf "\n"
}

# ============================================================================
# PATH SETUP
# ============================================================================

add_to_path() {
    local _path_entry
    _path_entry="export PATH=\"$JESSUP_BIN:\$PATH\""

    # Check if already in PATH
    case ":$PATH:" in
        *":$JESSUP_BIN:"*)
            return
            ;;
    esac

    # Try to add to shell profile
    local _added=false

    for _profile in "$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.profile"; do
        if [ -f "$_profile" ]; then
            # Check if already added
            if ! grep -q "jessup" "$_profile" 2>/dev/null; then
                printf "\n# Jessup (Kestrel version manager)\n%s\n" "$_path_entry" >> "$_profile"
                info "Added jessup to PATH in $_profile"
                _added=true
            fi
        fi
    done

    if [ "$_added" = "false" ]; then
        # Create .profile if nothing exists
        if [ ! -f "$HOME/.profile" ] && [ ! -f "$HOME/.bashrc" ] && [ ! -f "$HOME/.zshrc" ]; then
            printf "# Jessup (Kestrel version manager)\n%s\n" "$_path_entry" >> "$HOME/.profile"
            info "Added jessup to PATH in $HOME/.profile"
        fi
    fi
}

main "$@"
