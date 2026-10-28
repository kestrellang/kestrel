// test: diagnostics
// stdlib: false

// Only `Copyable` is legal as a negative bound on an opaque type — the same
// rule as conformance lists (E424: language-feature protocols only).

module Test

protocol Shape {
    func area() -> lang.i64
}

struct Circle {
    public init() {}
}

extend Circle: Shape {
    public func area() -> lang.i64 { 1 }
}

func makeShape() -> some Shape and not Shape { // ERROR: negative bound on an opaque type must be 'Copyable'
    Circle()
}
