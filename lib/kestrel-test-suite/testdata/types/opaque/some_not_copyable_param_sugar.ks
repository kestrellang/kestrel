// test: execution
// stdlib: true

// `some P and not Copyable` in parameter position desugars to a synthetic
// type param with a `not Copyable` negative bound, so move-only values can
// be passed where plain `some P` would require Copyable.

module Test

protocol Shape {
    func area() -> std.numeric.Int64
}

struct Token: not Copyable {
    let value: std.numeric.Int64;
    public init(value: std.numeric.Int64) {
        self.value = value;
    }
}

extend Token: Shape {
    public func area() -> std.numeric.Int64 { self.value }
}

func readArea(shape: some Shape and not Copyable) -> std.numeric.Int64 {
    shape.area()
}

@main
func main() -> lang.i64 {
    if readArea(Token(5)) != 5 { return 1 }
    0
}
