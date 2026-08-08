// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md: a `consuming` closure is `not Copyable` — "handing
// it around is a move". Forwarding it through an intermediate `consuming`
// parameter must transfer the one environment intact and still call exactly
// once at the far end.
module Test

import std.numeric.Int64

func callIt(consuming f: consuming () -> Int64) -> Int64 { f() }

func forward(consuming f: consuming () -> Int64) -> Int64 { callIt(f) }

@main
func main() -> lang.i64 {
    let base: Int64 = 5;
    let g: consuming () -> Int64 = { base * 3 };
    if forward(g) != 15 { return 1; }
    0
}
