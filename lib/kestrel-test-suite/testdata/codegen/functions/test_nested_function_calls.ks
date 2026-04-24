// test: execution
// stdlib: true

module Test

func add(a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 {
    a + b
}

func main() -> lang.i64 {
    // add(10, 12) = 22, add(10, 10) = 20, add(22, 20) = 42
    if add(add(10, 12), add(10, 10)) != 42 { return 1 }
    0
}
