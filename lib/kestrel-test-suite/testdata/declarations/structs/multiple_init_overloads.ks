// test: diagnostics
// stdlib: false
module Test
struct Point {
    var x: lang.i64
    var y: lang.i64

    init(x: lang.i64, y: lang.i64) {
        self.x = x;
        self.y = y;
    }

    init() {
        self.x = 0;
        self.y = 0;
    }

    init(value: lang.i64) {
        self.x = value;
        self.y = value;
    }
}

func test() {
    let p1 = Point(1, 2);
    let p2 = Point();
    let p3 = Point(5);
}
