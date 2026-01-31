#!/bin/bash
# Build script for SDL Pong example
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Building SDL Pong..."

# Get SDL2 flags
SDL_CFLAGS=$(sdl2-config --cflags)
SDL_LIBS=$(sdl2-config --libs)

# Compile the C helpers
cc -c $SDL_CFLAGS "$SCRIPT_DIR/sdl_helpers.c" -o "$SCRIPT_DIR/sdl_helpers.o"

# Build with Kestrel
"$PROJECT_ROOT/target/release/kestrel" build \
    "$SCRIPT_DIR/sdl.ks" \
    "$SCRIPT_DIR/pong.ks" \
    -l ":$SCRIPT_DIR/sdl_helpers.o" \
    $SDL_LIBS \
    -o "$SCRIPT_DIR/pong"

# Clean up
rm -f "$SCRIPT_DIR/sdl_helpers.o"

echo "Done! Run with: $SCRIPT_DIR/pong"
