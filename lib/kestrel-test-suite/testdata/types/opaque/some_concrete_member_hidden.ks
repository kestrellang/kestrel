// test: diagnostics
// stdlib: false

// Calling a concrete-type-only method on an opaque type should fail.
// Only protocol methods are visible through `some P`.

module Test

protocol Shape {
    func area() -> lang.i64
}

struct Circle {
    let radius: lang.i64
    public init(radius radius: lang.i64) { self.radius = radius }
    public func diameter() -> lang.i64 { lang.i64_mul(self.radius, 2) }
}

extend Circle: Shape {
    public func area() -> lang.i64 { self.radius }
}

func makeShape() -> some Shape {
    Circle(radius: 5)
}

func test() {
    let s = makeShape();
    s.area();
    s.diameter() // ERROR: no member
}
