// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md, Behavior Changes #4: "Returning a capturing closure
// becomes possible — with `escaping` ... or `consuming` ... in the return
// type." A `consuming` closure owns everything it holds, so its provenance is
// self-rooted and it leaves the frame instead of tripping E494.
module Test

import std.numeric.Int64

func makeThunk(n: Int64) -> consuming () -> Int64 {
    { n + 1 }
}

@main
func main() -> lang.i64 {
    let t = makeThunk(41);
    if t() != 42 { return 1; }
    0
}
