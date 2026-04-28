// test: execution
// stdlib: true

module Test

func apply(f: (std.numeric.Int64) -> std.numeric.Int64, x: std.numeric.Int64) -> std.numeric.Int64 {
    f(x)
}

func main() -> lang.i64 {
    let offset: std.numeric.Int64 = 20;
    if apply({ it + offset }, 22) != 42 { return 1 }
    0
}
