// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let t: (std.numeric.Int64, std.numeric.Int64, std.numeric.Int64) = (10, 20, 12);
    if t.0 + t.1 + t.2 != 42 { return 1 }
    0
}
