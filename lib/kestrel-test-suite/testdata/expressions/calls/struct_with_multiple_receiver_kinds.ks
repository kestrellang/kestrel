// test: diagnostics
// stdlib: false

module Main

struct Counter {
    let value: lang.i64
    var mutableValue: lang.i64

    func getValue() -> lang.i64 {
        42
    }

    mutating func increment() -> () {
        ()
    }

    consuming func consume() -> lang.i64 {
        self.value
    }
}
