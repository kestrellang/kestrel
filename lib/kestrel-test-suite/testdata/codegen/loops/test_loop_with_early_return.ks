// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    var x: std.numeric.Int64 = 0;
    loop {
        x = x + 1;
        if x == 42 {
            return 0
        }
    }
}
