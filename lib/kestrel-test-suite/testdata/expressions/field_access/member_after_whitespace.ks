// test: execution
// stdlib: true
// expect-exit: 0

// Trivia between `.` and a member name (or between a call and its `.`) is
// still a member access. The old body lowering read the member name as the
// token right after `.`, so `mk(). x` dropped `.x` and evaluated to `mk()`.
module Main

import std.numeric.Int64

struct S { var x: Int64 }

func mk() -> S { S(x: 3) }

@main
func main() -> Int64 {
    let a: Int64 = mk(). x;
    let b: Int64 = mk() .x;
    let c: Int64 = mk().
        x;
    if a != 3 { return 1; }
    if b != 3 { return 2; }
    if c != 3 { return 3; }
    return 0;
}
