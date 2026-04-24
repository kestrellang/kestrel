// test: diagnostics
// stdlib: false

module Test
struct Point {
    var x: lang.i64
    var y: lang.i64
}
func test() -> lang.i64 {
    Point(x: 1, y: 2) = Point(x: 3, y: 4); // ERROR: cannot assign to this expression
    0
}
