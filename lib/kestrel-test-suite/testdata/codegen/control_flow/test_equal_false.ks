// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let x: std.numeric.Int64 = 5;
    if x == 10 {
        1
    } else {
        0
    }
}
