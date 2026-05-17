// test: diagnostics
// stdlib: false

module Test

struct Outer[T] {
    struct Inner[T] { // ERROR: shadows
        let value: T
    }
}
