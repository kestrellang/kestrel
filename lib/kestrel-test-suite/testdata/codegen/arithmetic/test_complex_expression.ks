// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let a: std.num.Int64 = 10;
    let b: std.num.Int64 = 3;
    let c: std.num.Int64 = 2;
    // (10 + 3) * 2 + (10 - 3) = 13 * 2 + 7 = 26 + 7 = 33
    if (a + b) * c + (a - b) != 33 { return 1 }
    0
}
