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

"$KESTREL" build \
    --std "$LANG_DIR/std" \
    "$LANG_DIR/quill/src/error.ks" \
    "$LANG_DIR/quill/src/value.ks" \
    "$LANG_DIR/quill/src/serialize.ks" \
    "$LANG_DIR/quill/src/format.ks" \
    "$LANG_DIR/quill/src/deserialize.ks" \
    "$LANG_DIR/quill-toml/src/error.ks" \
    "$LANG_DIR/quill-toml/src/parser.ks" \
    "$LANG_DIR/quill-toml/src/emitter.ks" \
    "$LANG_DIR/quill-toml/src/toml.ks" \
    "$LANG_DIR/quill-json/src/error.ks" \
    "$LANG_DIR/quill-json/src/parser.ks" \
    "$LANG_DIR/quill-json/src/emitter.ks" \
    "$LANG_DIR/quill-json/src/json.ks" \
    "$LANG_DIR/clutch/src/error.ks" \
    "$LANG_DIR/clutch/src/arg.ks" \
    "$LANG_DIR/clutch/src/matches.ks" \
    "$LANG_DIR/clutch/src/parser.ks" \
    "$LANG_DIR/clutch/src/help.ks" \
    "$LANG_DIR/clutch/src/command.ks" \
    "$LANG_DIR/clutch/src/os.ks" \
    "$LANG_DIR/clutch/src/clutch.ks" \
    "$LANG_DIR/http/src/method.ks" \
    "$LANG_DIR/http/src/url.ks" \
    "$LANG_DIR/http/src/status.ks" \
    "$LANG_DIR/http/src/cookie.ks" \
    "$LANG_DIR/http/src/headers.ks" \
    "$LANG_DIR/http/src/wire.ks" \
    "$LANG_DIR/swoop/src/error.ks" \
    "$LANG_DIR/swoop/src/response.ks" \
    "$LANG_DIR/swoop/src/body.ks" \
    "$LANG_DIR/swoop/src/url.ks" \
    "$LANG_DIR/swoop/src/send.ks" \
    "$LANG_DIR/swoop/src/swoop.ks" \
    "$LANG_DIR/swoop/src/tls.ks" \
    "$SCRIPT_DIR/src/error.ks" \
    "$SCRIPT_DIR/src/version.ks" \
    "$SCRIPT_DIR/src/dependency.ks" \
    "$SCRIPT_DIR/src/manifest.ks" \
    "$SCRIPT_DIR/src/source.ks" \
    "$SCRIPT_DIR/src/discover.ks" \
    "$SCRIPT_DIR/src/compiler.ks" \
    "$SCRIPT_DIR/src/registry.ks" \
    "$SCRIPT_DIR/src/registry_source.ks" \
    "$SCRIPT_DIR/src/cache.ks" \
    "$SCRIPT_DIR/src/lock.ks" \
    "$SCRIPT_DIR/src/graph.ks" \
    "$SCRIPT_DIR/src/main.ks" \
    -o "$SCRIPT_DIR/flock" \
    -l ssl -l crypto \
    $OPENSSL_LIB_FLAG $EXTRA_LINK_FLAGS

echo "Done! Built flock at $SCRIPT_DIR/flock"
echo ""
echo "To use flock globally, add it to your PATH or copy:"
echo "  cp $SCRIPT_DIR/flock /usr/local/bin/"
