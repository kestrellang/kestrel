// test: execution
// stdlib: true

module Test

func add(a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 {
    a + b
}

func apply_binary(f: (std.num.Int64, std.num.Int64) -> std.num.Int64, x: std.num.Int64, y: std.num.Int64) -> std.num.Int64 {
    f(x, y)
}

func main() -> lang.i64 {
    if apply_binary(add, 20, 22) != 42 { return 1 }
    0
}
