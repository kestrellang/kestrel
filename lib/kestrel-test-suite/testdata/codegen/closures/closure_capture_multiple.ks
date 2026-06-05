// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let a: std.numeric.Int64 = 10;
    let b: std.numeric.Int64 = 20;
    let c: std.numeric.Int64 = 12;
    let f = { a + b + c };
    if f() != 42 { return 1 }
    0
}
