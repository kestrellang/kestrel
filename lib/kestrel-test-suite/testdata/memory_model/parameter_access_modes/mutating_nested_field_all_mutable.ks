// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
struct Inner { var point: Point }
struct Outer { var inner: Inner }
func reset(mutating p: Point) {
    p.x = 0;
}
func test() {
    var o = Outer(inner: Inner(point: Point(x: 1, y: 2)));
    reset(o.inner.point)
}
