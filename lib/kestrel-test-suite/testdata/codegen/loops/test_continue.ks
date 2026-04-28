// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    var sum: std.numeric.Int64 = 0;
    var i: std.numeric.Int64 = 0;
    while i < 10 {
        i = i + 1;
        if i == 5 {
            continue
        }
        sum = sum + i;
    }
    // 1+2+3+4+6+7+8+9+10 = 55-5 = 50
    if sum != 50 { return 1 }
    0
}
