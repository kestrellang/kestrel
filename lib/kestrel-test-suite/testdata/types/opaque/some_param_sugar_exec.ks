// test: execution
// stdlib: true

// `some Drawable` in parameter position — call with different types.

module Test

protocol Drawable {
    func draw() -> std.numeric.Int64
}

struct Circle {
    public init() {}
}
extend Circle: Drawable {
    public func draw() -> std.numeric.Int64 { 10 }
}

struct Square {
    public init() {}
}
extend Square: Drawable {
    public func draw() -> std.numeric.Int64 { 20 }
}

func render(shape: some Drawable) -> std.numeric.Int64 {
    shape.draw()
}

@main
func main() -> lang.i64 {
    if render(Circle()) != 10 { return 1 }
    if render(Square()) != 20 { return 2 }
    0
}
