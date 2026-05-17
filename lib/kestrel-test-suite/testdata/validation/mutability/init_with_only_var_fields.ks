// test: diagnostics
// stdlib: false

module Test
struct Mutable {
    var x: lang.i64
    var y: lang.i64

    init(x: lang.i64, y: lang.i64) {
        self.x = x;
        self.y = y;
    }
}
