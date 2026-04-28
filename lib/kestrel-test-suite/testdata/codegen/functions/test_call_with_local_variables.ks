// test: execution
// stdlib: true

module Test

func square(x: std.numeric.Int64) -> std.numeric.Int64 {
    x * x
}

func main() -> lang.i64 {
    let a: std.numeric.Int64 = 6;
    let b = square(a);
    // square(6) = 36, 36 + 6 = 42
    if b + 6 != 42 { return 1 }
    0
}
