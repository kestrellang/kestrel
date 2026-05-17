// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    var x: std.numeric.Int64 = 0;
    loop {
        x = x + 1;
        if x == 10 {
            break
        }
        if x == 20 {
            break
        }
    }
    // First break at x == 10
    if x != 10 { return 1 }
    0
}
