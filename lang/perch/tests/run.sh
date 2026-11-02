#!/bin/sh
# Build and run the perch check programs against the in-repo compiler.
# Each prints one PASS/FAIL line per check; the script fails if any FAILs.
# Usage: lang/perch/tests/run.sh [path/to/kestrel] [test names...]
set -e
here="$(cd "$(dirname "$0")" && pwd)"
pkg="$here/.."
repo="$pkg/../.."
kestrel="${1:-$repo/target/release/kestrel}"
[ $# -gt 0 ] && shift
tests="${*:-checks}"
# A fresh private directory (mode 0700): the script runs the binaries it
# builds, so it must not reuse a predictable path someone else could plant.
out="$(mktemp -d "${TMPDIR:-/tmp}/kestrel-perch-tests.XXXXXX")"
trap 'rm -rf "$out"' EXIT

sources=$(find "$pkg/src" "$pkg/../http/src" "$pkg/../quill/src" "$pkg/../quill-json/src" -name '*.ks')
failed=0
for test in $tests; do
    "$kestrel" build "$here/$test.ks" $sources -o "$out/$test"
    "$out/$test" > "$out/$test.log"
    pass=$(grep -c '^PASS' "$out/$test.log" || true)
    fail=$(grep -c 'FAIL' "$out/$test.log" || true)
    echo "$test: $pass passed, $fail failed"
    if [ "$fail" -ne 0 ]; then
        grep 'FAIL' "$out/$test.log"
        failed=1
    fi
done
exit $failed
