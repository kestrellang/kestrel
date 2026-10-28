// test: execution
// stdlib: true

// `and not Copyable` only relaxes the requirement on the underlier — a
// Copyable concrete type is still allowed behind it. Use sites conservatively
// treat the value as move-only either way.

module Test

protocol Shape {
    func area() -> std.numeric.Int64
}

struct Plain {
    let value: std.numeric.Int64;
    public init(value: std.numeric.Int64) {
        self.value = value;
    }
}

extend Plain: Shape {
    public func area() -> std.numeric.Int64 { self.value }
}

func makeShape() -> some Shape and not Copyable {
    Plain(3)
}

@main
func main() -> lang.i64 {
    let s = makeShape();
    let t = s; // moves — the opaque type is treated as non-Copyable
    if t.area() != 3 { return 1 }
    0
}
