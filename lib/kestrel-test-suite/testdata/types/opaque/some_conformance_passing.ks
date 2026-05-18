// test: execution
// stdlib: true

// Pass an opaque type to a generic function with a protocol constraint.
// `some Shape` conforms to Shape, so it satisfies `T: Shape`.

module Test

protocol Shape {
    func area() -> std.numeric.Int64
}

struct Circle {
    let r: std.numeric.Int64
    public init(r r: std.numeric.Int64) { self.r = r }
}

extend Circle: Shape {
    public func area() -> std.numeric.Int64 { self.r * self.r }
}

func makeShape() -> some Shape {
    Circle(r: 4)
}

func measure[T](s: T) -> std.numeric.Int64 where T: Shape {
    s.area()
}

func main() -> lang.i64 {
    let s = makeShape();
    if measure(s) != 16 { return 1 }
    0
}
