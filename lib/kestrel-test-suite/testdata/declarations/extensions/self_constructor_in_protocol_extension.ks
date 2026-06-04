// test: execution
// stdlib: false
// expect-exit: 0

// `Self()` inside an `extend SomeProtocol { ... }` body dispatches through
// witness resolution to the conforming type's `init` — same dispatch path
// as `T()` where `T: SomeProtocol`. The result is `Self`, which monomorphizes
// to the concrete conforming type at the call site.

module Main

protocol Factory {
    init()
    func value() -> lang.i64
}

extend Factory {
    public func makeOther() -> Self {
        Self()
    }
}

struct A {
    public init() {}
}

extend A: Factory {
    public init() {}
    public func value() -> lang.i64 { 0 }
}

@main
func main() -> lang.i64 {
    let a = A();
    let b = a.makeOther();
    b.value()
}
