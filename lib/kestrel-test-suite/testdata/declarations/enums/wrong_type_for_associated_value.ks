// test: diagnostics
// stdlib: false

module Test

enum Shape {
    case Circle(radius: lang.f64)
}

func test() -> Shape {
    Shape.Circle(radius: "big") // ERROR: type mismatch
}
