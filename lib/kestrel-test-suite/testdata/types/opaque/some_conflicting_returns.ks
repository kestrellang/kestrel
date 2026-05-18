// test: diagnostics
// stdlib: true

// Two branches return different concrete types through an opaque return.
// The unification of the return TyVar should produce a type mismatch.

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

struct Square {
    public init() {}
}
extend Square: Shape {
    public func area() -> lang.i64 { 2 }
}

func bad(flag: std.core.Bool) -> some Shape {
    if flag { return Circle() }
    Square() // ERROR: type mismatch
}
