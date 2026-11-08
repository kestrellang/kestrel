// Rosetta Code: Exceptions
// https://rosettacode.org/wiki/Exceptions
// Kestrel has no exceptions; failures are values of Result, propagated with `try`.
module Main

enum ParseError {
    case Empty
    case BadDigit(Char)
    case Negative(Int64)
}

extend ParseError: Formattable {
    public func format(mutating into writer: some Formatter, options: FormatOptions) {
        match self {
            .Empty => writer.append("empty input"),
            .BadDigit(c) => writer.append("bad digit '\(c)'"),
            .Negative(n) => writer.append("\(n) is negative"),
        }
    }
}

func parseNatural(s: String) -> Int64 throws ParseError {
    if s.isEmpty { throw .Empty }
    var n: Int64 = 0;
    var sign: Int64 = 1;
    for (i, c) in s.chars.iter().enumerate() {
        if i == 0 and c == '-' { sign = -1; continue }
        guard let .Some(d) = c.digitValue() else { throw .BadDigit(c) }
        n = n * 10 + Int64(from: d);
    }
    if sign < 0 { throw .Negative(-n) }
    n
}

// `try` re-throws the error to our caller.
func sumOf(a: String, b: String) -> Int64 throws ParseError {
    let x = try parseNatural(a);
    let y = try parseNatural(b);
    x + y
}

@main
func main() {
    for (a, b) in [("12", "30"), ("7", ""), ("4x", "1"), ("5", "-9")] {
        match sumOf(a, b) {
            .Ok(n) => println("\"\(a)\" + \"\(b)\" = \(n)"),
            .Err(e) => println("\"\(a)\" + \"\(b)\" failed: \(e)"),
        }
    }
}
