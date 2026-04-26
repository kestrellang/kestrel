// test: execution
// stdlib: true

module Test

func double(x: std.num.Int64) -> std.num.Int64 {
    x * 2
}

func main() -> lang.i64 {
    let f = double;
    if f(21) != 42 { return 1 }
    0
}
