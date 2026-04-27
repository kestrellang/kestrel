#!/bin/bash
# Build script for the Game of Life example. Reuses the SDL bindings and
# C helpers from the sdl_pong example so we keep a single source of truth
# for SDL integration.
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SDL_DIR="$SCRIPT_DIR/../sdl_pong"

echo "Building Game of Life..."

SDL_CFLAGS=$(sdl2-config --cflags)
SDL_LIBS=$(sdl2-config --libs)

cc -c $SDL_CFLAGS "$SDL_DIR/sdl_helpers.c" -o "$SCRIPT_DIR/sdl_helpers.o"

"$PROJECT_ROOT/target/release/kestrel" build \
    "$SDL_DIR/sdl.ks" \
    "$SCRIPT_DIR/life.ks" \
    -l ":$SCRIPT_DIR/sdl_helpers.o" \
    $SDL_LIBS \
    -o "$SCRIPT_DIR/life"

rm -f "$SCRIPT_DIR/sdl_helpers.o"

echo "Done! Run with: $SCRIPT_DIR/life"
