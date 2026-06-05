// test: execution
// stdlib: true

module Test

func add_one(x: std.numeric.Int64) -> std.numeric.Int64 {
    x + 1
}

func apply(f: (std.numeric.Int64) -> std.numeric.Int64, x: std.numeric.Int64) -> std.numeric.Int64 {
    f(x)
}

@main
func main() -> lang.i64 {
    if apply(add_one, 41) != 42 { return 1 }
    0
}
