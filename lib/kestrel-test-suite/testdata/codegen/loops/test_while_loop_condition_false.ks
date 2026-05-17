// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    var x: std.numeric.Int64 = 100;
    while x < 42 {
        x = x + 1;
    }
    // Loop body never executes, x stays 100
    if x != 100 { return 1 }
    0
}
