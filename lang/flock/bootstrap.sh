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
    "$LANG_DIR/clutch/src/error.ks" \
    "$LANG_DIR/clutch/src/arg.ks" \
    "$LANG_DIR/clutch/src/matches.ks" \
    "$LANG_DIR/clutch/src/parser.ks" \
    "$LANG_DIR/clutch/src/help.ks" \
    "$LANG_DIR/clutch/src/command.ks" \
    "$LANG_DIR/clutch/src/os.ks" \
    "$LANG_DIR/clutch/src/clutch.ks" \
    "$SCRIPT_DIR/src/error.ks" \
    "$SCRIPT_DIR/src/version.ks" \
    "$SCRIPT_DIR/src/dependency.ks" \
    "$SCRIPT_DIR/src/manifest.ks" \
    "$SCRIPT_DIR/src/source.ks" \
    "$SCRIPT_DIR/src/discover.ks" \
    "$SCRIPT_DIR/src/graph.ks" \
    "$SCRIPT_DIR/src/compiler.ks" \
    "$SCRIPT_DIR/src/main.ks" \
    -o "$SCRIPT_DIR/flock"

echo "Done! Built flock at $SCRIPT_DIR/flock"
echo ""
echo "To use flock globally, add it to your PATH or copy:"
echo "  cp $SCRIPT_DIR/flock /usr/local/bin/"
