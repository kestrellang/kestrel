#!/bin/sh
# Build and run the crypto check programs against the in-repo compiler.
# Each prints one PASS/FAIL line per check; the script fails if any FAILs.
# Usage: lang/crypto/tests/run.sh [path/to/kestrel] [-O level, default 1]
set -e
here="$(cd "$(dirname "$0")" && pwd)"
pkg="$here/.."
repo="$pkg/../.."
kestrel="${1:-$repo/target/release/kestrel}"
opt="${2:-1}"
# A fresh private directory (mode 0700): the script runs the binaries it
# builds, so it must not reuse a predictable path someone else could plant.
out="$(mktemp -d "${TMPDIR:-/tmp}/kestrel-crypto-tests.XXXXXX")"
trap 'rm -rf "$out"' EXIT

failed=0
for test in core digests passwords; do
    "$kestrel" build -O "$opt" "$here/$test.ks" $(find "$pkg/src" -name '*.ks') -o "$out/$test"
    "$out/$test" > "$out/$test.log"
    pass=$(grep -c '^PASS' "$out/$test.log" || true)
    fail=$(grep -c 'FAIL' "$out/$test.log" || true)
    echo "$test: $pass passed, $fail failed"
    if [ "$fail" -ne 0 ]; then
        grep -A2 'FAIL' "$out/$test.log"
        failed=1
    fi
done
exit $failed
