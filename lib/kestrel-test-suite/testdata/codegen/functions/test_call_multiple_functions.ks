// test: execution
// stdlib: true

module Test

func mul(a: std.numeric.Int64, b: std.numeric.Int64) -> std.numeric.Int64 {
    a * b
}

func add(a: std.numeric.Int64, b: std.numeric.Int64) -> std.numeric.Int64 {
    a + b
}

@main
func main() -> lang.i64 {
    if add(mul(6, 7), 0) != 42 { return 1 }
    0
}
