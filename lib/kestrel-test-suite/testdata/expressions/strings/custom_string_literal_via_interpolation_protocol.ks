// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.text.(StringBuilder, Interpolatable, ExpressibleByStringInterpolation, Formattable, FormatOptions)

// Minimal accumulator — only needed to satisfy the Interpolatable requirement.
struct TestAccumulator: Interpolatable, Cloneable {
    var buf: StringBuilder

    init(literalCapacity literalCapacity: Int64, interpolationCount interpolationCount: Int64) {
        self.buf = StringBuilder(capacity: literalCapacity);
    }

    public func clone() -> TestAccumulator {
        var c = TestAccumulator(literalCapacity: 0, interpolationCount: 0);
        c.buf = self.buf.clone();
        c
    }

    mutating func appendLiteral(literal: String) {
        self.buf.append(literal);
    }

    mutating func appendInterpolation[T](value: T) where T: Formattable {
        self.buf.append("?");
    }

    mutating func build() -> Tagged {
        Tagged(template: self.buf.build(), tag: 42)
    }
}

// Multi-field struct that conforms to ExpressibleByStringInterpolation
// (which refines ExpressibleByStringLiteral). The init(stringLiteral:) must
// be found transitively through the protocol refinement chain.
struct Tagged: ExpressibleByStringInterpolation, Cloneable {
    type Interpolation = TestAccumulator

    var template: String
    var tag: Int64

    init(template template: String, tag tag: Int64) {
        self.template = template;
        self.tag = tag;
    }

    init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        self.template = String(stringLiteral: ptr, length);
        self.tag = 99;
    }

    init(interpolation: TestAccumulator) {
        var acc = interpolation;
        self.template = acc.buf.build();
        self.tag = 42;
    }

    public func clone() -> Tagged {
        Tagged(template: self.template.clone(), tag: self.tag)
    }
}

@main
func main() -> lang.i64 {
    // Plain string literal (no interpolation) assigned to an ESI type.
    // Must route through init(stringLiteral:) found via transitive
    // conformance: ESI refines EBSL.
    let t: Tagged = "hello";
    if t.template != "hello" { return 1 }
    if t.tag != 99 { return 2 }

    0
}
