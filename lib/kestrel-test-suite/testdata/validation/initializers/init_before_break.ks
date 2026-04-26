// test: diagnostics
// stdlib: false

module Main

struct Value {
    var n: lang.i64

    init() {
        loop {
            self.n = 42;
            break;
        }
    }
}
