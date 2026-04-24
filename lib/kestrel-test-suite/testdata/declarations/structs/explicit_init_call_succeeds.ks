// test: diagnostics
// stdlib: false
module Test
struct Point {
    var x: lang.i64
    var y: lang.i64

    init(x x: lang.i64, y y: lang.i64) {
        self.x = x;
        self.y = y;
    }
}

func test() -> Point {
    Point(x: 1, y: 2)
}
