// test: execution
// stdlib: false
// expect-exit: 0

// A static method declared in `extend SomeProtocol { ... }` is callable via
// any conforming type's name: `A.staticMethod()` where `A: SomeProtocol`.
// Dispatches through witness with `MirTy::Named(A)` as the self_type so
// monomorphization picks A's witness.

module Main

protocol Factory {
    static func zero() -> Self
}

extend Factory {
    public static func magic() -> lang.i64 { 99 }
}

struct A {
    public init() {}
}

extend A: Factory {
    public static func zero() -> Self { A() }
}

func main() -> lang.i64 {
    lang.i64_sub(A.magic(), 99)
}
