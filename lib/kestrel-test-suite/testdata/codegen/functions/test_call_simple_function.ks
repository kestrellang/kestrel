// test: execution
// stdlib: true

module Test

func add(a: std.numeric.Int64, b: std.numeric.Int64) -> std.numeric.Int64 {
    a + b
}

func main() -> lang.i64 {
    if add(20, 22) != 42 { return 1 }
    0
}
