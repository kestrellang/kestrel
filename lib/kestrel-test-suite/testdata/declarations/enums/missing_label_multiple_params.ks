// test: diagnostics
// stdlib: false
module Test
enum Shape {
    case Rectangle(width: lang.f64, height: lang.f64)
}

func test() -> Shape {
    Shape.Rectangle(10.0, 20.0) // ERROR: no matching overload
}
