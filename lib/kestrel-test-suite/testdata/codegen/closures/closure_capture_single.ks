// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let captured: std.numeric.Int64 = 32;
    let f = { (x: std.numeric.Int64) in x + captured };
    if f(10) != 42 { return 1 }
    0
}
