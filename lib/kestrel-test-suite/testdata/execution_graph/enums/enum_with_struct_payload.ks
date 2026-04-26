// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
}

struct Size {
    let width: lang.i64
    let height: lang.i64
}

enum Shape {
    case Circle(center: Point, radius: lang.i64)
    case Rectangle(origin: Point, size: Size)
}
