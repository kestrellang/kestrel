// test: execution
// stdlib: true

module Test

func double(x: std.num.Int64) -> std.num.Int64 {
    x * 2
}

func add_ten(x: std.num.Int64) -> std.num.Int64 {
    x + 10
}

func main() -> lang.i64 {
    // double(16) = 32, add_ten(32) = 42
    if add_ten(double(16)) != 42 { return 1 }
    0
}
