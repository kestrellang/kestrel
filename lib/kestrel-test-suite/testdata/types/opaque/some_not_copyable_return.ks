// test: execution
// stdlib: true

// `some P and not Copyable` — a move-only concrete type may hide behind the
// opaque return; protocol methods still dispatch correctly at runtime.

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

func makeShape() -> some Shape and not Copyable {
    Token(7)
}

@main
func main() -> lang.i64 {
    let s = makeShape();
    if s.area() != 7 { return 1 }
    0
}
