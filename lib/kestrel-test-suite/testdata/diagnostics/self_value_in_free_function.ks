// test: diagnostics
// stdlib: false

// `Self` in value position is only meaningful inside an enclosing type body
// (extension, protocol, or struct/enum). Used in a free function it must
// produce a clear, Self-aware diagnostic.

module Test

struct Point {
    public let x: lang.i64
    public init(x x: lang.i64) { self.x = x }
}

func make() -> Point {
    Self(x: 0) // ERROR: 'Self'
}
