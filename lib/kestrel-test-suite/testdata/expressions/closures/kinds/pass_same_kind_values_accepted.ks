// test: execution
// stdlib: true
// expect-exit: 0

// Passing table diagonal: every kind passes as itself. A pre-existing
// `mutating` closure value must be held in a `var` (its calls are exclusive);
// a `consuming` value is `not Copyable`, so handing it on is a move and the
// callee's single call consumes it; an `escaping` value passes as `escaping`
// by sharing its handle.
// See docs/design/closures.md — "Passing: What Fits Where".
module Test

import std.numeric.(Int64)

func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { count = count + 1; count }
}

func bumpTwice(mutating f: mutating () -> ()) { f(); f(); }

func runOnce(consuming f: consuming () -> Int64) -> Int64 { f() }

func callEscaping(f: escaping () -> Int64) -> Int64 { f() }

@main
func main() -> lang.i64 {
    // mutating -> mutating, reused across repeated calls from a `var`.
    var total = 0;
    var bump: mutating () -> () = { total = total + 1; };
    bumpTwice(bump);
    bumpTwice(bump);
    if total != 4 { return 1 }

    // consuming -> consuming: moved in, called exactly once.
    let seed = 6;
    let once: consuming () -> Int64 = { seed };
    if runOnce(once) != 6 { return 2 }

    // escaping -> escaping: the handle is shared, both sides drive one counter.
    let e = makeCounter(0);
    if callEscaping(e) != 1 { return 3 }
    if e() != 2 { return 4 }

    0
}
