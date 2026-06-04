// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let none: std.result.Optional[std.numeric.Int64] = .None;
    match none {
        null => 0,
        some _ => 1
    }
}
