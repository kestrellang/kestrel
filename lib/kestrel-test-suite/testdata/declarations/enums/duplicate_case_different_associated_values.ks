// test: diagnostics
// stdlib: false
module Test
enum Shape {
    case Circle(radius: lang.f64)
    case Circle(diameter: lang.f64) // ERROR: duplicate enum case
}
