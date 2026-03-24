// test: diagnostics
// stdlib: false

module Test
struct Counter {
    var count: lang.i64

    @dummy
    mutating func increment() {
        self.count = lang.i64_add(self.count, 1);
    }
}
