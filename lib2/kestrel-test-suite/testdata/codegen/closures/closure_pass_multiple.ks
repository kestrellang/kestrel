// test: execution
// stdlib: true

module Test

func combine(
    f: (std.num.Int64) -> std.num.Int64,
    g: (std.num.Int64) -> std.num.Int64,
    x: std.num.Int64
) -> std.num.Int64 {
    g(f(x))
}

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
