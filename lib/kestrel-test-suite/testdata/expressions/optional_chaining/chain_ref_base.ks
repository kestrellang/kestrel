// test: execution
// stdlib: true
// expect-exit: 0

// A base that is already `Optional[&T]`: `chainBorrow` yields `& &T`, which
// the ref-of-ref peel collapses, so members resolve on `T`.

module Test

struct Point {
    var x: Int64
    func twice() -> Int64 { self.x * 2 }
}

@main
func main() -> lang.i64 {
    let point: Point? = .Some(Point(x: 6));
    let empty: Point? = .None;
    if point.asRef()?.x != .Some(6) { return 1 }
    if point.asRef()?.twice() != .Some(12) { return 2 }
    if empty.asRef()?.x != .None { return 3 }
    // a named `Optional[&T]`
    let borrowed = point.asRef();
    if borrowed?.x != .Some(6) { return 4 }
    0
}
