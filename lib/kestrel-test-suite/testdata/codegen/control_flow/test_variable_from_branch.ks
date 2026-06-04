// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let x: std.numeric.Int64 = 10;
    let y: std.numeric.Int64 = if x > 5 { 40 } else { 0 };
    if y + 2 != 42 { return 1 }
    0
}
