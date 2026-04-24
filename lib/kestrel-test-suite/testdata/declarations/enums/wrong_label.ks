// test: diagnostics
// stdlib: false
module Test
enum Shape {
    case Circle(radius: lang.f64)
}

func test() -> Shape {
    Shape.Circle(r: 5.0) // ERROR: no matching overload
}
