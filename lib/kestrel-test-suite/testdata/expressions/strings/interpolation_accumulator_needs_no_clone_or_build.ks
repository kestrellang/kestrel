// test: execution
// stdlib: true
// expect-exit: 0
//
// An interpolated literal finishes through the result type's
// `ExpressibleByStringInterpolation.init(consuming interpolation:)`, which
// takes the accumulator by value. So an accumulator needs no `build()` and
// need not be `Cloneable` or even `Copyable`: here it is `not Copyable` and
// its buffer is moved straight into the result.

module Test

import std.text.(StringBuilder, Interpolatable, ExpressibleByStringInterpolation, Formattable)

struct OnceAccumulator: Interpolatable, not Copyable {
    var builder: StringBuilder
    var holes: Int64

    init(literalCapacity literalCapacity: Int64, interpolationCount interpolationCount: Int64) {
        self.builder = StringBuilder(capacity: literalCapacity + interpolationCount * 16);
        self.holes = 0;
    }

    mutating func appendLiteral(literal: String) {
        self.builder.append(literal);
    }

    mutating func appendInterpolation[T](value: T) where T: Formattable {
        value.format(into: self.builder);
        self.holes = self.holes + 1;
    }
}

struct Counted: ExpressibleByStringInterpolation {
    type Interpolation = OnceAccumulator

    var text: String
    var holes: Int64

    init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        self.text = String(stringLiteral: ptr, length);
        self.holes = 0;
    }

    init(consuming interpolation: OnceAccumulator) {
        var acc = interpolation;
        self.text = acc.builder.build();
        self.holes = acc.holes;
    }
}

@main
func main() -> lang.i64 {
    let n: Int64 = 7;
    let c: Counted = "n=\(n), twice=\(n * 2)";
    if c.text != "n=7, twice=14" { return 1 }
    if c.holes != 2 { return 2 }
    0
}
