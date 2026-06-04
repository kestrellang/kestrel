// test: execution
// stdlib: true

module Test

func combine(
    f: (std.numeric.Int64) -> std.numeric.Int64,
    g: (std.numeric.Int64) -> std.numeric.Int64,
    x: std.numeric.Int64
) -> std.numeric.Int64 {
    g(f(x))
}

@main
func main() -> lang.i64 {
    let result = combine(
        { (x) in x + 10 },
        { (x) in x * 2 },
        11
    );
    // (11 + 10) * 2 = 42
    if result != 42 { return 1 }
    0
}
