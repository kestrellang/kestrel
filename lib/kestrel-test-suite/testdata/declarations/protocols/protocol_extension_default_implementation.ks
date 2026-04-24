// test: diagnostics
// stdlib: false

module Test

public protocol Parent {
    func parentMethod() -> lang.i64
}

// Provide default implementation in an extension
extend Parent {
    public func parentMethod() -> lang.i64 {
        42
    }
}

public protocol Child: Parent {
    func childMethod() -> lang.i64
}

// MyStruct should only need to implement childMethod,
// not parentMethod (it has a default implementation)
public struct MyStruct: Child {
    public func childMethod() -> lang.i64 {
        10
    }
}
