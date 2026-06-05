// test: execution
// stdlib: true

// Simple opaque return: call protocol method on `some Shape`.

module Test

protocol Shape {
    func area() -> std.numeric.Int64
}

struct Circle {
    let radius: std.numeric.Int64
    public init(radius radius: std.numeric.Int64) { self.radius = radius }
}

extend Circle: Shape {
    public func area() -> std.numeric.Int64 { self.radius * self.radius }
}

func makeShape() -> some Shape {
    Circle(radius: 3)
}

@main
func main() -> lang.i64 {
    let s = makeShape();
    if s.area() != 9 { return 1 }
    0
}
