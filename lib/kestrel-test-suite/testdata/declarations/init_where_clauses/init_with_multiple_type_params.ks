// test: diagnostics
// stdlib: false

module Test

struct Pair {
    var first: lang.i64
    var second: lang.i64

    init[A, B](a: A, b: B) {
        self.first = 0;
        self.second = 0
    }
}
