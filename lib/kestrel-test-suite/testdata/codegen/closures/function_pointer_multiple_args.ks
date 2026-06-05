// test: execution
// stdlib: true

module Test

func add(a: std.numeric.Int64, b: std.numeric.Int64) -> std.numeric.Int64 {
    a + b
}

func apply_binary(f: (std.numeric.Int64, std.numeric.Int64) -> std.numeric.Int64, x: std.numeric.Int64, y: std.numeric.Int64) -> std.numeric.Int64 {
    f(x, y)
}

@main
func main() -> lang.i64 {
    if apply_binary(add, 20, 22) != 42 { return 1 }
    0
}
