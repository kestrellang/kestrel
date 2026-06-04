// test: diagnostics
// stdlib: false

// Two `some Drawable` params are independent types — they can accept
// different concrete types. This should compile without errors.

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

struct Square {
    public init() {}
}
extend Square: Drawable {
    public func draw() -> lang.i64 { 2 }
}

func overlay(a: some Drawable, b: some Drawable) -> lang.i64 {
    lang.i64_add(a.draw(), b.draw())
}

func test() {
    let x = overlay(Circle(), Square());
}
