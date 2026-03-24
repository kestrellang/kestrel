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

func makePoint() -> Point {
    Point(x: 5, y: 10)
}
