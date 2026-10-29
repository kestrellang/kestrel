#!/bin/sh
# Build and run the datetime regression program against the in-repo compiler.
# Usage: lang/datetime/tests/run.sh [path/to/kestrel]
set -e
here="$(cd "$(dirname "$0")" && pwd)"
pkg="$here/.."
repo="$pkg/../.."
kestrel="${1:-$repo/target/release/kestrel}"
out="${TMPDIR:-/tmp}/kestrel-datetime-tests"
mkdir -p "$out"

cc -c "$pkg/datetime_shims.c" -o "$out/shims.o"
ar rcs "$out/libdatetime_shims.a" "$out/shims.o"
"$kestrel" build "$here/regressions.ks" "$pkg"/src/*.ks \
    -L "$out" -l datetime_shims -o "$out/regressions"
"$out/regressions"
