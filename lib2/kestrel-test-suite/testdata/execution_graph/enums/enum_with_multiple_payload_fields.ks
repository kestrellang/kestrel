// test: diagnostics
// stdlib: false

module Main

enum Shape {
    case Circle(x: lang.i64, y: lang.i64, radius: lang.i64)
    case Rectangle(x: lang.i64, y: lang.i64, width: lang.i64, height: lang.i64)
}
