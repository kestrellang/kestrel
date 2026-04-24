// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    var n: std.num.Int64 = 5;
    var result: std.num.Int64 = 1;
    while n > 1 {
        result = result * n;
        n = n - 1;
    }
    // 5! = 120
    if result != 120 { return 1 }
    0
}
