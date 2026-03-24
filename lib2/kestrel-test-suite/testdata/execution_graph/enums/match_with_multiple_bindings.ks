// test: diagnostics
// stdlib: false

module Main

enum Shape {
    case Circle(x: lang.i64, y: lang.i64, radius: lang.i64)
    case Point(x: lang.i64, y: lang.i64)
}

func getX(s: Shape) -> lang.i64 {
    match s {
        .Circle(x, y, radius) => x,
        .Point(x, y) => x
    }
}
