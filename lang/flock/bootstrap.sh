#!/bin/bash
# Bootstrap script for building flock from source.
# This is needed for the initial build since flock can't build itself yet.
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LANG_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROJECT_ROOT="$(cd "$LANG_DIR/.." && pwd)"

KESTREL="$PROJECT_ROOT/target/release/kestrel"

if [ ! -f "$KESTREL" ]; then
    echo "Error: kestrel compiler not found at $KESTREL"
    echo "Build the compiler first: cargo build --release"
    exit 1
fi

echo "Bootstrapping flock..."

# Detect platform
PLATFORM=""
EXTRA_LINK_FLAGS=""
case "$(uname -s)" in
    Darwin) PLATFORM="darwin" ;;
    Linux)  PLATFORM="linux" ;;
    *)      echo "Unsupported platform: $(uname -s)"; exit 1 ;;
esac

# Resolve OpenSSL path (needed by swoop)
OPENSSL_PREFIX=""
OPENSSL_LIB_FLAG=""

if [ "$PLATFORM" = "darwin" ]; then
    OPENSSL_PREFIX="$(brew --prefix openssl@3 2>/dev/null || true)"
    if [ -z "$OPENSSL_PREFIX" ]; then
        echo "Error: OpenSSL 3 not found. Install with: brew install openssl@3"
        exit 1
    fi
    OPENSSL_LIB_FLAG="-L $OPENSSL_PREFIX/lib"
elif [ "$PLATFORM" = "linux" ]; then
    # On Linux, OpenSSL is typically in the default library path
    if pkg-config --exists openssl 2>/dev/null; then
        OPENSSL_LIB_FLAG="$(pkg-config --libs-only-L openssl)"
    fi
    # Default path works if libssl-dev is installed
    # Linux needs explicit libm for math functions
    EXTRA_LINK_FLAGS="-l m"
fi

# Dependency packages, in dependency order. Keep this list in sync with the
# [dependencies] table in flock.toml. Every `.ks` under each package's `src/`
# is compiled — globbing instead of listing individual files means renamed or
# added sources are picked up automatically (the old hand-maintained list drifted
# out of sync, e.g. swoop's body.ks -> content.ks).
DEP_PACKAGES="quill quill-toml quill-json clutch http swoop"

SOURCES=()
for pkg in $DEP_PACKAGES; do
    for f in "$LANG_DIR/$pkg/src"/*.ks; do
        SOURCES+=("$f")
    done
done
# flock's own sources (compiled last).
for f in "$SCRIPT_DIR/src"/*.ks; do
    SOURCES+=("$f")
done

"$KESTREL" build \
    --std "$LANG_DIR/std" \
    "${SOURCES[@]}" \
    -o "$SCRIPT_DIR/flock" \
    -l ssl -l crypto \
    $OPENSSL_LIB_FLAG $EXTRA_LINK_FLAGS

echo "Done! Built flock at $SCRIPT_DIR/flock"
echo ""
echo "To use flock globally, add it to your PATH or copy:"
echo "  cp $SCRIPT_DIR/flock /usr/local/bin/"
