// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let t: ((std.numeric.Int64, std.numeric.Int64), std.numeric.Int64) = ((40, 2), 0);
    let inner = t.0;
    if inner.0 + inner.1 != 42 { return 1 }
    0
}
