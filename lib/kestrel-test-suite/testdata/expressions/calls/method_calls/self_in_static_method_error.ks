// test: diagnostics
// stdlib: false

module Main

struct Calculator {
    let value: lang.i64

    static func compute() -> lang.i64 {
        self.value // ERROR: cannot use 'self' in static method
    }
}
