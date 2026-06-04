// test: diagnostics
// stdlib: false

// Defaults must live in `extend SomeProtocol { ... }`, never inline in a
// protocol declaration. Inline bodies emit E417 (and nothing else — the
// conformance check skips them to avoid a misleading "must implement" E454).

module Test

protocol Factory {
    init()
    func value() -> lang.i64
    func twice() -> lang.i64 { // ERROR: cannot have a body
        lang.i64_add(self.value(), self.value())
    }
}

struct A {
    public init() {}
}

extend A: Factory {
    public init() {}
    public func value() -> lang.i64 { 7 }
    // No `twice` — must not provoke E454 either; only E417 above is the
    // honest error.
}
