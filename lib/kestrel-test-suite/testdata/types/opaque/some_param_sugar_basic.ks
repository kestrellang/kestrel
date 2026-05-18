// test: diagnostics
// stdlib: false

// `some P` in parameter position desugars to a synthetic type parameter.
// No errors expected — this is pure sugar for generics.

module Test

protocol Drawable {
    func draw() -> lang.i64
}

struct Circle {
    public init() {}
}
extend Circle: Drawable {
    public func draw() -> lang.i64 { 1 }
}

func render(shape: some Drawable) -> lang.i64 {
    shape.draw()
}

func test() {
    let x = render(Circle());
}
