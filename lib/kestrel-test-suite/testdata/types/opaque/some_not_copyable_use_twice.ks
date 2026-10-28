// test: diagnostics
// stdlib: false

// A `some P and not Copyable` value is move-only at use sites: binding it
// twice is a use-after-move, exactly like any other non-Copyable value.

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

func makeShape() -> some Shape and not Copyable {
    Token(7)
}

func caller() {
    let s = makeShape();
    let a = s;
    let b = s; // ERROR: use of moved value
}
