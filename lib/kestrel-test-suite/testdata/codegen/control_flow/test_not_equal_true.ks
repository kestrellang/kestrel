// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: std.numeric.Int64 = 5;
    if x != 10 {
        0
    } else {
        1
    }
}
