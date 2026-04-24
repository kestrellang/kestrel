// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
func process(a: Point, mutating b: Point, consuming c: Point) -> lang.i64 {
    b.x = a.x;
    c.x
}
func caller() -> lang.i64 {
    let pt1 = Point(x: 1, y: 2);
    var pt2 = Point(x: 3, y: 4);
    let pt3 = Point(x: 5, y: 6);
    process(pt1, pt2, pt3)
}
