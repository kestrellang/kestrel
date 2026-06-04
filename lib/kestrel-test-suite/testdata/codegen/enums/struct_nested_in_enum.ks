// test: execution
// stdlib: true

module Test

struct Point {
    let x: std.numeric.Int64
    let y: std.numeric.Int64
}

enum Shape {
    case Circle(center: Point, radius: std.numeric.Int64)
    case Rectangle(origin: Point, width: std.numeric.Int64, height: std.numeric.Int64)
}

func get_value(s: Shape) -> std.numeric.Int64 {
    match s {
        .Circle(center: c, radius: r) => c.x + c.y + r,
        .Rectangle(origin: o, width: w, height: h) => o.x + o.y + w + h
    }
}

@main
func main() -> lang.i64 {
    let circle = Shape.Circle(center: Point(x: 10, y: 12), radius: 20);
    // 10 + 12 + 20 = 42
    if get_value(circle) != 42 { return 1 }
    0
}
