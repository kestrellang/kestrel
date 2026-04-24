// test: diagnostics
// stdlib: false
module Test
enum Shape {
    case Circle(radius: lang.f64)
}

func draw(shape: Shape) { }

func test() {
    draw(.Circle(5.0)) // ERROR: no matching overload
}
