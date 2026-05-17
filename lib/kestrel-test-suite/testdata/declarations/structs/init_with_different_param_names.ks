// test: diagnostics
// stdlib: false
module Test
struct Point {
    var x: lang.i64
    var y: lang.i64

    init(xVal: lang.i64, yVal: lang.i64) {
        self.x = xVal;
        self.y = yVal;
    }
}

func test() -> Point {
    Point(1, 2)
}
