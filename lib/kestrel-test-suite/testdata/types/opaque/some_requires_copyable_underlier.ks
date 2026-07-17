// test: diagnostics
// stdlib: false

// A plain `some P` promises callers a Copyable value; hiding a move-only
// concrete type behind it is rejected — the annotation needs an explicit
// `and not Copyable`.

module Test

protocol Shape {
    func area() -> lang.i64
}

struct Token: not Copyable {
    let value: lang.i64;
    public init(value: lang.i64) {
        self.value = value;
    }
}

extend Token: Shape {
    public func area() -> lang.i64 { self.value }
}

func makeShape() -> some Shape { // ERROR: opaque return type hides non-Copyable type 'Token'; add 'and not Copyable' to the return type
    Token(7)
}
