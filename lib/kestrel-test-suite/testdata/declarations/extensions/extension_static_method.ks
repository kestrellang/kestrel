// test: diagnostics
// stdlib: false
module Test
struct Point { var x: lang.i64; var y: lang.i64 }
extend Point {
    static func origin() -> Point { return Point(x: 0, y: 0); }
}
func test() -> Point { return Point.origin(); }
