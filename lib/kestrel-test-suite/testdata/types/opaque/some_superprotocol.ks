// test: execution
// stdlib: true

// `some Shape` where Shape inherits from Identifiable.
// Both Shape and Identifiable methods should be accessible.

module Test

protocol Identifiable {
    func id() -> std.numeric.Int64
}

protocol Shape: Identifiable {
    func area() -> std.numeric.Int64
}

struct Circle {
    let r: std.numeric.Int64
    public init(r r: std.numeric.Int64) { self.r = r }
}

extend Circle: Identifiable {
    public func id() -> std.numeric.Int64 { 1 }
}

extend Circle: Shape {
    public func area() -> std.numeric.Int64 { self.r * self.r }
}

func makeShape() -> some Shape {
    Circle(r: 5)
}

@main
func main() -> lang.i64 {
    let s = makeShape();
    if s.area() != 25 { return 1 }
    if s.id() != 1 { return 2 }
    0
}
