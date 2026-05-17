// test: diagnostics
// stdlib: false

module Test

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func consume(consuming p: Point) {}

func test() {
    var pt = Point(x: 1, y: 2);
    consume(pt);
    consume(pt)
}
