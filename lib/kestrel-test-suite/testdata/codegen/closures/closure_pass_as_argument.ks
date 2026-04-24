// test: execution
// stdlib: true

module Test

func apply(f: (std.num.Int64) -> std.num.Int64, x: std.num.Int64) -> std.num.Int64 {
    f(x)
}

func main() -> lang.i64 {
    let result = apply({ (x) in x * 2 }, 21);
    if result != 42 { return 1 }
    0
}
