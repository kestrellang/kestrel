// test: execution
// stdlib: true
// expect-exit: 0

// A call with a `_` on the right fills the slot directly: no closure.

module Test

func sub(a: Int64, b: Int64) -> Int64 { a - b }
func mid(a: Int64, b: Int64, c: Int64) -> Int64 { a * 100 + b * 10 + c }
func clamp(x: Int64, min lo: Int64, max hi: Int64) -> Int64 {
    if x < lo { return lo }
    if x > hi { return hi }
    x
}

@main
func main() -> lang.i64 {
    // first
    if (10 |> sub(_, 3)) != 7 { return 1 }
    // last
    if (10 |> sub(3, _)) != -7 { return 2 }
    // middle
    if (5 |> mid(1, _, 9)) != 159 { return 3 }
    // labeled
    if (300 |> clamp(_, min: 0, max: 255)) != 255 { return 4 }
    if (7 |> clamp(1, min: _, max: 255)) != 7 { return 5 }
    // chained slots
    if (10 |> sub(_, 3) |> sub(100, _)) != 93 { return 6 }
    0
}
