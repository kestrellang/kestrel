// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
func reset(mutating p: Point) {
    p.x = 0;
}
func caller() {
    var pt = Point(x: 1, y: 2);
    reset(pt)
}
