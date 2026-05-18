// test: diagnostics
// stdlib: false

// Mutual recursion where both functions have opaque returns
// and neither has a concrete base case.

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

func f() -> some Shape { // ERROR: circular opaque return type
    g()
}

func g() -> some Shape { // ERROR: circular opaque return type
    f()
}
