#!/bin/sh
# Build and run the deflate check programs against the in-repo compiler.
# Each prints one PASS/FAIL line per check; the script fails if any FAILs.
# When python3 is available, also checks that the reference zlib reads
# everything this package writes.
# Usage: lang/deflate/tests/run.sh [path/to/kestrel] [-O level, default 1]
set -e
here="$(cd "$(dirname "$0")" && pwd)"
pkg="$here/.."
repo="$pkg/../.."
kestrel="${1:-$repo/target/release/kestrel}"
opt="${2:-1}"
# A fresh private directory (mode 0700): the script runs the binaries it
# builds, so it must not reuse a predictable path someone else could plant.
out="$(mktemp -d "${TMPDIR:-/tmp}/kestrel-deflate-tests.XXXXXX")"
trap 'rm -rf "$out"' EXIT

failed=0
report() {
    pass=$(grep -c '^PASS' "$2" || true)
    fail=$(grep -c 'FAIL' "$2" || true)
    echo "$1: $pass passed, $fail failed"
    if [ "$fail" -ne 0 ]; then
        grep -A2 'FAIL' "$2"
        failed=1
    fi
}

"$kestrel" build -O "$opt" "$here/checksums.ks" $(find "$pkg/src" -name '*.ks') -o "$out/checksums"
"$out/checksums" > "$out/checksums.log"
report checksums "$out/checksums.log"

"$kestrel" build -O "$opt" "$here/decode.ks" "$here/inputs.ks" "$here/fixtures.ks" $(find "$pkg/src" -name '*.ks') -o "$out/decode"
"$out/decode" > "$out/decode.log"
report decode "$out/decode.log"

"$kestrel" build -O "$opt" "$here/encode.ks" "$here/inputs.ks" $(find "$pkg/src" -name '*.ks') -o "$out/encode"
"$out/encode" > "$out/encode.log"
report encode "$out/encode.log"

if command -v python3 >/dev/null 2>&1; then
    python3 "$here/check_with_python.py" < "$out/encode.log" > "$out/python.log" || true
    report "python interop" "$out/python.log"
fi
exit $failed
