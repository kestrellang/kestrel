// test: diagnostics
// stdlib: false
module Test
struct Point {
    var x: lang.i64
    var y: lang.i64
}

func takePoint(p: Point) -> lang.i64 {
    42
}

func test() -> lang.i64 {
    takePoint(Point(x: 1, y: 2))
}
