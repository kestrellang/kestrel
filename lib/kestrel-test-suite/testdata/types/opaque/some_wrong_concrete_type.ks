// test: diagnostics
// stdlib: false

// Concrete return type does not conform to the opaque bound.
// Should produce a conformance error.

module Test

protocol Shape {
    func area() -> lang.i64
}

struct NotAShape {
    public init() {}
}

func makeShape() -> some Shape { // ERROR: does not conform
    NotAShape()
}
