#!/bin/bash
# Build script for Snake example
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
GAMES_DIR="$SCRIPT_DIR/../games"

echo "Building Snake..."

# Compile the C input handler (shared)
cc -c "$GAMES_DIR/game_input.c" -o "$SCRIPT_DIR/game_input.o"

# Build with Kestrel (using shared modules from games folder)
"$PROJECT_ROOT/target/release/kestrel" build \
    "$GAMES_DIR/tui.ks" \
    "$GAMES_DIR/input.ks" \
    "$SCRIPT_DIR/snake.ks" \
    -l ":$SCRIPT_DIR/game_input.o" \
    -o "$SCRIPT_DIR/snake"

# Clean up object file
rm -f "$SCRIPT_DIR/game_input.o"

echo "Done! Run with: $SCRIPT_DIR/snake"
