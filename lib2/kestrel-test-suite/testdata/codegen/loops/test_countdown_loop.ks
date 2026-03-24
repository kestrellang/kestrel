// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    var countdown: std.num.Int64 = 10;
    var result: std.num.Int64 = 0;
    while countdown > 0 {
        result = result + countdown;
        countdown = countdown - 1;
    }
    // 10+9+8+7+6+5+4+3+2+1 = 55
    if result != 55 { return 1 }
    0
}
