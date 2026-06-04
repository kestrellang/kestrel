// test: execution
// stdlib: false
// expect-exit: 0

// `Self.staticMethod(...)` resolves through the extended type, the same way
// `Self(...)` does. Useful when one static factory delegates to another.

module Main

struct Counter {
    public let n: lang.i64
    public init(n n: lang.i64) { self.n = n }
}

extend Counter {
    public static func zero() -> Self {
        Self(n: 0)
    }
    public static func one() -> Self {
        // Self.zero() resolves to Counter.zero() via the extension target.
        Self.zero()
    }
}

@main
func main() -> lang.i64 {
    let c = Counter.one();
    c.n
}
