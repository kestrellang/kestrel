// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    var x: std.numeric.Int64 = 50;
    while x > 42 {
        x = x - 1;
    }
    if x != 42 { return 1 }
    0
}
