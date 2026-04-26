# Values & Variables

Names hold values. Kestrel splits "a name I'll change" from "a name I won't" at the syntax level — the compiler enforces the difference.

## Variables

`let` binds a value once. `var` lets you change it.

```swift
let pi = 3.14159
var counter = 0

counter = counter + 1   // ok
// pi = 3.0             // compile error: pi is `let`
```

Type annotations are optional when the type is obvious from the right-hand side. Add one when you want to widen, narrow, or document:

```swift
let port: Int = 8080
let name: String = "kestrel"
var attempts: Int = 0
```

There's no uninitialized binding — every `let` and `var` must have a value at the point it's declared. If you genuinely don't have one yet, use `Optional`:

```swift
var session: Optional[Session] = .None
```

## Literals

Integer literals support decimal, hex, binary, and octal:

```swift
let a = 42
let b = 0xff       // 255
let c = 0b1010     // 10
let d = 0o755      // 493
```

Float literals require a `.`:

```swift
let pi = 3.14159
let small = 0.001
```

Booleans are `true` and `false`. Characters use single quotes; strings use double quotes:

```swift
let yes: Bool = true
let letter: Char = 'A'
let greeting: String = "hello"
```

Common escape sequences in strings: `\n`, `\t`, `\\`, `\"`, `\0`.

## String Interpolation

Embed any expression inside `"..."` with `\(...)`:

```swift
let name = "Morgana"
let level = 7
let line = "\(name) reached level \(level)!"
```

Interpolated expressions can be any code that produces a value, including method calls:

```swift
let summary = "\(items.count) items totaling $\(items.sum())"
```

The syntax is `\(...)` — not `${...}` or `#{...}`. Easy to mistype if you're coming from JavaScript or Ruby.

## Operators

Standard arithmetic and comparison work the way you'd expect:

```swift
let sum = 3 + 4
let product = 3 * 4
let quotient = 10 / 3
let remainder = 10 % 3

let equal = (a == b)
let less  = (a < b)
let any   = condA || condB
let both  = condA && condB
```

Bitwise: `&`, `|`, `^`, `<<`, `>>`, `~`. String concatenation uses `+`. For the full table with precedence and associativity, see [Reference → Operators](reference/operators.md). To define operators on your own types, see [Functions → Operator Overloading](functions/operator-overloading.md).

---

[← Turtle Graphics](tour/turtle-graphics.md) · [↑ The Kestrel Language](index.md) · [Functions →](functions/index.md)
