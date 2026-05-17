// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let a: std.numeric.Int64 = 5;
    let b: std.numeric.Int64 = 10;
    if (a < b) and (b > 5) {
        0
    } else {
        1
    }
}
