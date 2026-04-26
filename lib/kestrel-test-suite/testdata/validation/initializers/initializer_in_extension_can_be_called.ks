// test: diagnostics
// stdlib: false

module Main

public struct Foo {
    var x: lang.i64
    public init() { self.x = 0; }
}

extend Foo {
    public init(value: lang.i64) {
        self.x = value;
    }
}

public func test() {
    let f = Foo(42);
}
