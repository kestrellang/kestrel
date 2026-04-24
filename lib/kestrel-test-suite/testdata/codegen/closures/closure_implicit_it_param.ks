// test: execution
// stdlib: true

module Test

func apply(f: (std.num.Int64) -> std.num.Int64, x: std.num.Int64) -> std.num.Int64 {
    f(x)
}

func main() -> lang.i64 {
    if apply({ it * 2 }, 21) != 42 { return 1 }
    0
}
