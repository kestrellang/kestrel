// test: execution
// stdlib: true

module Test

func apply(f: (std.numeric.Int64) -> std.numeric.Int64, x: std.numeric.Int64) -> std.numeric.Int64 {
    f(x)
}

@main
func main() -> lang.i64 {
    if apply({ it * 2 }, 21) != 42 { return 1 }
    0
}
