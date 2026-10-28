// test: execution
// stdlib: true
// expect-exit: 0

// Passing table, accepted cell `escaping` -> normal. The coercion does NOT
// re-label the value (bit-copying a shared handle would skip the share
// operation); it produces a non-owning *view* of the shared environment rooted
// at the original handle. So the callee sees the same counter state and the
// original handle stays usable afterwards.
// See docs/design/closures.md — "Passing: What Fits Where".
module Test

import std.numeric.(Int64)

func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { count = count + 1; count }
}

func callNormal(f: () -> Int64) -> Int64 { f() }

@main
func main() -> lang.i64 {
    let e = makeCounter(0);

    if callNormal(e) != 1 { return 1 }
    // The view neither consumed nor forked the environment.
    if e() != 2 { return 2 }
    if callNormal(e) != 3 { return 3 }

    0
}
