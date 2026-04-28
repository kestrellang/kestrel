// test: execution
// stdlib: true

module Test

func apply_twice(f: (std.numeric.Int64) -> std.numeric.Int64, x: std.numeric.Int64) -> std.numeric.Int64 {
    f(f(x))
}

func main() -> lang.i64 {
    // apply_twice(add10, 22) = (22 + 10) + 10 = 42
    if apply_twice({ (x) in x + 10 }, 22) != 42 { return 1 }
    0
}
