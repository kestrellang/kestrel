// test: diagnostics
// stdlib: false

// Mutual recursion where both functions have opaque returns
// and neither has a concrete base case. Currently produces
// conformance errors because the opaque return resolves to ?.
// TODO: implement E470 "circular opaque type inference"

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

func f() -> some Shape { // ERROR: does not conform
    g()
}

func g() -> some Shape { // ERROR: does not conform
    f()
}
