// test: execution
// stdlib: false
// expect-exit: 0

// Two structs conform to the same protocol via separate extensions.
// `Self()` inside the protocol extension's default method must dispatch
// through the *caller's* witness — i.e., `a.makeOther()` constructs an A,
// `b.makeOther()` constructs a B. A bug that bound `Self` to a fixed type
// would make both branches return the same tag.

module Main

protocol Factory {
    init()
    func tag() -> lang.i64
}

extend Factory {
    public func makeOther() -> Self {
        Self()
    }
}

struct A { public init() {} }
struct B { public init() {} }

extend A: Factory {
    public init() {}
    public func tag() -> lang.i64 { 1 }
}

extend B: Factory {
    public init() {}
    public func tag() -> lang.i64 { 2 }
}

func main() -> lang.i64 {
    let a = A();
    let b = B();
    let a2 = a.makeOther();
    let b2 = b.makeOther();
    // a2.tag() == 1 (A's witness), b2.tag() == 2 (B's witness).
    lang.i64_sub(lang.i64_add(a2.tag(), b2.tag()), 3)
}
