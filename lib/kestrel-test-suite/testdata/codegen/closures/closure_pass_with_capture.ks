// test: execution
// stdlib: true

module Test

func apply(f: (std.numeric.Int64) -> std.numeric.Int64, x: std.numeric.Int64) -> std.numeric.Int64 {
    f(x)
}

@main
func main() -> lang.i64 {
    let multiplier: std.numeric.Int64 = 2;
    let result = apply({ (x) in x * multiplier }, 21);
    if result != 42 { return 1 }
    0
}
