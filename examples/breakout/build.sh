#!/bin/bash
# Build script for Breakout example
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Building Breakout..."

# Compile the C input handler
cc -c "$SCRIPT_DIR/breakout_input.c" -o "$SCRIPT_DIR/breakout_input.o"

# Build with Kestrel
"$PROJECT_ROOT/target/release/kestrel" build "$SCRIPT_DIR/tui.ks" "$SCRIPT_DIR/input.ks" "$SCRIPT_DIR/breakout.ks" -l ":$SCRIPT_DIR/breakout_input.o" -o "$SCRIPT_DIR/breakout"

# Clean up object file
rm -f "$SCRIPT_DIR/breakout_input.o"

echo "Done! Run with: $SCRIPT_DIR/breakout"
