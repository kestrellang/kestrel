// test: diagnostics
// stdlib: false

module Main

struct Value {
    var n: lang.i64

    init(x: lang.i64) {
        if lang.i64_eq(x, 1) {
            self.n = 10;
        } else if lang.i64_eq(x, 2) {
            self.n = 20;
        } else {
            self.n = 0;
        }
    }
}
