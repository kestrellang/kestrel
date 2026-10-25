// test: execution
// stdlib: true
// expect-exit: 6

// `asRef()` on an `Optional[&T]` is generic `asRef` at `T = &Point`, so its
// result is `Optional[& &Point]`, which typing peels to one level. The value
// read through it must be the point's field. Today it reads garbage: the
// generic body's `&value` makes a real pointer-to-pointer.

module Test
struct Point {
    var x: Int64
}
@main
func main() -> Int64 {
    let point: Point? = .Some(Point(x: 6));
    let borrowed = point.asRef();
    let o = borrowed.asRef();
    match o {
        .Some(r) => r.x,
        .None => 100
    }
}
