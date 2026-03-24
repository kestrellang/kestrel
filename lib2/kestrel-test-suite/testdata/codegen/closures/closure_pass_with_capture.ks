// test: execution
// stdlib: true

module Test

func apply(f: (std.num.Int64) -> std.num.Int64, x: std.num.Int64) -> std.num.Int64 {
    f(x)
}

func main() -> lang.i64 {
    let multiplier: std.num.Int64 = 2;
    let result = apply({ (x) in x * multiplier }, 21);
    if result != 42 { return 1 }
    0
}
