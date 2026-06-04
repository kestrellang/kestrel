// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let opt: std.result.Optional[std.numeric.Int64] = .Some(42);
    match opt {
        some _ => 0,
        .None => 1
    }
}
