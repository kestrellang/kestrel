// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    var sum: std.num.Int64 = 0;
    var i: std.num.Int64 = 0;
    while i < 10 {
        var j: std.num.Int64 = 0;
        while j < 10 {
            sum = sum + 1;
            if sum == 42 {
                break
            }
            j = j + 1;
        }
        if sum == 42 {
            break
        }
        i = i + 1;
    }
    if sum != 42 { return 1 }
    0
}
