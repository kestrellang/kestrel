// test: execution
// stdlib: true

module Test

func apply(f: (std.num.Int64) -> std.num.Int64, x: std.num.Int64) -> std.num.Int64 {
    f(x)
}

func main() -> lang.i64 {
    let offset: std.num.Int64 = 20;
    if apply({ it + offset }, 22) != 42 { return 1 }
    0
}
