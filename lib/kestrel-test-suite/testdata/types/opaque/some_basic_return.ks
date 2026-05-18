// test: diagnostics
// stdlib: false

// Basic opaque return type: func returning `some P` where the body
// returns a concrete type conforming to P. No errors expected.

module Test

protocol Shape {
    func area() -> lang.i64
}

struct Circle {
    let radius: lang.i64
    public init(radius radius: lang.i64) { self.radius = radius }
}

extend Circle: Shape {
    public func area() -> lang.i64 { self.radius }
}

func makeShape() -> some Shape {
    Circle(radius: 5)
}
