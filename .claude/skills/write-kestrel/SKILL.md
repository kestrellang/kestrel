---
name: write-kestrel
description: Quick reference for writing Kestrel source code — syntax, semantics, idioms, and the label/access-mode rules that bite first-time writers. Use when writing or editing `.ks` files, when the user asks "how do I write X in Kestrel?", when drafting stdlib modules, or any time the task involves producing Kestrel code rather than modifying the compiler. Skip for compiler-internals work (Rust code in `lib2/`), which is covered by `kestrel-pipeline` / `debug-kestrel`.
---

# Kestrel Language Quick Reference

A concise guide to the Kestrel programming language.

For idiomatic patterns and preferred-form guidelines (type operators, `if let` / `guard let` / `while let`, range operators, integer-literal style), see the [Kestrel Style Guide](../../../docs/STYLE_GUIDE.md). This skill covers syntax and semantics; the style guide covers taste.

## Overview

Kestrel is a statically-typed language with:
- Copy-by-default value semantics (not move semantics like Rust)
- Explicit parameter access modes (borrowing, mutating, consuming)
- Protocol-based polymorphism
- Monomorphized generics
- RAII resource management

## Variables and Constants

```kestrel
let x: Int = 42;        // Immutable binding
let message = "Hello";  // Type inferred
var count: Int = 0;     // Mutable variable
count = count + 1;
```

## Functions

Functions use `func` keyword. Methods do NOT declare `self` as a parameter - it's implicit.

```kestrel
// Basic function
func add(x: Int, y: Int) -> Int {
    x + y  // Implicit return
}

// Expression-bodied function (shorthand for single expressions)
func add(x: Int, y: Int) -> Int = x + y

// Generic function
func identity[T](value: T) -> T = value

// With constraints
func compare[T](a: T, b: T) -> Bool where T: Comparable { }
```

## Parameter Labels

Kestrel has a specific system for parameter labels that determines how functions are called.

### Without Labels (Positional Calling)

When parameters have only a name and type, they are called positionally:

```kestrel
func add(x: Int, y: Int) -> Int { x + y }

// Called POSITIONALLY (no labels):
add(1, 2)
```

### With Labels (Labeled Calling)

To require labels at the call site, provide an external label before the internal name:

```kestrel
// Syntax: externalLabel internalName: Type
func send(to recipient: String) { }
func send(from sender: String) { }

// Called WITH labels:
send(to: "alice@example.com")
send(from: "bob@example.com")
```

This enables function overloading by label:
```kestrel
func move(to destination: Point) { }
func move(by offset: Point) { }

// Different functions, distinguished by label:
move(to: target)
move(by: delta)
```

### Labels with Access Modes

Order: `accessMode externalLabel internalName: Type`

```kestrel
func offset(mutating point p: Point, by delta: Int) {
    p.x = p.x + delta;
}

// Called as:
offset(point: myPoint, by: 5)
```

## Parameter Access Modes

```kestrel
// Default: borrowing (read-only)
func read(p: Point) -> Int { p.x }

// Mutating: can modify the argument (caller must pass var)
func reset(mutating p: Point) {
    p.x = 0;
    p.y = 0;
}

// Consuming: takes ownership
func consume(consuming f: File) { }
```

## Structs

```kestrel
struct Point {
    var x: Int;
    var y: Int;
}
```

### Initializers

**Memberwise Initializer (Automatic)**: When no custom `init` is defined, field names become labels:

```kestrel
struct Point {
    var x: Int;
    var y: Int;
}

// Called with field names as labels:
let p = Point(x: 10, y: 20);
```

**Custom Initializer Without Labels**: Positional calling:

```kestrel
struct Point {
    var x: Int;
    var y: Int;

    init(x: Int, y: Int) {
        self.x = x;
        self.y = y;
    }
}

// Called POSITIONALLY:
let p = Point(1, 2);
```

**Custom Initializer With Labels**: Labeled calling:

```kestrel
struct Point {
    var x: Int;
    var y: Int;

    init(atX x: Int, atY y: Int) {
        self.x = x;
        self.y = y;
    }
}

// Called WITH labels:
let p = Point(atX: 5, atY: 10);
```

### Methods

Methods do NOT declare `self` as a parameter - it's implicit:

```kestrel
struct Point {
    var x: Int;
    var y: Int;

    // Instance method (self is implicit!)
    func sum() -> Int {
        self.x + self.y
    }

    // Mutating method
    mutating func offset(by: Int) {
        self.x = self.x + by;
    }

    // Static method
    static func origin() -> Point {
        Point(x: 0, y: 0)
    }

    // RAII cleanup
    deinit {
        // Cleanup code
    }
}
```

### Copy vs Move Semantics

```kestrel
// Default: copy semantics
let p1 = Point(x: 1, y: 2);
let p2 = p1;  // COPIED - both valid

// Opt-out for move semantics
struct File: not Copyable {
    var handle: Int;
}

let f1 = File(handle: 1);
let f2 = f1;  // MOVED - f1 invalid
```

## Enums

### Simple Enum

```kestrel
enum Direction {
    case North
    case South
    case East
    case West
}

let d: Direction = .North;  // Shorthand when type known
```

### With Labeled Associated Values

```kestrel
enum Shape {
    case Circle(radius: Float64)
    case Rectangle(width: Float64, height: Float64)
    case Point
}

// Must use labels:
let s = Shape.Circle(radius: 5.0);
let r = Shape.Rectangle(width: 10.0, height: 20.0);
```

### With Unlabeled Associated Values (Positional)

```kestrel
enum Option[T] {
    case Some(T)
    case None
}

// Positional:
let opt = Option.Some(42);
```

### Recursive Enum

```kestrel
indirect enum Tree[T] {
    case Leaf(value: T)
    case Node(left: Tree[T], right: Tree[T])
}
```

## Pattern Matching

```kestrel
match value {
    0 => "Zero",
    1..<10 => "Small",         // Range (exclusive end)
    1..=10 => "Small",         // Range (inclusive end)
    .Circle(radius: r) => r,   // Enum destructuring
    .Some(x) if x > 10 => x,   // Guard clause
    _ => "Other"               // Wildcard
}

// If-let
if let .Some(val) = optional {
    // use val
}

// While-let
while let .Some(item) = iterator.next() {
    process(item);
}
```

## Control Flow

```kestrel
// If-else
if x > 0 {
    // ...
} else if x < 0 {
    // ...
} else {
    // ...
}

// Guard
guard x > 0 else {
    return .Err(Error.Invalid);
}

// While
while condition {
    // ...
}

// Infinite loop
loop {
    if done { break; }
}

// Labeled loops
outer: loop {
    while true {
        break outer;
    }
}

// Note: for loops are NOT implemented yet
```

## Closures

```kestrel
// Basic syntax
let add = { (a: Int, b: Int) in a + b };

// Single parameter with `it`
let double: (Int) -> Int = { it * 2 };

// Trailing closure
numbers.map { it * 2 }

// Multi-statement
let compute = { (x: Int) in
    let doubled = x * 2;
    let result = doubled + 1;
    result
};
```

Closures capture by value (copied at creation time).

## Protocols

```kestrel
protocol Drawable {
    func draw()
    mutating func reset()
    static func default() -> Self
}

// With associated types
protocol Container {
    type Element
    func get() -> Element
}

// Generic protocol
protocol Comparable[T] {
    func compare(other: T) -> Int
}
```

### Conformance

```kestrel
// Direct conformance
struct Circle: Drawable {
    func draw() { }
    mutating func reset() { }
    static func default() -> Circle { Circle() }
}

// Via extension
extend Circle: Hashable {
    func hash() -> Int { 0 }
}

// Protocol extension (default implementation)
extend Drawable {
    func redraw() {
        self.draw()
    }
}
```

## Generics

```kestrel
// Generic struct
struct Box[T] {
    var value: T;
}

// Multiple parameters
struct Pair[A, B] {
    var first: A;
    var second: B;
}

// With constraints
struct SortedList[T] where T: Comparable {
    var items: [T];
}

// Generic + non-copyable
struct Container[T] where T: not Copyable {
    var value: T;
}
```

## Extensions

```kestrel
// Add methods
extend Point {
    func distance(other: Point) -> Float64 { }
}

// Add protocol conformance
extend Point: Hashable {
    func hash() -> Int { }
}

// Conditional extension
extend Box[T] where T: Equatable {
    func equals(other: Box[T]) -> Bool { }
}

// Specialized extension
extend Box[Int] {
    func doubled() -> Int { self.value * 2 }
}
```

## Modules and Imports

```kestrel
module MyApp.Utils

import std.collections.Array
import std.io as IO
import Library.(Item1, Item2)
import ModuleA.(Widget as WidgetA)

public import internal.types.Core  // Re-export
```

Visibility: `public`, `internal` (default), `private`

## Types

### Primitives
- Integers: `lang.i8`, `lang.i16`, `lang.i32`, `lang.i64` (default)
- Floats: `lang.f16`, `lang.f32`, `lang.f64` (default)
- Boolean: `lang.i1` (or `Bool`)
- String: `lang.str`
- Unit: `()` (void-like)
- Never: `!` (bottom type, never returns)

### Composite Types
```kestrel
(Int, String)           // Tuple
[Int]                   // Array
(Int, Int) -> Int       // Function type
lang.ptr[Int]           // Pointer (unsafe)
```

### Type Aliases
```kestrel
type ID = String;
type Handler = (Int) -> Bool;
type Pair[T] = (T, T);
```

## Computed Properties

```kestrel
struct Rectangle {
    var width: Float64;
    var height: Float64;

    // Getter only (shorthand)
    var area: Float64 { self.width * self.height }

    // Getter and setter
    var diagonal: Float64 {
        get { sqrt(self.width * self.width + self.height * self.height) }
        set { /* newValue is implicit */ }
    }

    // Static computed
    static var zero: Rectangle { Rectangle(width: 0.0, height: 0.0) }
}
```

## Subscripts

```kestrel
struct Grid {
    subscript(row r: Int, col c: Int) -> Int {
        get { self.data[r * width + c] }
        set { self.data[r * width + c] = newValue }
    }
}

let val = grid(row: 0, col: 1);
grid(row: 0, col: 1) = 42;
```

## Error Handling

```kestrel
// Result type
func read() -> Result[String, FileError] {
    if success {
        .Ok(data)
    } else {
        .Err(.NotFound)
    }
}

// Try operator (propagates errors)
func process() -> Result[Data, Error] {
    let content = try readFile();
    .Ok(parse(content))
}
```

## Key Differences from Other Languages

### vs Rust
- Copy by default (not move)
- Methods don't declare `self` as parameter
- `extend` instead of `impl`
- `mutating` instead of `&mut self`
- Generic constraints use `where T: Protocol`
- No lifetimes

### vs Swift
- Semicolons required for statements
- `extend` instead of `extension`
- Parameters without labels are positional (Swift uses `_` to omit)
- No optionals with `?` syntax (use `Option[T]`)

### vs TypeScript
- Static types only (no `any`)
- No classes - only structs
- Value semantics by default
- Explicit mutability

## Label Rules Summary

| Declaration | Call Site |
|-------------|-----------|
| `func foo(x: Int)` | `foo(42)` |
| `func foo(label x: Int)` | `foo(label: 42)` |
| `init(x: Int)` | `Type(42)` |
| `init(label x: Int)` | `Type(label: 42)` |
| Memberwise (no init) | `Type(fieldName: value)` |
| `case Foo(label: Type)` | `.Foo(label: value)` |
| `case Foo(Type)` | `.Foo(value)` |

## Common Patterns

```kestrel
// Factory method
struct Config {
    static func default() -> Config { }
}

// Builder pattern
struct Builder {
    mutating func set(value: Int) { }
    consuming func build() -> Product { }
}

// RAII resource management
struct Connection: not Copyable {
    deinit {
        self.close();
    }
}
```

## Style Conventions

- **Visibility**: `public` (cross-module), `internal` (default, module tree), `fileprivate` (file), `private` (type).
- **Mutability**: prefer `let`; use `var` only when the binding actually mutates.
- **Naming**: `PascalCase` for types/protocols/enums; `camelCase` for functions, methods, variables; `SCREAMING_SNAKE_CASE` for constants.
- **Self parameter**: `self` = borrowing (read-only, default); `mutating self` = can modify fields; `consuming self` = takes ownership.

## Gotchas (from project memory)

- Single-name params (`func foo(x: Int)`) have **no external label** — call as `foo(42)`, not `foo(x: 42)`. This is unlike Swift.
- Auto-generated struct memberwise inits **do** require labels matching field names.
- `_` label syntax (`func foo(_ x: Int)`) is NOT supported — use a single-name param.
- Outside stdlib: do NOT `import std.*` — all public stdlib types are auto-imported.
- `as`, `get`, `set`, `protocol` are keywords — can't be parameter names/labels.
- Subscripts: `dict(key)` not `dict[key]` — brackets are only for type parameters.
- `let _ = expr;` needs a semicolon when it's the only statement in a void body.
- Structs with `String`/`Array`/`Dictionary` fields need explicit `Cloneable` conformance with a `clone()` method.
- Multi-line method chaining (`.foo()\n.bar()`) doesn't parse — use intermediate variables.
- Closures that capture variables can't be returned from functions.
- `F.Type` metatype syntax is not yet supported.
- `for` loops are not implemented — use `while` / `loop` / `while let`.
- `Never` type is spelled `!`, not `Never`.
