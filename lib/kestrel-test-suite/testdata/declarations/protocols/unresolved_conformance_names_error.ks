// test: diagnostics
// stdlib: false

// A conformance naming a protocol or protocol arg that doesn't exist is an
// error. Both used to be dropped silently.

module Test

protocol Holds[T] {
    func hold(value: lang.i64) -> lang.i64
}

struct Slot {}

extend Slot: Nope {} // ERROR: cannot find type 'Nope'

extend Slot: Holds[Zork] { // ERROR: cannot find type 'Zork'
    func hold(value: lang.i64) -> lang.i64 { value }
}

struct Pod: Holds[Zork] { // ERROR: cannot find type 'Zork'
    func hold(value: lang.i64) -> lang.i64 { value }
}
