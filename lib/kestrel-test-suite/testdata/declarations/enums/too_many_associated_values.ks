// test: diagnostics
// stdlib: false

module Test

enum Shape {
    case Circle(radius: lang.f64)
}

func test() -> Shape {
    Shape.Circle(radius: 5.0, extra: 10.0) // ERROR: no matching overload
}
