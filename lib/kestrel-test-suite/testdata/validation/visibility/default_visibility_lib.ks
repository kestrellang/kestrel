// skip: included file, not a standalone test
// Not a test: the `Lib` module the default-visibility tests include.
// Members without a visibility keyword are internal to `Lib`.
module Lib

public protocol Shape {
    func area() -> lang.i64
}

public enum Color {
    case Red
    case Blue
}

public struct Grid: Shape {
    public var size: lang.i64

    public init(size size: lang.i64) { self.size = size }

    // No keyword: internal to `Lib`.
    subscript(index: lang.i64) -> lang.i64 { get { lang.i64_mul(index, self.size) } }
    func secret() -> lang.i64 { 7 }

    public subscript(checked index: lang.i64) -> lang.i64 { get { index } }
    public func area() -> lang.i64 { lang.i64_mul(self.size, self.size) }
    public func color() -> Color { .Blue }
}
