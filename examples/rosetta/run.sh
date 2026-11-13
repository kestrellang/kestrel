#!/usr/bin/env bash
# Builds every Rosetta Code task (or the ones named) and checks its output
# against the matching .out file. Tasks that read input get <name>.in on stdin.
#
#   ./run.sh                 # all tasks
#   ./run.sh fizzbuzz towers_of_hanoi
#   BLESS=1 ./run.sh name    # write the current output to name.out
#   KESTREL=path OPT=2 JOBS=8 ./run.sh
set -u
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
kestrel="${KESTREL:-$root/target/release/kestrel}"
out="${OUT:-${TMPDIR:-/tmp}/kestrel-rosetta}"
opt="${OPT:-0}"
jobs="${JOBS:-$(nproc 2>/dev/null || echo 4)}"
mkdir -p "$out"

if [ $# -eq 0 ]; then
    set -- $(cd "$here" && ls *.ks | sed 's/\.ks$//')
fi

check() {
    local name="$1" src="$here/$1.ks" exe="$out/$1"
    local input=/dev/null
    [ -f "$here/$name.in" ] && input="$here/$name.in"
    if ! { "$kestrel" --std "$root/lang/std" build -O "$opt" "$src" -o "$exe" 2>&1 | sed 's/\x1b\[[0-9;]*m//g' >"$out/$name.build.log"; [ "${PIPESTATUS[0]}" -eq 0 ]; }; then
        echo "BUILD FAIL  $name"; return
    fi
    (cd "$out" && timeout 60 "$exe" <"$input" >"$out/$name.actual" 2>"$out/$name.stderr")
    local code=$?
    if [ -n "${BLESS:-}" ]; then
        cp "$out/$name.actual" "$here/$name.out"; echo "BLESSED     $name"; return
    fi
    if [ ! -f "$here/$name.out" ]; then
        echo "NO .out     $name (exit $code)"
    elif cmp -s "$out/$name.actual" "$here/$name.out"; then
        echo "ok          $name"
    else
        echo "WRONG       $name (exit $code)"
    fi
}
export -f check
export here root kestrel out opt

printf '%s\n' "$@" | xargs -P "$jobs" -I{} bash -c 'check {}' | sort -k2 | tee "$out/summary.txt"
echo "$(grep -c '^ok' "$out/summary.txt") / $# ok"
