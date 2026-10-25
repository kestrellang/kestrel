# String Interpolation

Embed any expression in a string with `\(expression)`, optionally followed by a format specifier: `"\(price:.2)"`. Interpolation works in single-line and multi-line cooked strings, holes can nest arbitrarily, and the whole mechanism is protocol-driven — a type interpolates if it conforms to `Formattable`, and even the accumulating string type can be swapped out. Raw (`#"..."#`) string forms exist for the opposite job: text where backslashes and quotes must survive untouched.

## Basics

```kestrel
let name = "Ada";
let age: Int64 = 36;

let greeting = "Hello, \(name)!";           // "Hello, Ada!"
let info = "\(name) is \(age) years old";
let math = "sum: \(age + 6)";               // any expression
```

Holes nest — a string inside a hole can itself interpolate:

```kestrel
println("msg: \("got: \(name)")");     // msg: got: Ada
println("L1 \("L2 \("L3 \(name)")")"); // L1 L2 L3 Ada
```

## Format Specifiers

A colon inside the hole introduces a format spec: `\(value:spec)`. The spec grammar is `[[fill]align][sign][#][0][width][.precision][type]`.

```kestrel
let n: Int64 = 42;
"[\(n:>6)]"     // [    42]   right-align, width 6
"[\(n:<6)]"     // [42    ]   left-align
"[\(n:^6)]"     // [  42  ]   center
"[\(n:*>6)]"    // [****42]   custom fill character
"[\(n:05)]"     // [00042]    zero-pad
"\(n:+)"        // +42        always show sign

let byte: Int64 = 255;
"\(byte:x)"     // ff         hex (X for FF)
"\(byte:08x)"   // 000000ff   zero-pad + width + hex
"\(byte:#x)"    // 0xff       alternate form (also #b → 0b…, #o → 0o…)
"\(5:b)"        // 101        binary
"\(64:o)"       // 100        octal

let third = 1.0 / 3.0;
"\(third:.4)"        // 0.3333       precision (correctly rounded, half-even)
"\(12345.678:e)"     // 1.2345678e4  scientific
"\(0.375:%)"         // 37.5%        percent

let opt: Int64? = .Some(3);
"\(opt:?)"      // Some(3)    debug format
```

Precision on floats rounds the stored binary value correctly; the *default* float rendering is the shortest string that round-trips, switching to scientific automatically at extreme magnitudes (`\(1.0e20)` → `1e20`).

## Escape Sequences

Cooked strings (`"..."` and `"""..."""`) support:

| Escape | Meaning |
|---|---|
| `\n` `\r` `\t` | newline, carriage return, tab |
| `\\` `\"` `\'` | backslash, double quote, single quote |
| `\0` | NUL |
| `\xNN` | ASCII by hex (00–7F; out of range is E701) |
| `\u{NNNN}` | Unicode scalar, 1–6 hex digits (up to 10FFFF; invalid is E702) |
| `\` at end of line | line continuation — swallows the newline and following indentation |
| `\(expr)` | interpolation |

Anything else after a backslash is E700; a trailing lone `\` is E703.

```kestrel
let s = "col1\tcol2\nrow";
let arrow = "\u{2192}";        // →
let long = "one \
    two";                       // "one two"
```

## Multi-Line Strings

Triple quotes open a multi-line cooked string. Escapes and interpolation both work; indentation is stripped based on the closing delimiter's column:

```kestrel
let block = """
    Dear \(name),
      indented two more
    Bye.
    """;
// "Dear Ada,\n  indented two more\nBye."
```

Rules: the opening `"""` must be immediately followed by a newline (E705), the closing `"""` must sit on its own line (E706), and every content line must be indented at least as far as the closer (E704). To embed a literal `"""`, escape one quote (`\"""`) or use a raw form.

## Raw Strings

`#`-delimited strings are fully raw: **no escapes, no interpolation**. Use them for regex, JSON, HTML — anything where backslashes are data:

```kestrel
let regex = #"\d{3}-\d{4}"#;              // backslashes literal
let tag   = ##"<a href="/x" class="big">"##;  // ## lets `"#` appear in the body
let doc   = #"""
    literal \(not interpolated) \n
    """#;                                  // multi-line raw: indent strip only
```

Escalate the pound count until the closing delimiter (`"#`, `"##`, …) doesn't occur in your text. The four string forms at a glance:

| Form | Escapes | Interpolation | Multi-line |
|---|---|---|---|
| `"..."` | yes | yes | no |
| `"""…"""` | yes | yes | yes (indent strip) |
| `#"..."#` (any pound count) | no | no | no |
| `#"""…"""#` (any pound count) | no | no | yes (indent strip) |

## Interpolating Your Own Types

A value can appear in a hole if its type conforms to `Formattable`:

```kestrel
struct Point {
    var x: Int64;
    var y: Int64;
}

extend Point: Formattable {
    public func format(mutating into writer: some Formatter, options: FormatOptions) {
        writer.append("(\(self.x), \(self.y))");
    }
}

let p = Point(x: 3, y: 4);
println("point: \(p)");   // point: (3, 4)
```

`format(into:options:)` receives the requested `FormatOptions` (width, alignment, precision…), so conformers can honor format specs or ignore them.

## Custom Interpolation Targets

Interpolated literals aren't hard-wired to `String`. A literal like `"a\(b)c"` desugars to: create an accumulator, `appendLiteral(...)` / `appendInterpolation(...)` for each segment, then build the result. Any type conforming to `ExpressibleByStringInterpolation` (which refines `ExpressibleByStringLiteral`) can be the result type:

```kestrel
public protocol Interpolatable {
    init(literalCapacity literalCapacity: Int64, interpolationCount interpolationCount: Int64)
    mutating func appendLiteral(literal: String)
}

public protocol ExpressibleByStringInterpolation: ExpressibleByStringLiteral {
    type Interpolation: Interpolatable
    init(interpolation: Interpolation)
}
```

Your accumulator supplies its own `appendInterpolation(...)` overloads, which may constrain hole values to any protocol you like (an SQL builder might require `Bindable` instead of `Formattable`, making `"WHERE id = \(userInput)"` safe by construction):

```kestrel
let tagged: BracketString = "Hello, \(name)!";   // builds via BracketString's accumulator
```

## Diagnostics You May Hit

| Code | Meaning |
|---|---|
| E700 | Invalid escape sequence (valid: `\n \r \t \\ \" \' \0 \xNN \u{NNNN}`) |
| E701 | `\xNN` out of ASCII range (above 0x7F) |
| E702 | Malformed `\u{...}` (empty, too many digits, out of Unicode range) |
| E703 | Incomplete escape — string ends after `\` |
| E704 | Multi-line string content less indented than the closing `"""` |
| E705 | Multi-line opener `"""` not followed by a newline |
| E706 | Multi-line closer `"""` not on its own line |
| E707 | Unterminated string literal |
| — | `invalid expression in string interpolation` — the text inside `\(...)` doesn't parse (e.g. `\(t.0.0)`, where `0.0` lexes as a float) |
| — | `T !: Formattable` — the interpolated value's type has no `Formattable` conformance |

## See Also

- [Syntax](syntax.md) — literal forms and lexical structure
- [Protocols](protocols.md) and [Extensions](extensions.md) — conforming to `Formattable`
- [Error Handling](error-handling.md) — `Formattable` is also what throwing `main` requires of error types
- [Entry Points](entry-points.md)
