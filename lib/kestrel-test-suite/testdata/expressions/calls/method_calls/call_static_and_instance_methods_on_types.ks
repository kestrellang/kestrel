// test: diagnostics
// stdlib: false

module Main

struct Counter {
    let value: lang.i64
    static func zero() -> lang.i64 { 0 }
    static func max(a: lang.i64, b: lang.i64) -> lang.i64 { 42 }
    func getValue() -> lang.i64 { self.value }
    func increment() -> lang.i64 { 42 }
}

func test(c: Counter) -> lang.i64 {
    c.increment()
}
