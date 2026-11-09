#!/bin/sh
# Builds the sojourn-js command with the in-repo compiler.
# Usage: lang/sojourn-js/build.sh [path/to/kestrel] [-O level, default 2]
set -e
here="$(cd "$(dirname "$0")" && pwd)"
kestrel="${1:-$here/../../target/release/kestrel}"
opt="${2:-2}"
mkdir -p "$here/build"
cd "$here/src"
"$kestrel" build -O "$opt" $(ls *.ks) -o "$here/build/sojourn-js"
