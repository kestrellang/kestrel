// test: execution
// stdlib: true

// Protocol extension methods should be visible on `some P`.

module Test

protocol Shape {
    func area() -> std.numeric.Int64
}

extend Shape {
    public func doubleArea() -> std.numeric.Int64 { self.area() * 2 }
}

struct Circle {
    let r: std.numeric.Int64
    public init(r r: std.numeric.Int64) { self.r = r }
}

extend Circle: Shape {
    public func area() -> std.numeric.Int64 { self.r * self.r }
}

func makeShape() -> some Shape {
    Circle(r: 3)
}

@main
func main() -> lang.i64 {
    let s = makeShape();
    if s.area() != 9 { return 1 }
    if s.doubleArea() != 18 { return 2 }
    0
}
