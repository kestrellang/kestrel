// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let x: std.numeric.Int64 = 50;
    let y: std.numeric.Int64 = 8;
    if x - y != 42 { return 1 }
    0
}
