// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    var x: std.numeric.Int64 = 0;
    while x < 100 {
        x = x + 1;
        if x == 42 {
            return 0
        }
    }
    1
}
