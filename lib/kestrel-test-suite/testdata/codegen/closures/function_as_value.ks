// test: execution
// stdlib: true

module Test

func add_one(x: std.num.Int64) -> std.num.Int64 {
    x + 1
}

func apply(f: (std.num.Int64) -> std.num.Int64, x: std.num.Int64) -> std.num.Int64 {
    f(x)
}

func main() -> lang.i64 {
    if apply(add_one, 41) != 42 { return 1 }
    0
}
