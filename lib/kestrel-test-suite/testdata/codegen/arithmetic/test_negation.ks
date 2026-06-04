// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let x: std.numeric.Int64 = -42;
    if -x != 42 { return 1 }
    0
}
