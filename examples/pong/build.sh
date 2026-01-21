#!/bin/bash
# Build script for Pong example
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Building Pong..."

# Compile the C input handler
cc -c "$SCRIPT_DIR/pong_input.c" -o "$SCRIPT_DIR/pong_input.o"

# Build with Kestrel
"$PROJECT_ROOT/target/release/kestrel" build "$SCRIPT_DIR/pong.ks" -l ":$SCRIPT_DIR/pong_input.o" -o "$SCRIPT_DIR/pong"

# Clean up object file
rm -f "$SCRIPT_DIR/pong_input.o"

echo "Done! Run with: $SCRIPT_DIR/pong"
