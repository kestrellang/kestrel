// test: execution
// stdlib: true
// expect-exit: 0
//
// An interpolated literal's static segments go to the
// `Interpolatable.appendLiteral` requirement — not whatever member is named
// `appendLiteral`. `Tape` also witnesses `Journal.appendLiteral(literal:)`
// (same name and labels, declared first), which must never be picked.

module Test

import std.text.(StringBuilder, Interpolatable, ExpressibleByStringInterpolation, Formattable)

protocol Journal {
    mutating func appendLiteral(literal: Int64)
}

struct Tape: Journal, Interpolatable {
    var builder: StringBuilder
    var journaled: Int64

    init(literalCapacity literalCapacity: Int64, interpolationCount interpolationCount: Int64) {
        self.builder = StringBuilder(capacity: literalCapacity);
        self.journaled = 0;
    }

    // Journal's witness: must not receive the literal segments.
    mutating func appendLiteral(literal: Int64) {
        self.journaled = self.journaled + literal;
    }

    // Interpolatable's witness.
    mutating func appendLiteral(literal: String) {
        self.builder.append(literal);
    }

    mutating func appendInterpolation[T](value: T) where T: Formattable {
        value.format(into: self.builder);
    }
}

struct Recorded: ExpressibleByStringInterpolation {
    type Interpolation = Tape

    var text: String
    var journaled: Int64

    init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        self.text = String(stringLiteral: ptr, length);
        self.journaled = 0;
    }

    init(consuming interpolation: Tape) {
        var tape = interpolation;
        self.text = tape.builder.build();
        self.journaled = tape.journaled;
    }
}

@main
func main() -> lang.i64 {
    let x: Int64 = 5;
    let r: Recorded = "a\(x)b";
    if r.text != "a5b" { return 1 }
    if r.journaled != 0 { return 2 }
    0
}
