#!/bin/bash
# Bootstrap script for building jessup from source.
# Uses kestrel directly instead of flock to avoid registry dependency.
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LANG_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROJECT_ROOT="$(cd "$LANG_DIR/.." && pwd)"

KESTREL="${KESTREL:-$PROJECT_ROOT/target/release/kestrel}"

if [ ! -f "$KESTREL" ]; then
    echo "Error: kestrel compiler not found at $KESTREL"
    echo "Build the compiler first: cargo build --release"
    exit 1
fi

echo "Bootstrapping jessup..."

# Detect platform for OpenSSL flags
PLATFORM=""
EXTRA_LINK_FLAGS=""
case "$(uname -s)" in
    Darwin) PLATFORM="darwin" ;;
    Linux)  PLATFORM="linux" ;;
    *)      echo "Unsupported platform: $(uname -s)"; exit 1 ;;
esac

OPENSSL_LIB_FLAG=""

if [ "$PLATFORM" = "darwin" ]; then
    OPENSSL_PREFIX="$(brew --prefix openssl@3 2>/dev/null || true)"
    if [ -z "$OPENSSL_PREFIX" ]; then
        echo "Error: OpenSSL 3 not found. Install with: brew install openssl@3"
        exit 1
    fi
    OPENSSL_LIB_FLAG="-L $OPENSSL_PREFIX/lib"
elif [ "$PLATFORM" = "linux" ]; then
    if pkg-config --exists openssl 2>/dev/null; then
        OPENSSL_LIB_FLAG="$(pkg-config --libs-only-L openssl)"
    fi
    EXTRA_LINK_FLAGS="-l m"
fi

"$KESTREL" build \
    --std "$LANG_DIR/std" \
    "$LANG_DIR/quill/src/error.ks" \
    "$LANG_DIR/quill/src/value.ks" \
    "$LANG_DIR/quill/src/serialize.ks" \
    "$LANG_DIR/quill/src/format.ks" \
    "$LANG_DIR/quill/src/deserialize.ks" \
    "$LANG_DIR/quill-toml/src/TomlParseError.ks" \
    "$LANG_DIR/quill-toml/src/Parser.ks" \
    "$LANG_DIR/quill-toml/src/Emitter.ks" \
    "$LANG_DIR/quill-toml/src/Toml.ks" \
    "$LANG_DIR/quill-json/src/JsonParseError.ks" \
    "$LANG_DIR/quill-json/src/Parser.ks" \
    "$LANG_DIR/quill-json/src/Emitter.ks" \
    "$LANG_DIR/quill-json/src/Json.ks" \
    "$LANG_DIR/clutch/src/ParseError.ks" \
    "$LANG_DIR/clutch/src/Argument.ks" \
    "$LANG_DIR/clutch/src/ArgumentMatches.ks" \
    "$LANG_DIR/clutch/src/Parser.ks" \
    "$LANG_DIR/clutch/src/Help.ks" \
    "$LANG_DIR/clutch/src/Command.ks" \
    "$LANG_DIR/clutch/src/Os.ks" \
    "$LANG_DIR/clutch/src/Clutch.ks" \
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
    "$SCRIPT_DIR/src/config.ks" \
    "$SCRIPT_DIR/src/error.ks" \
    "$SCRIPT_DIR/src/github.ks" \
    "$SCRIPT_DIR/src/platform.ks" \
    "$SCRIPT_DIR/src/toolchain.ks" \
    "$SCRIPT_DIR/src/main.ks" \
    -o "$SCRIPT_DIR/jessup" \
    -l ssl -l crypto \
    $OPENSSL_LIB_FLAG $EXTRA_LINK_FLAGS

echo "Done! Built jessup at $SCRIPT_DIR/jessup"
