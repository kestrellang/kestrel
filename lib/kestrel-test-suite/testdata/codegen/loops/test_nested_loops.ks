// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    var sum: std.numeric.Int64 = 0;
    var i: std.numeric.Int64 = 0;
    while i < 6 {
        var j: std.numeric.Int64 = 0;
        while j < 7 {
            sum = sum + 1;
            j = j + 1;
        }
        i = i + 1;
    }
    // 6 * 7 = 42
    if sum != 42 { return 1 }
    0
}
