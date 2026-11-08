// test: execution
// stdlib: true
// expect-exit: 0

// `Result` conforms to `OptionalChain`: `?.` chains on the `Ok` value and
// gives an `Optional`, dropping the error.

module Test

struct Point {
    var x: Int64
}

@main
func main() -> lang.i64 {
    let ok: Result[Point, Int64] = .Ok(Point(x: 3));
    let err: Result[Point, Int64] = .Err(5);
    if ok?.x != .Some(3) { return 1 }
    if err?.x != .None { return 2 }
    0
}
