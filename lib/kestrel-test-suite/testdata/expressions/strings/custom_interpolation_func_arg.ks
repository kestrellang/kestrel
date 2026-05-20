// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.text.(StringBuilder, Interpolatable, ExpressibleByStringInterpolation, Formattable, FormatOptions)

struct TagAccumulator: Interpolatable, Cloneable {
    var builder: StringBuilder

    init(literalCapacity literalCapacity: Int64, interpolationCount interpolationCount: Int64) {
        self.builder = StringBuilder(capacity: literalCapacity + interpolationCount * 16);
    }

    public func clone() -> TagAccumulator {
        var c = TagAccumulator(literalCapacity: 0, interpolationCount: 0);
        c.builder = self.builder.clone();
        c
    }

    mutating func appendLiteral(literal: String) {
        self.builder.append(literal);
    }

    mutating func appendInterpolation[T](value: T) where T: Formattable {
        self.builder.append("{");
        value.format(into: self.builder);
        self.builder.append("}");
    }

    mutating func build() -> TagString {
        TagString(inner: self.builder.build())
    }
}

struct TagString: ExpressibleByStringInterpolation, Cloneable {
    type Interpolation = TagAccumulator

    var inner: String

    init(inner inner: String) {
        self.inner = inner;
    }

    init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        self.inner = String(stringLiteral: ptr, length);
    }

    init(interpolation: TagAccumulator) {
        var acc = interpolation;
        self.inner = acc.build().inner;
    }

    public func clone() -> TagString {
        TagString(inner: self.inner.clone())
    }
}

// Function that takes a TagString — interpolation should infer the accumulator
// from the parameter type, not just from let annotations.
func process(query: TagString) -> String {
    query.inner
}

func main() -> lang.i64 {
    let name = "Alice";

    // Function argument inference: the expected type comes from the parameter
    let result = process("hello \(name)");
    if result != "hello {Alice}" { return 1 }

    // Multiple args
    let result2 = process("\(1) + \(2) = \(3)");
    if result2 != "{1} + {2} = {3}" { return 2 }

    0
}
