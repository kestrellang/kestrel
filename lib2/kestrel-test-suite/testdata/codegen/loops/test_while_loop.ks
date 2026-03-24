// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    var x: std.num.Int64 = 0;
    while x < 42 {
        x = x + 1;
    }
    if x != 42 { return 1 }
    0
}
