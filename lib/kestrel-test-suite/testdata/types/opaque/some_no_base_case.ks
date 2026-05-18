// test: diagnostics
// stdlib: false

// All returns are recursive — no concrete return path exists.
// Currently produces a conformance error because the return type
// stays unresolved. TODO: implement E469 for a clearer message.

module Test

protocol Shape {
    func area() -> lang.i64
}

func spin() -> some Shape { // ERROR: does not conform
    spin()
}
