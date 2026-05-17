// test: diagnostics
// stdlib: false

module Test

struct Point {
    var x: lang.i64
    var y: lang.i64

    init(x: lang.i64, y: lang.i64) {
        self.x = x;
        self.y = y
    }

    init(x: lang.i64) {
        self.init(x, 0)
    }
}
