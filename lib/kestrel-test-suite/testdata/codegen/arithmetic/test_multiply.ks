// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: std.numeric.Int64 = 6;
    let y: std.numeric.Int64 = 7;
    if x * y != 42 { return 1 }
    0
}
