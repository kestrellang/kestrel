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

    func sum() -> lang.i64 {
        self.x
    }

    func product() -> lang.i64 {
        self.y
    }
}
