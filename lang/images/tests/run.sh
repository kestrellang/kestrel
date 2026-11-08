#!/bin/sh
# Build and run the images check programs against the in-repo compiler.
# Each prints one PASS/FAIL line per check; the script fails if any FAILs.
# When python3 with Pillow is available, also checks that Pillow reads
# every file the package writes.
# Usage: lang/images/tests/run.sh [path/to/kestrel] [-O level, default 1]
set -e
here="$(cd "$(dirname "$0")" && pwd)"
pkg="$here/.."
repo="$pkg/../.."
kestrel="${1:-$repo/target/release/kestrel}"
opt="${2:-1}"
# A fresh private directory (mode 0700): the script runs the binaries it
# builds, so it must not reuse a predictable path someone else could plant.
out="$(mktemp -d "${TMPDIR:-/tmp}/kestrel-images-tests.XXXXXX")"
trap 'rm -rf "$out"' EXIT
sources="$(find "$pkg/src" "$pkg/../deflate/src" -name '*.ks')"

failed=0
report() {
    pass=$(grep -c '^PASS' "$2" || true)
    fail=$(grep -c 'FAIL' "$2" || true)
    echo "$1: $pass passed, $fail failed"
    if [ "$fail" -ne 0 ]; then
        grep 'FAIL' "$2"
        failed=1
    fi
}

"$kestrel" build -O "$opt" "$here/pngsuite.ks" "$here/common.ks" "$here/pngsuite_expected.ks" $sources -o "$out/pngsuite"
PNGSUITE="$here/pngsuite" "$out/pngsuite" > "$out/pngsuite.log"
report pngsuite "$out/pngsuite.log"

mkdir "$out/written"
"$kestrel" build -O "$opt" "$here/features.ks" "$here/common.ks" $sources -o "$out/features"
IMAGES_OUT="$out/written" "$out/features" > "$out/features.log"
report features "$out/features.log"

if command -v python3 >/dev/null 2>&1 && python3 -c 'import PIL, numpy' 2>/dev/null; then
    python3 -I "$here/check_with_python.py" "$out/written" > "$out/python.log" || true
    report "pillow interop" "$out/python.log"
fi
exit $failed
