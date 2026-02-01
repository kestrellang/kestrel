# String Interpolation Design

## Overview

String interpolation allows embedding expressions directly within string literals, with optional format specifiers for controlling output formatting.

```kestrel
let name = "Alice"
let age = 30

let greeting = "Hello, \(name)!"           // "Hello, Alice!"
let info = "\(name) is \(age) years old"   // "Alice is 30 years old"
let padded = "Value: \(age:>5)"            // "Value:    30"
let hex = "Code: \(code:08x)"              // "Code: 0000002a"
```

## Syntax

### Basic Interpolation

```
"\(expression)"
```

The expression is evaluated, formatted using `Formattable.format(options: .default)`, and inserted into the string.

### With Format Specifier

```
"\(expression:format_spec)"
```

The format spec is parsed at compile-time into `FormatOptions` and passed to `format(options:)`.

### Format Spec Grammar

```
format_spec    := [[fill]align][sign][#][0][width][.precision][type]

fill           := <any character>
align          := '<' | '>' | '^'           // left, right, center
sign           := '+' | '-' | ' '           // always, negative-only, space-for-positive
width          := <integer>
precision      := <integer>
type           := 'b' | 'o' | 'x' | 'X'     // binary, octal, hex lower, hex upper
               |  'e' | 'E' | 'f' | '%'     // scientific, fixed, percent
               |  '?'                        // debug
```

### Format Spec Examples

| Spec | Meaning | Example Input | Output |
|------|---------|---------------|--------|
| `>8` | right-align, width 8 | `42` | `"      42"` |
| `<8` | left-align, width 8 | `42` | `"42      "` |
| `^8` | center, width 8 | `42` | `"   42   "` |
| `08` | zero-pad, width 8 | `42` | `"00000042"` |
| `+` | always show sign | `42` | `"+42"` |
| `#x` | hex with 0x prefix | `42` | `"0x2a"` |
| `#X` | hex upper with 0X prefix | `42` | `"0X2A"` |
| `08x` | zero-pad hex, width 8 | `42` | `"0000002a"` |
| `.2` | precision 2 | `3.14159` | `"3.14"` |
| `.2e` | scientific, precision 2 | `3.14159` | `"3.14e0"` |
| `%` | percentage | `0.5` | `"50%"` |
| `?` | debug format | `"hi"` | `"\"hi\""` |
| `*>10` | fill with *, right-align | `42` | `"********42"` |

## Protocols

### Formattable (Updated)

```kestrel
/// Protocol for types that can be formatted as a string.
public protocol Formattable {
    /// Returns this value formatted as a string with the given options.
    func format(options: FormatOptions = .default) -> String
}
```

All primitive types (`Int`, `Float`, `Bool`, `String`, `Char`) conform to `Formattable`.

### ExpressibleByStringLiteral (Existing)

```kestrel
/// Protocol for types that can be created from a simple string literal.
public protocol ExpressibleByStringLiteral {
    init(stringLiteral: String)
}
```

### StringInterpolationProtocol (New)

```kestrel
/// Protocol for types that accumulate string interpolation parts.
public protocol StringInterpolationProtocol {
    /// Initialize with capacity hints for optimization.
    init(literalCapacity: Int64, interpolationCount: Int64)

    /// Append a literal string segment.
    mutating func appendLiteral(literal: String)

    /// Append a formatted value with options.
    mutating func appendInterpolation[T: Formattable](value: T, options: FormatOptions)
}
```

### ExpressibleByStringInterpolation (New)

```kestrel
/// Protocol for types that can be created from string interpolation.
/// Refines ExpressibleByStringLiteral.
public protocol ExpressibleByStringInterpolation: ExpressibleByStringLiteral {
    /// The type used to accumulate interpolation parts.
    type Interpolation: StringInterpolationProtocol

    /// Create from a completed interpolation.
    init(interpolation: Interpolation)
}
```

### Default Implementation for String

```kestrel
public struct DefaultStringInterpolation: StringInterpolationProtocol {
    private var parts: [String]

    public init(literalCapacity: Int64, interpolationCount: Int64) {
        self.parts = []
    }

    public mutating func appendLiteral(literal: String) {
        if !literal.isEmpty {
            parts.append(literal)
        }
    }

    public mutating func appendInterpolation[T: Formattable](value: T, options: FormatOptions) {
        parts.append(value.format(options: options))
    }

    public func build() -> String {
        concat(parts)
    }
}

extend String: ExpressibleByStringInterpolation {
    public type Interpolation = DefaultStringInterpolation

    public init(interpolation: DefaultStringInterpolation) {
        self = interpolation.build()
    }
}
```

### FormatOptions

```kestrel
public struct FormatOptions: Equatable {
    /// Minimum field width.
    public var width: Optional[Int64]

    /// For floats: decimal places. For strings: max characters.
    public var precision: Optional[Int64]

    /// Text alignment within the field width.
    public var alignment: Alignment

    /// Character used for padding (default: ' ').
    public var fill: Char

    /// Numeric base: 2 (binary), 8 (octal), 10 (decimal), 16 (hex).
    public var radix: Int64

    /// Use uppercase for hex digits (A-F vs a-f).
    public var uppercase: Bool

    /// How to display the sign for numbers.
    public var sign: Sign

    /// Alternate form: show 0x/0b/0o prefix for non-decimal radix.
    public var alternate: Bool

    /// Float display style (fixed, scientific, percent).
    public var floatStyle: FloatStyle

    /// Debug mode: show structural representation.
    public var debug: Bool

    /// Default format options.
    public static var default: FormatOptions { get }
}

public enum Alignment: Equatable, Matchable {
    case left
    case right
    case center
}

public enum Sign: Equatable, Matchable {
    case negative   // Only show sign for negative (default)
    case always     // Always show sign (+ or -)
    case space      // Space for positive, minus for negative
}

public enum FloatStyle: Equatable, Matchable {
    case default     // Shortest representation
    case fixed       // Fixed-point (3.14)
    case scientific  // Scientific with lowercase e
    case scientificUpper  // Scientific with uppercase E
    case percent     // Multiply by 100, add %
}
```

## Semantic Behavior

### Type Inference

```kestrel
// No context → defaults to String
let a = "Hello \(name)!"                    // String

// Explicit type annotation
let b: String = "Hello \(name)!"            // String

// Context from function parameter
func log(msg: LogMessage) { ... }
log("User \(id) logged in")                 // LogMessage

// Context from assignment
let query: SQLQuery = "SELECT * FROM \(table)"  // SQLQuery
```

### Expression Evaluation

- Expressions inside `\(...)` are evaluated left-to-right
- Full Kestrel expressions are allowed (calls, operators, closures, etc.)
- The result must conform to `Formattable`

```kestrel
"\(a + b)"                    // Binary operation
"\(items.count)"              // Property access
"\(compute(x, y))"            // Function call
"\(items.map { $0 * 2 })"     // Closure (if result is Formattable)
"\(if flag { "yes" } else { "no" })"  // If expression
```

### Nested Interpolation

Allowed and handled recursively:

```kestrel
let inner = "world"
let outer = "Hello \("dear \(inner)")!"  // "Hello dear world!"
```

### Raw Strings

Raw strings (`"""..."""`) do **not** support interpolation:

```kestrel
let raw = """Hello \(name)!"""  // Literal: "Hello \(name)!"
```

### Format Spec Parsing

Format specs are parsed at **compile-time**:
- Invalid specs produce compile-time errors
- The parsed spec becomes a `FormatOptions` value in generated code

```kestrel
"\(x:>8)"   // Valid
"\(x:z)"    // Compile error: invalid type specifier 'z'
"\(x:>)"    // Compile error: width expected after '>'
```

### Type Mismatches

Using a format spec incompatible with the type:

```kestrel
"\(name:x)"   // Error: hex format 'x' not valid for String
"\(42:.2)"    // OK: precision ignored for integers (or error - TBD)
```

## Code Generation

For `"Hello \(name:>10)!"`, generate:

```kestrel
{
    var __interp = String.Interpolation(literalCapacity: 7, interpolationCount: 1)
    __interp.appendLiteral("Hello ")
    __interp.appendInterpolation(name, options: FormatOptions(
        width: .Some(10),
        alignment: .right,
        // ... other fields at defaults
    ))
    __interp.appendLiteral("!")
    String(interpolation: __interp)
}
```

### Optimization: Simple Strings

For strings without interpolation, use `ExpressibleByStringLiteral` directly:

```kestrel
let s: String = "hello"
// Generates: String(stringLiteral: "hello")
// NOT the interpolation machinery
```

### Optimization: Single Interpolation, No Literals

```kestrel
let s = "\(value)"
// Could optimize to: value.format(options: .default)
```

## Lexer Changes

### New Tokens

```rust
StringStart,         // Opening " of an interpolated string
StringPart,          // Literal text segment
InterpolationStart,  // \(
FormatSpec,          // :>8.2f (including the colon)
InterpolationEnd,    // ) closing the interpolation
StringEnd,           // Closing "

// Keep existing for simple strings
String,              // Complete simple string with no interpolation
```

### Lexer State Machine

The lexer maintains a mode stack:

```
enum LexerMode {
    Normal,
    InString,
    InInterpolation { paren_depth, bracket_depth, brace_depth },
}
```

Transitions:
- `Normal` + `"` → push `InString`, emit `StringStart`
- `InString` + `\(` → push `InInterpolation{1,0,0}`, emit `InterpolationStart`
- `InString` + `"` → pop, emit `StringEnd`
- `InString` + text → accumulate into `StringPart`
- `InInterpolation` + `(` → increment paren_depth
- `InInterpolation` + `)` at depth 1 → pop, emit `InterpolationEnd`
- `InInterpolation` + `:` at depth (1,0,0) → emit `FormatSpec` until `)`
- `InInterpolation` + `"` → push `InString` (nested string)
- `InInterpolation` + other → normal token lexing

### Token Stream Examples

`"Hello \(name)!"`:
```
StringStart
StringPart("Hello ")
InterpolationStart
Identifier("name")
InterpolationEnd
StringPart("!")
StringEnd
```

`"Value: \(x:>8)"`:
```
StringStart
StringPart("Value: ")
InterpolationStart
Identifier("x")
FormatSpec(":>8")
InterpolationEnd
StringEnd
```

Simple string `"hello"` (no interpolation):
```
String("hello")
```

## Error Cases

| Condition | Error Message |
|-----------|---------------|
| Unterminated interpolation | `unterminated string interpolation, expected ')'` |
| Unterminated string in interpolation | `unterminated string literal inside interpolation` |
| Empty interpolation `"\()"` | `expected expression in string interpolation` |
| Missing expression `"\(:>5)"` | `expected expression before format specifier` |
| Invalid format spec `"\(x:z)"` | `invalid format specifier 'z'` |
| Type not Formattable | `type 'Foo' does not conform to 'Formattable'` |
| Incompatible format spec | `format specifier 'x' not valid for type 'String'` |

## Edge Cases

### Escaped Backslash Before Paren

```kestrel
"\\(not interpolation)"  // Literal: \(not interpolation)
"\(interpolation)"       // Interpolates
```

### Consecutive Interpolations

```kestrel
"\(a)\(b)\(c)"  // Valid: concatenates formatted a, b, c
```

### Empty Literal Parts

```kestrel
"\(a)"       // No literals, just one interpolation
"\(a)\(b)"   // Literals: ["", "", ""], values: [a, b]
```

### Deeply Nested

```kestrel
"\(a + "\(b + "\(c)")")"  // Valid but discouraged
```

### Interpolation at String Boundaries

```kestrel
"\(x)"      // Just the interpolation
"x\(y)"     // Literal prefix
"\(x)y"     // Literal suffix
```

### Format Spec with Special Characters

```kestrel
"\(x:*>10)"  // Fill with '*'
"\(x:->10)"  // Fill with '-'
"\(x: >10)"  // Fill with ' ' (space, explicit)
```

## Open Questions (Resolved)

### Q: Parentheses or Curly Braces?
**A**: Parentheses `\(expr)` - more familiar from Swift/Kotlin.

### Q: Format Specs from Start?
**A**: Yes, include format specifiers in initial implementation.

### Q: Compile-time or Runtime Spec Parsing?
**A**: Compile-time - better error messages, no runtime overhead.

### Q: Protocol for String Building?
**A**: Yes, `ExpressibleByStringInterpolation` + `StringInterpolationProtocol` for extensibility.

### Q: String Concatenation Strategy?
**A**: Accumulate parts in array, then `concat()` - simple and efficient.

### Q: Raw String Interpolation?
**A**: No - raw strings are truly raw.

## Future Extensions

### Custom Interpolation Methods

Types could add overloads for specific value types:

```kestrel
extend SQLInterpolation {
    // Special handling for SQL-injectable types
    mutating func appendInterpolation(value: String, options: FormatOptions) {
        sql.append("?")
        params.append(value.escaped())
    }
}
```

### Interpolation with Labels

```kestrel
"\(value, radix: 16)"  // Alternative to format spec
```

### Localization Support

```kestrel
let msg: LocalizedString = "Hello \(name)!"
// Tracks which parts are translatable
```
