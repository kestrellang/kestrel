// test: diagnostics
// stdlib: false

module Test

struct Counter {
    var count: lang.i64

    @deprecated(replacement: "make")
    init() {
        self.count = 0;
    }
}

func use() -> Counter {
    Counter() // WARN: 'Counter.init' is deprecated; use 'make' instead
}
