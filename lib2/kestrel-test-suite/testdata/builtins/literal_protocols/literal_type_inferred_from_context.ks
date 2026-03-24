// test: diagnostics
// stdlib: false

module Test
struct Counter: Prelude.ExpressibleByIntegerLiteral {
    var count: lang.i64

    init(intLiteral value: lang.i64) {
        self.count = value
    }
}
func increment(c: Counter) -> Counter {
    Counter(intLiteral: lang.i64_add(c.count, 1))
}
func test() -> Counter {
    increment(0)
}
