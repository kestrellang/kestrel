module Counter

// `bump` stands in for an operator hook: only its syntax may call it.
public protocol Bump {
    @sugarOnly
    func bumpHook() -> lang.i64
    func plain() -> lang.i64
}

public struct C {
    public var v: lang.i64
}

extend C: Bump {
    public func bumpHook() -> lang.i64 { 1 }
    // Inside the protocol's module the hook stays callable by name.
    public func plain() -> lang.i64 { self.bumpHook() }
}

extend Bump {
    public func viaDefault() -> lang.i64 { self.bumpHook() }
}
