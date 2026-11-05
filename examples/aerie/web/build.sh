#!/bin/bash
# Build Aerie for the browser: wasm64 through Emscripten, WebGPU through
# Dawn's emdawnwebgpu port, SDL2 through Emscripten's port.
#
#   source ~/emsdk/emsdk_env.sh     # Emscripten 4 or later
#   examples/aerie/web/build.sh     # writes examples/aerie/web/out/
#   python3 -m http.server -d examples/aerie/web/out 8000
#
# Needs a browser with WebGPU and wasm64 (Memory64), e.g. a recent Chrome.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
aerie="$here/.."
lang="$aerie/../../lang"
kestrel="${KESTREL:-$aerie/../../target/release/kestrel}"
target=wasm64-unknown-emscripten
out="$here/out"
mkdir -p "$out"

# The SDL package's C helpers, built for the same target.
emcc --target=$target -sUSE_SDL=2 -O2 -c "$lang/sdl/sdl_helpers.c" -o "$out/sdl_helpers.o"

sources=("$aerie"/src/*.ks)
for dep in wgpu sdl linalg images deflate; do
    sources+=("$lang/$dep"/src/*.ks)
done

cd "$aerie"  # so --preload-file's `assets` path matches the desktop layout
"$kestrel" --target $target build -O 2 "${sources[@]}" -o "$out/index.html" \
    -l ":$out/sdl_helpers.o" \
    --link-arg=-sUSE_SDL=2 \
    --link-arg=--use-port=emdawnwebgpu \
    --link-arg=-sDEFAULT_TO_CXX \
    --link-arg=-sASYNCIFY \
    --link-arg=-sASYNCIFY_STACK_SIZE=262144 \
    --link-arg=-sALLOW_MEMORY_GROWTH \
    --link-arg=-sSTACK_SIZE=8388608 \
    --link-arg=-lidbfs.js \
    --link-arg=--preload-file=assets \
    --link-arg=--shell-file="$here/shell.html" \
    --link-arg=--post-js="$here/wasm64_fixes.js"
echo "built $out/index.html"
