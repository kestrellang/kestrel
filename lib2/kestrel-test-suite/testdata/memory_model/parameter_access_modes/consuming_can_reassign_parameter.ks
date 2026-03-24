// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
func replace(consuming p: Point) -> Point {
    p = Point(x: 0, y: 0);
    p
}
