// test: diagnostics
// stdlib: false

module Test

struct Point {
    var x: lang.i64
    var y: lang.i64
}

enum Shape {
    case Circle(radius: lang.i64)
    case Rectangle(origin: Point)
}
