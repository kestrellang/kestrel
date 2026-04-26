// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64

    init(x: lang.i64 = 0, y: lang.i64 = 0) {
        self.x = x;
        self.y = y;
    }
}

func test() -> Point {
    Point()
}
