// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: std.numeric.Int64 = 142;
    let y: std.numeric.Int64 = 100;
    if x % y != 42 { return 1 }
    0
}
