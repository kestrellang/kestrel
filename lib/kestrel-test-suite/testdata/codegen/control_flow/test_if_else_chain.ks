// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let x: std.numeric.Int64 = 10;
    if x < 5 {
        1
    } else if x < 15 {
        0
    } else {
        1
    }
}
