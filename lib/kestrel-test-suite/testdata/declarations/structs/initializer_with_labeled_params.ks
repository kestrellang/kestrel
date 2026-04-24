// test: diagnostics
// stdlib: false

module Test

struct Point {
    var x: lang.i64
    var y: lang.i64

    init(atX x: lang.i64, atY y: lang.i64) {
        self.x = x;
        self.y = y;
    }
}
