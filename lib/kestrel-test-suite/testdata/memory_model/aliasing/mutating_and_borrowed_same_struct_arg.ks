// test: execution
// stdlib: true
// expect-exit: 0

// The same variable passed as a `mutating` argument and a borrowed argument of
// one call. docs/memory-model/access-modes.md describes a borrowed parameter
// as read-only with the caller retaining ownership, but here `src` observes the
// write made through `dst`: `src.x` reads 2 on the second line, so `p.y`
// becomes 12. Found in the 2026-10 architecture review at 9767d2dc.
// references-gaps.md §10.4 records this aliasing as accepted behavior; this
// test pins the value-semantics reading of the docs so the decision is
// visible either way.
//
// Expected: `src` is `p`'s value at the call, so `p` becomes (2, 11).
// EXPECTED TO FAIL until a `mutating` argument may not alias another argument
// of the same call. If the decision is static rejection, this becomes a
// diagnostics test; if aliasing stays legal, delete it and say so in
// access-modes.md.

module Test

struct Point {
    var x: Int64;
    var y: Int64;
}

func addInto(mutating dst: Point, src: Point) {
    dst.x = dst.x + src.x;
    dst.y = dst.y + src.x;
}

@main
func main() -> lang.i64 {
    var p = Point(x: 1, y: 10);
    addInto(p, p);
    if p.x != 2 { return 1; }
    if p.y != 11 { return 2; }
    0
}
