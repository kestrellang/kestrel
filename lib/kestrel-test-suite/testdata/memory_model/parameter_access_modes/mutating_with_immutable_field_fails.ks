// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
struct Container { let point: Point }
func reset(mutating p: Point) {
    p.x = 0;
}
func test() {
    var c = Container(point: Point(x: 1, y: 2));
    reset(c.point) // ERROR: mutating
}
