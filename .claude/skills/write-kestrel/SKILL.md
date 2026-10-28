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

### Writing `clone()`

Inside a `clone()` body, **`self` is a bitwise copy**. The compiler deliberately
suppresses clone-insertion there (otherwise `clone()` would call itself forever),
so returning `self` — or a payload bound out of `self` — **aliases** any heap field
(`String`, `Array`, `Dictionary`, `Rc`, …) instead of duplicating it. Both the
original and the "copy" then free the same buffer → double-free / use-after-free.
Deep-clone every heap payload explicitly:

```kestrel
// WRONG — `{ self }` bit-copies, aliasing the String; double-free on drop
enum Token: Cloneable {
    case Plain
    case Literal(String)
    func clone() -> Token { self }
}

// RIGHT — clone each heap payload; bare `self` is fine only for payload-less cases
enum Token: Cloneable {
    case Plain
    case Literal(String)
    func clone() -> Token {
        match self { .Literal(s) => .Literal(s.clone()), _ => self }
    }
}
```

Same rule for structs: `func clone() -> Box { self }` aliases the fields — instead
build a fresh value and `.clone()` each heap field. (Types that *don't* hand-write
`clone()` get correct compiler-synthesized cloning; the footgun is specific to a
hand-written body that returns `self`.)

## Enums

```kestrel
enum Direction { case North case South case East case West }  // inline cases: NO `;` separator
let d: Direction = .North;

enum Shape {
    case Circle(radius: Float64)              // labeled
    case Rectangle(width: Float64, height: Float64)
    case Point
}
let s = Shape.Circle(radius: 5.0);

enum Option[T] { case Some(T) case None }    // positional (cases separated by whitespace, not `;`)
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

### Closure Kinds

A function type may carry a **kind** prefix. The kind decides how the closure holds its
captures, how it can be called, and whether it can leave the frame. Kinds are spelled
**on types only — never on a literal**; a literal is built for whatever kind its expected
type asks for.

```kestrel
(Int64) -> Int64             // normal (default) — read-only VIEWS of the frame
mutating (Int64) -> ()       // &mutating views — assignments write back to the caller
consuming () -> File         // owns its captures; called exactly once; may move them out
escaping () -> Int64         // owns snapshots in a shared heap env; may outlive the frame
```

| kind | captures | callable | copy class | can leave frame |
|---|---|---|---|---|
| normal | views (see later writes) | many times, from `let` | Copyable | no (E494) |
| `mutating` | `&mutating` views | many times, needs `var` | `not Copyable` | no |
| `consuming` | owned; body may move them **out** | exactly once | `not Copyable` | yes |
| `escaping` | owned snapshots, **shared** | many times, from `let` | Cloneable (clone = retain) | yes |

Which to reach for: **normal** for predicates/transforms/visitors — the default and ~86% of
the stdlib; **`mutating`** when the callback writes back into caller variables (`forEach`);
**`escaping`** when the callee stores or returns the callback (lazy adapters, struct fields);
**`consuming`** for a one-shot hand-off that transfers a resource.

A kind goes anywhere a function type goes — params, return types, `let` annotations, struct
fields, protocol requirements, type aliases. `escaping` is a contextual keyword (still usable
as an identifier); `mutating`/`consuming` are reserved.

```kestrel
func each(mutating action: mutating (Int64) -> ()) { }        // kind + matching access mode
func store(consuming f: escaping () -> Int64) -> Holder { }
func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { count = count + 1; count }                               // literal built as escaping
}
struct Button { let onClick: escaping () -> Int64 }            // storable long-term
let b2 = b; (b2.onClick)();                                    // shares one environment
```

### Capture Semantics per Kind

Normal/`mutating` capture **views** — later writes are visible. `escaping`/`consuming`
capture **owned snapshots** at creation (bit-copy Copyable, `clone()` Cloneable, **move**
non-Copyable → source dead, E500 on later use).

```kestrel
var x = 10;
let f = { x };                            // view
x = 20;
f()                                        // 20  — NOT a snapshot

var y = 10;
let g: escaping () -> Int64 = { y };      // snapshot of 10
y = 20;
g()                                        // 10
```

`escaping` closures have **reference semantics**: `clone`/copy retains one shared heap
environment, so aliases see each other's mutations, and the captures drop once at last
release. Refcounting does not collect **strong cycles** — a cycle of escaping environments
leaks and its captures are never deinitialized.

### Passing Rules (the ones that bite)

| have ↓ \ expected → | normal | `mutating` | `consuming` | `escaping` |
|---|---|---|---|---|
| normal | ✓ | ✓ (adapter) | ✗ E624 | ✗ E624 |
| `mutating` | ✗ | ✓ | ✗ | ✗ |
| `consuming` | ✗ | ✗ | ✓ | ✗ |
| `escaping` | ✓ (view) | ✗ E624 | ✓ | ✓ |

- A closure **literal** adapts automatically — the expected type retrofits the kind, trailing
  closures included. An already-bound closure **value** does not: `let scale = { x * f };
  iter.map(as: scale)` is **E624**. Fix by annotating the expected kind where the closure is
  created: `let scale: escaping (Int64) -> Int64 = { x * f };` (it is then built with owned
  captures) — never by re-annotating the existing value.
- A `mutating`-kind value must be held in `var` to be called; calling one held in `let` is the
  **E203** family, not E624.
- Kind and access mode must pair: `mutating` kind on a `mutating` param, `consuming` kind on a
  `consuming` param, else **E625**. The kind is independent of a *param's own* convention
  inside the type — `(mutating T) -> R` is still a normal closure.
- Named/capture-free functions satisfy every kind.

### The Freeze Rule (view kinds)

While a live normal/`mutating` closure views a place, that place is frozen against
**destruction** — it cannot be moved, passed to a `consuming` param, or `deinit`ed (**E507**).
Plain reassignment stays legal. A view-carrying value also cannot be stored into a
longer-lived binding than its captures (E507 "cannot outlive").

```kestrel
let r = Res(v: 3);
let f = { r.v };        // views r
sink(r);                // ERROR E507 — consuming a frozen place
```

```kestrel
var g: () -> lang.i64 = { 0 };
if cond {
    let r = Res(v: 9);
    g = { r.v };        // ERROR E507 — the view would outlive `r`
}
```

Assigning to a capture in a **normal** body is **E603** (including through a field,
`c.n = 5`) — the fix is a `mutating` expected type. Moving a capture out is E506 except in a
`consuming` body. Returning/storing a view closure is **E494**; the fix-it is an owning kind
in the expected type.

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
(Int64, Int64) -> Int64     // function type (normal kind)
escaping () -> Int64        // kinded function type — see Closure Kinds
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
- Enum cases are separated by **whitespace/newlines, not `;`** — `enum D { case A case B }` parses, `enum D { case A; case B }` is a parse error. (Statement semicolons inside bodies are still required.)
- Protocol requirements take **no trailing `;`** — `func matches(other: Self) -> Bool`, not `... -> Bool;`.
- `let _ = expr;` needs a semicolon when it's the only statement in a void body.
- Structs with `String`/`Array`/`Dictionary` fields need explicit `Cloneable` conformance.
- Inside a hand-written `clone()`, `self` is a **bitwise copy** — deep-clone heap fields explicitly; `clone() { self }` aliases them → double-free. (See *Writing `clone()`*.)
- Multi-line method chaining (`.foo()\n.bar()`) **parses fine** (verified 2026-07 by compiling+running). The real chaining constraint: **a trailing closure only binds to an unlabeled closure param** — `iter().filter { it > 0 }` fails with "wrong label: expected 'where', got '_'"; write `filter(where: { it > 0 })`. `map` takes its closure positionally so `xs.map { it * 2 }` works, but `filter` is labeled on both Array and Iterator (`filter(where: { it % 2 == 0 })`), as are most adapters (`where:`, `as:`, `by:`) — when a trailing closure fails with a label error, spell the label.
- `it` **works inside string-interpolation holes** — `map { "x \(it)" }` is fine (fixed 2026-07-17; older compilers errored "undefined name 'it'").
- Normal/`mutating` closures that **capture** can't escape the defining function (E494) — return or store them by writing `escaping`/`consuming` in the expected type. Capture-free closures escape freely.
- Normal closures capture **views**: they see writes made after creation. For a snapshot, use an `escaping`/`consuming` expected type.
- Kinds live **on function types only** — there is no `{ mutating () in … }` literal syntax, and no capture lists.
- Stdlib kinds: `Iterator.forEach`/`tryForEach`, `Optional.inspect`, `Result.inspect`/`inspectErr` take `mutating` closures; the lazy builders (`Iterator.map`/`filter`/`filterMap`/`flatMap`/`scan`/`takeWhile`/`skipWhile`/`inspect`/`intersperseWith`, `ArraySlice.split(where:)`, `Str.split(where:)`) and the adapter/view inits take `escaping`. Everything else is normal.
- Calling a closure held in a **field** from outside the type needs parens — `(h.action)()`; inside the type `self.predicate(x)` works directly.
- `F.Type` metatype syntax is not yet supported.
- `!` is the Never type, not `Never`.
