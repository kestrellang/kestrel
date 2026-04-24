// test: diagnostics
// stdlib: false

module Test

struct Counter {
    var count: lang.i64

    func getValue() -> lang.i64 { self.count }
    static func zero() -> Counter { Counter(count: 0) }
}
