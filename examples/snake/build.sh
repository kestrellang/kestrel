#!/bin/bash
# Build script for Snake example
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Building Snake..."

# Compile the C input handler
cc -c "$SCRIPT_DIR/snake_input.c" -o "$SCRIPT_DIR/snake_input.o"

# Build with Kestrel
"$PROJECT_ROOT/target/release/kestrel" build "$SCRIPT_DIR/snake.ks" -l ":$SCRIPT_DIR/snake_input.o" -o "$SCRIPT_DIR/snake"

# Clean up object file
rm -f "$SCRIPT_DIR/snake_input.o"

echo "Done! Run with: $SCRIPT_DIR/snake"
