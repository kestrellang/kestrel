---
name: write-kestrel
description: Quick reference for writing Kestrel source code — syntax, semantics, idioms, and the label/access-mode rules that bite first-time writers. Use when writing or editing `.ks` files, when the user asks "how do I write X in Kestrel?", when drafting stdlib modules, or any time the task involves producing Kestrel code rather than modifying the compiler. Skip for compiler-internals work (Rust code in `lib/`), which is covered by `kestrel-pipeline` / `debug-kestrel`.
---

# Kestrel Language Quick Reference

## Overview

Statically-typed language with copy-by-default value semantics, explicit parameter access modes (borrowing, mutating, consuming), protocol-based polymorphism, monomorphized generics, and RAII resource management. Semicolons are required after statements.

## Variables

```kestrel
let x: Int64 = 42;        // immutable
let message = "Hello";     // type inferred
var count: Int32 = 0;      // mutable
count = count + 1;
```

## Functions

Methods do NOT declare `self` — it's implicit.

```kestrel
func add(x: Int64, y: Int64) -> Int64 { x + y }
func add(x: Int64, y: Int64) -> Int64 = x + y   // expression-bodied
func identity[T](value: T) -> T = value
func compare[T](a: T, b: T) -> Bool where T: Comparable { }
```

## Parameter Labels

Single-name params are positional (no label at call site — unlike Swift). Add an external label before the internal name to require it.

```kestrel
func add(x: Int64, y: Int64) -> Int64 { x + y }
add(1, 2)                                        // positional

func send(to recipient: String) { }
send(to: "alice@example.com")                     // labeled

// Overloading by label
func move(to destination: Point) { }
func move(by offset: Point) { }
```

Order with access modes: `accessMode externalLabel internalName: Type`

```kestrel
func offset(mutating point p: Point, by delta: Int64) {
    p.x = p.x + delta;
}
offset(point: myPoint, by: 5)
```

## Parameter Access Modes

```kestrel
func read(p: Point) -> Int64 { p.x }             // borrowing (default, read-only)
func reset(mutating p: Point) { p.x = 0; }       // mutating (caller must pass var)
func consume(consuming f: File) { }               // consuming (takes ownership)
```

## Structs

```kestrel
struct Point {
    var x: Int64;
    var y: Int64;

    // Instance method (self is implicit)
    func sum() -> Int64 { self.x + self.y }

    // Mutating method
    mutating func offset(by: Int64) { self.x = self.x + by; }

    // Static method
    static func origin() -> Point { Point(x: 0, y: 0) }

    // RAII cleanup
    deinit { }
}
```

### Initializers

```kestrel
// No custom init → memberwise (labels = field names)
let p = Point(x: 10, y: 20);

// Custom init without labels → positional
init(x: Int64, y: Int64) { self.x = x; self.y = y; }
let p = Point(1, 2);

// Custom init with labels → labeled
init(atX x: Int64, atY y: Int64) { self.x = x; self.y = y; }
let p = Point(atX: 5, atY: 10);
```

### Copy vs Move

```kestrel
let p2 = p1;  // COPIED — both valid (default)

struct File: not Copyable { var handle: Int64; }
let f2 = f1;  // MOVED — f1 invalid
```

## Enums

```kestrel
enum Direction { case North; case South; case East; case West }
let d: Direction = .North;

enum Shape {
    case Circle(radius: Float64)              // labeled
    case Rectangle(width: Float64, height: Float64)
    case Point
}
let s = Shape.Circle(radius: 5.0);

enum Option[T] { case Some(T); case None }    // positional
let opt = Option.Some(42);

indirect enum Tree[T] {                        // recursive
    case Leaf(value: T)
    case Node(left: Tree[T], right: Tree[T])
}
```

## Pattern Matching

```kestrel
match value {
    0 => "Zero",
    1..<10 => "Small",                         // exclusive range
    1..=10 => "Small",                         // inclusive range
    .Circle(radius: r) => r,                   // destructure
    .Some(x) if x > 10 => x,                  // guard
    _ => "Other"
}

if let .Some(val) = optional { }               // if-let
guard let .Some(val) = optional else { return; } // guard-let
while let .Some(item) = iter.next() { }        // while-let
```

## Control Flow

```kestrel
if x > 0 { } else if x < 0 { } else { }

guard x > 0 else { return; }

while condition { }

loop { if done { break; } }

// Labeled loops
outer: loop { while true { break outer; } }

// For-in
for elem in collection { }
for i in 0..<10 { }
for i in 0..=9 { }
```

## Closures

```kestrel
let add = { (a: Int64, b: Int64) in a + b };
let double: (Int64) -> Int64 = { it * 2 };     // single param → `it`
numbers.map { it * 2 }                          // trailing closure
```

Closures capture by value (copied at creation time).

## Types

### Primitives

- Integers: `Int8`, `Int16`, `Int32`, `Int64` / `UInt8`, `UInt16`, `UInt32`, `UInt64`
- Floats: `Float16`, `Float32`, `Float64`
- Boolean: `Bool`
- String: `String`
- Unit: `()`
- Never: `!`

### Type Operators (sugar)

- `T?` → `Optional[T]`
- `[T]` → `Array[T]`
- `[K: V]` → `Dictionary[K, V]`
- `T throws E` → `Result[T, E]`

```kestrel
let name: String? = .None;
let nums: [Int64] = [1, 2, 3];
let ages: [String: Int64] = [:];
func parse(input: String) -> Int64 throws ParseError { }
```

### Composite Types

```kestrel
(Int64, String)             // tuple
[Int64]                     // array
(Int64, Int64) -> Int64     // function type
lang.ptr[Int64]             // pointer (unsafe)
```

### Type Aliases

```kestrel
type ID = String;
type Handler = (Int64) -> Bool;
type Pair[T] = (T, T);
```

### String Forms

| Form | Multi-line? | Escapes? | Interpolation? |
|---|---|---|---|
| `"..."` | no | yes | yes |
| `"""\n...\n"""` | **yes** (Swift-style indent strip from closing `"""` column) | yes | yes |
| `#"..."#` | no | no | no |
| `#"""\n...\n"""#` | yes | no | no |
| `##"..."##`, `##"""\n...\n"""##`, etc. | escalate pound count to embed `"#`, `"##`, etc. literally | no | no |

Multi-line cooked rules: the opening `"""` must be followed immediately by `\n`; the closing `"""` must be on its own line (only whitespace before it). The closing line's indentation column defines the strip prefix — every content line must start with at least that whitespace, otherwise E704.

`#`-prefixed forms are **fully raw** — no escapes, no interpolation, no `\#(...)` escalator. Use them for embedded source (regex, HTML, CSS, JSON, JS) where backslashes and quotes shouldn't be touched. Pick the smallest pound count whose closer (`"#`, `"##`, …) doesn't appear in the body.

```kestrel
let html  = ##"<a href="/x" class="big">"##;     // single-line raw
let regex = #"\d{3}-\d{4}"#;                     // single-line raw
let block = """
    line one
    line two
    """;                                          // multi-line cooked → "line one\nline two"
let css   = ##"""
*{box-sizing:border-box}
"""##;                                            // multi-line raw
```

### String Interpolation

```kestrel
let greeting = "Hello, \(name)!";
let info = "\(name) is \(age) years old";
let padded = "Value: \(age:>5)";       // right-align, width 5
let hex = "Code: \(code:08x)";         // zero-pad, width 8, hex
let debug = "\(value:?)";              // debug format
```

Format specifiers: `>` right-align, `<` left-align, `^` center, `0` zero-pad, `x`/`X` hex, `b` binary, `o` octal, `.n` precision. Interpolation works in both single-line and multi-line cooked strings (`"..."` and `"""..."""`); raw forms (`#"..."#` etc.) do **not** support interpolation.

## Protocols

```kestrel
protocol Drawable {
    func draw();
    mutating func reset();
    static func default() -> Self;
}

protocol Container { type Element; func get() -> Element; }
```

### Conformance

```kestrel
struct Circle: Drawable { }                    // direct
extend Circle: Hashable { func hash() -> Int64 { 0 } }  // via extension
extend Drawable { func redraw() { self.draw(); } }       // default impl
```

## Generics

```kestrel
struct Box[T] { var value: T; }
struct Pair[A, B] { var first: A; var second: B; }
struct SortedList[T] where T: Comparable { var items: [T]; }
struct Container[T] where T: not Copyable { var value: T; }
```

## Extensions

```kestrel
extend Point { func distance(other: Point) -> Float64 { } }
extend Point: Hashable { func hash() -> Int64 { } }
extend Box[T] where T: Equatable { func equals(other: Box[T]) -> Bool { } }
extend Box[Int64] { func doubled() -> Int64 { self.value * 2 } }
```

## Computed Properties & Subscripts

```kestrel
struct Rectangle {
    var width: Float64;
    var height: Float64;
    var area: Float64 { self.width * self.height }       // getter shorthand
    var diagonal: Float64 {
        get { sqrt(self.width * self.width + self.height * self.height) }
        set { }                                           // newValue is implicit
    }
    static var zero: Rectangle { Rectangle(width: 0.0, height: 0.0) }
}

struct Grid {
    subscript(row r: Int64, col c: Int64) -> Int64 {
        get { self.data(r * width + c) }
        set { self.data(r * width + c) = newValue; }
    }
}
let val = grid(row: 0, col: 1);
```

## Error Handling

```kestrel
// Using Result sugar
func read() -> String throws FileError {
    if success { return data; }
    throw FileError.NotFound;
}

// Try operator (propagates errors)
func process() -> Data throws Error {
    let content = try readFile();
    return parse(content);
}

// Try with default
let value = try someOperation() ?? defaultValue;
```

## Modules and Imports

```kestrel
module MyApp.Utils

import std.collections.Array
import std.io as IO
import Library.(Item1, Item2)
import ModuleA.(Widget as WidgetA)
public import internal.types.Core   // re-export
```

Visibility: `public`, `internal` (default), `fileprivate`, `private`.

## Common Patterns

```kestrel
struct Config { static func default() -> Config { } }                // factory
struct Builder {
    mutating func set(value: Int64) { }
    consuming func build() -> Product { }
}
struct Connection: not Copyable { deinit { self.close(); } }         // RAII
```

## Style

- **Naming**: `PascalCase` types/protocols/enums; `camelCase` functions/methods/variables; `SCREAMING_SNAKE_CASE` constants. No abbreviations in public APIs — `count`, `pointer`, `address`, not `cnt`, `ptr`, `addr`.
- **Mutability**: prefer `let`; `var` only when the binding mutates.
- **Integers**: use type annotations (`let x: Int32 = 42`) not constructors (`Int32(intLiteral: 42)`).
- **Self**: `self` = borrowing; `mutating self` = modify fields; `consuming self` = take ownership.

## Idioms

- **Mutating = verb, non-mutating = past participle.**
  ```kestrel
  sort() / sorted()       reverse() / reversed()
  trim() / trimmed()      formUnion(with:) / union(with:)
  ```
- **`to*` converts (allocates), `as*` views (no copy).**
  ```kestrel
  toArray()       // new value
  asSlice()       // cheap reinterpretation
  ```
- **Prefer enums over booleans** at call sites.
  ```kestrel
  sort(order: .ascending)     // good
  sort(ascending: true)       // bad — opaque
  ```
- **Properties = state, methods = actions.** `count`, `isEmpty`, `capacity` are properties everywhere. `collect()`, `fold()`, `iter()` are methods.
- **Closure labels are standardized:** predicates `matching:`, key extractors `byKey:`, combining `combining:`, mapping `mapping:`.
  ```kestrel
  filter(matching: { it > 0 })
  sort(byKey: { it.name })
  fold(from: 0, combining: { a + b })
  ```
- **Labels are prepositions** — `with:`, `from:`, `by:`, `of:`, `at:`. Not bare nouns like `predicate:` or `action:`.
- **Prefer `for` over `while` for iteration.** Use `for elem in collection`, `for i in 0..<n`.
- **Avoid indexing strings.** Use views and iterators; prefer utf8 operations when possible.
- **Prefer early returns.** Use `guard` for preconditions instead of deep nesting.
  ```kestrel
  guard x > 0 else { return; }
  ```

## Label Rules Summary

| Declaration              | Call Site                |
| ------------------------ | ------------------------ |
| `func foo(x: Int64)`     | `foo(42)`                |
| `func foo(label x: Int64)` | `foo(label: 42)`      |
| `init(x: Int64)`         | `Type(42)`               |
| `init(label x: Int64)`   | `Type(label: 42)`        |
| Memberwise (no init)     | `Type(fieldName: value)` |
| `case Foo(label: Type)`  | `.Foo(label: value)`     |
| `case Foo(Type)`         | `.Foo(value)`            |

## Gotchas

- Single-name params have **no external label** — `foo(42)` not `foo(x: 42)`. Unlike Swift.
- Memberwise inits **do** require labels matching field names.
- `_` label syntax (`func foo(_ x: Int64)`) is NOT supported — use single-name param.
- Outside stdlib: do NOT `import std.*` — public stdlib types are auto-imported.
- `as`, `get`, `set`, `protocol` are keywords — can't be param names/labels.
- Subscripts use `dict(key)` not `dict[key]` — brackets are for type parameters only.
- `let _ = expr;` needs a semicolon when it's the only statement in a void body.
- Structs with `String`/`Array`/`Dictionary` fields need explicit `Cloneable` conformance.
- Multi-line method chaining (`.foo()\n.bar()`) doesn't parse — use intermediate variables.
- Closures that capture variables can't be returned from functions.
- `F.Type` metatype syntax is not yet supported.
- `!` is the Never type, not `Never`.
