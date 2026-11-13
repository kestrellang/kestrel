#!/bin/sh
# Build and run the regex checks against the in-repo compiler. Prints one
# PASS/FAIL line per check and fails if any FAIL.
# Usage: lang/regex/tests/run.sh [path/to/kestrel] [-O level, default 1]
set -e
here="$(cd "$(dirname "$0")" && pwd)"
pkg="$here/.."
repo="$pkg/../.."
kestrel="${1:-$repo/target/release/kestrel}"
opt="${2:-1}"
# A fresh private directory (mode 0700): the script runs the binary it
# builds, so it must not reuse a predictable path someone else could plant.
out="$(mktemp -d "${TMPDIR:-/tmp}/kestrel-regex-tests.XXXXXX")"
trap 'rm -rf "$out"' EXIT

"$kestrel" build -O "$opt" "$here/matching.ks" "$here/fixtures.ks" $(find "$pkg/src" -name '*.ks') -o "$out/matching"
"$out/matching" > "$out/matching.log"
pass=$(grep -c '^PASS' "$out/matching.log" || true)
fail=$(grep -c 'FAIL' "$out/matching.log" || true)
echo "matching: $pass passed, $fail failed"
if [ "$fail" -ne 0 ]; then
    grep -A2 'FAIL' "$out/matching.log"
    exit 1
fi
