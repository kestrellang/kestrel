// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let inner: std.result.Optional[std.numeric.Int64] = .Some(42);
    let outer: std.result.Optional[std.result.Optional[std.numeric.Int64]] = .Some(inner);

    match outer {
        some some _ => 0,
        some null => 1,
        null => 2
    }
}
