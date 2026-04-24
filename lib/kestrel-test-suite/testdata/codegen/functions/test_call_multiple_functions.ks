// test: execution
// stdlib: true

module Test

func mul(a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 {
    a * b
}

func add(a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 {
    a + b
}

func main() -> lang.i64 {
    if add(mul(6, 7), 0) != 42 { return 1 }
    0
}
