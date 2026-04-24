// test: diagnostics
// stdlib: false

module Test

struct Counter {
    var count: lang.i64

    init() {
        self.count = 0
    }

    init(startingAt value: lang.i64) {
        self.init();
        self.count = value
    }
}
