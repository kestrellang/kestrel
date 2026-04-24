// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
struct Container { var point: Point }
func reset(mutating p: Point) {
    p.x = 0;
}
func test() {
    let c = Container(point: Point(x: 1, y: 2));
    reset(c.point) // ERROR: mutating
}
