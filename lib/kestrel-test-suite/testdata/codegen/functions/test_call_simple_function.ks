// test: execution
// stdlib: true

module Test

func add(a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 {
    a + b
}

func main() -> lang.i64 {
    if add(20, 22) != 42 { return 1 }
    0
}
