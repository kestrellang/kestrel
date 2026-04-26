// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() {
    var p: Point = Point(x: 0, y: 0);
    p.x = "not an lang.i64"; // ERROR
}
