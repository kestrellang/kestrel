// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.text.(StringBuilder, Interpolatable, ExpressibleByStringInterpolation, Formattable, FormatOptions)

// Custom accumulator that wraps interpolated values in brackets
struct BracketAccumulator: Interpolatable, Cloneable {
    var builder: StringBuilder

    init(literalCapacity literalCapacity: Int64, interpolationCount interpolationCount: Int64) {
        self.builder = StringBuilder(capacity: literalCapacity + interpolationCount * 16);
    }

    public func clone() -> BracketAccumulator {
        var c = BracketAccumulator(literalCapacity: 0, interpolationCount: 0);
        c.builder = self.builder.clone();
        c
    }

    mutating func appendLiteral(literal: String) {
        self.builder.append(literal);
    }

    // Custom appendInterpolation: wraps values in [brackets]
    mutating func appendInterpolation[T](value: T) where T: Formattable {
        self.builder.append("[");
        value.format(into: self.builder);
        self.builder.append("]");
    }

    mutating func build() -> BracketString {
        BracketString(inner: self.builder.build())
    }
}

// Custom result type using the bracket accumulator
struct BracketString: ExpressibleByStringInterpolation, Cloneable {
    type Interpolation = BracketAccumulator

    var inner: String

    init(inner inner: String) {
        self.inner = inner;
    }

    init(stringLiteral value: lang.str) {
        self.inner = "";
    }

    init(interpolation interpolation: BracketAccumulator) {
        var acc = interpolation;
        self.inner = acc.build().inner;
    }

    public func clone() -> BracketString {
        BracketString(inner: self.inner.clone())
    }
}

func main() -> lang.i64 {
    let name = "World";

    // Custom type via annotation — should use BracketAccumulator
    let tagged: BracketString = "Hello, \(name)!";
    if tagged.inner != "Hello, [World]!" { return 1 }

    // Default (no annotation) — should still use String
    let normal = "Hello, \(name)!";
    if normal != "Hello, World!" { return 2 }

    // Multiple interpolations
    let multi: BracketString = "\(1) + \(2) = \(3)";
    if multi.inner != "[1] + [2] = [3]" { return 3 }

    0
}
