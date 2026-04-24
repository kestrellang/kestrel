// test: diagnostics
// stdlib: false

module Test

struct Point {
    var x: lang.i64
    var y: lang.i64
}
func test() -> lang.i64 {
    var p = Point(x: 1, y: 2);
    p.x = 10;
    p.x
}
