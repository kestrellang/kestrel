// test: execution
// stdlib: true
// expect-exit: 0

// `xs.append(contentsOf: xs)` — the receiver passed `mutating` and the same
// array passed as the borrowed argument. The borrow takes no retain, so
// `makeUnique()` sees a unique buffer and `grow()` reallocates it, freeing the
// storage `other.asSlice()` still points into (array.ks `append(contentsOf:)`).
// Found in the 2026-10 architecture review at 9767d2dc: segfault (exit 139).
// The control with two distinct arrays runs clean.
//
// Expected semantics follow value semantics (Swift's behavior): the borrowed
// argument is the array's value at the call, so each round doubles it.
// EXPECTED TO FAIL until a `mutating` argument may not alias another argument
// of the same call — either rejected statically, or the overlapping borrow
// copied before the mutating access begins. If the fix is static rejection,
// this becomes a diagnostics test.

module Test

@main
func main() -> lang.i64 {
    var xs = [
        "first heap-allocated string, long enough to avoid inline storage",
        "second heap-allocated string, long enough to avoid inline storage",
    ];
    var i = 0;
    while i < 8 {
        xs.append(contentsOf: xs);
        i = i + 1;
    }
    if xs.count != 512 { return 1; }
    if xs(510) != xs(0) { return 2; }
    if xs(511) != xs(1) { return 3; }
    0
}
