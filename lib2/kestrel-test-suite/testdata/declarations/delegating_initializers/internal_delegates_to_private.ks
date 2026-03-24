// test: diagnostics
// stdlib: false

module Test

struct Internal {
    var value: lang.i64

    private init(value: lang.i64) {
        self.value = value
    }

    internal init(fromInt n: lang.i64) {
        self.init(n)
    }
}
