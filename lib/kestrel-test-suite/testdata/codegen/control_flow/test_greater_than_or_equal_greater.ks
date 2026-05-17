// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: std.numeric.Int64 = 10;
    if x >= 5 {
        0
    } else {
        1
    }
}
