// test: diagnostics
// stdlib: false

module Main

struct Circle {
    let radius: lang.i64

    func area() -> lang.i64 {
        lang.i64_mul(lang.i64_mul(self.radius, self.radius), 3)
    }
}

struct Rectangle {
    let width: lang.i64
    let height: lang.i64

    func area() -> lang.i64 {
        lang.i64_mul(self.width, self.height)
    }
}

enum Shape {
    case CircleShape(c: Circle)
    case RectShape(r: Rectangle)
}

func getArea(s: Shape) -> lang.i64 {
    match s {
        .CircleShape(c) => c.area(),
        .RectShape(r) => r.area()
    }
}
