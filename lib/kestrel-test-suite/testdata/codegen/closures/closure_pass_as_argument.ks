// test: execution
// stdlib: true

module Test

func apply(f: (std.numeric.Int64) -> std.numeric.Int64, x: std.numeric.Int64) -> std.numeric.Int64 {
    f(x)
}

func main() -> lang.i64 {
    let result = apply({ (x) in x * 2 }, 21);
    if result != 42 { return 1 }
    0
}
