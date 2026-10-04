// test: execution
// stdlib: true
// expect-exit: 0

// The same array passed as a `mutating` argument and a borrowed argument of one
// call. The borrow takes no retain, so appending through `dst` reallocates the
// storage that the loop over `src` is still reading. Found in the 2026-10
// architecture review at 9767d2dc: segfault (exit 139). The control
// (`var ys = xs; dup(ys, xs)`) runs clean.
//
// Expected semantics (value semantics, as Swift evaluates a by-value argument
// before the inout access begins): `src` is the array's value at the call,
// so the loop visits the two original elements.
// EXPECTED TO FAIL until a `mutating` argument may not alias another argument
// of the same call — either rejected statically, or the overlapping borrow
// copied first. If the fix is static rejection, this becomes a diagnostics test.

module Test

func dup(mutating dst: Array[String], src: Array[String]) -> Int64 {
    var n = 0;
    for s in src {
        dst.append("copy of \(s) with enough padding to force a heap allocation");
        n = n + 1;
        if n > 50 { break; }
    }
    n
}

@main
func main() -> lang.i64 {
    var xs = [
        "first heap-allocated string, long enough to avoid inline storage",
        "second heap-allocated string, long enough to avoid inline storage",
    ];
    let visited = dup(xs, xs);
    if visited != 2 { return 1; }
    if xs.count != 4 { return 2; }
    0
}
