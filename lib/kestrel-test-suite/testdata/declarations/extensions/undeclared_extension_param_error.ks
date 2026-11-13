// test: diagnostics
// stdlib: false

// A param only the protocol args use must be declared: `extend[T] Slot`.

module Test

protocol Holds[T] {
    func hold(value: T) -> T
}

struct Slot {}

extend Slot: Holds[T] { // ERROR: cannot find type 'T'
    func hold(value: T) -> T { value } // ERROR: cannot find type 'T'
}
