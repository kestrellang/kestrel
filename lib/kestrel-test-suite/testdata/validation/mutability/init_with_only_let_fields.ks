// test: diagnostics
// stdlib: false

module Test
struct Immutable {
    let x: lang.i64
    let y: lang.i64

    init(x: lang.i64, y: lang.i64) {
        self.x = x;
        self.y = y;
    }
}
