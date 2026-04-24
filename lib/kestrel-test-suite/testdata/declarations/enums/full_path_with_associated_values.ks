// test: diagnostics
// stdlib: false

module Test

enum Shape {
    case Circle(radius: lang.f64)
    case Rectangle(width: lang.f64, height: lang.f64)
}

func getShape() -> Shape {
    Shape.Circle(radius: 5.0)
}
