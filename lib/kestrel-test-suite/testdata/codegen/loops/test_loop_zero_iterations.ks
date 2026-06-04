// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    var x: std.numeric.Int64 = 42;
    while false {
        x = 0;
    }
    if x != 42 { return 1 }
    0
}
