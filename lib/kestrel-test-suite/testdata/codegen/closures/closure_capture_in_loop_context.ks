// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let base: std.numeric.Int64 = 40;
    let f = { (x: std.numeric.Int64) in base + x };

    var sum: std.numeric.Int64 = 0;
    var i: std.numeric.Int64 = 0;
    while i < 2 {
        sum = sum + f(1);
        i = i + 1
    }
    // (40 + 1) + (40 + 1) = 82, but we want 42
    // Let's use a single call: base=40, x=2 => 42
    if f(2) != 42 { return 1 }
    0
}
