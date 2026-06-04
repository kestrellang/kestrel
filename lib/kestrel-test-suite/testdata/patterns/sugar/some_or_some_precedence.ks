// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let opt: std.result.Optional[std.numeric.Int64] = .Some(2);
    match opt {
        some 1 or some 2 => 0,
        some _ => 1,
        null => 2
    }
}
