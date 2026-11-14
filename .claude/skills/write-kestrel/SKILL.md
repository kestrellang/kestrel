---
name: write-kestrel
description: Quick reference for writing Kestrel source code — program skeleton, syntax, labels and access modes, references and slices, closures, protocols and generics, and where things live in the stdlib. Use when writing or editing `.ks` files, when the user asks "how do I write X in Kestrel?", when drafting stdlib or package code, or any time the task produces Kestrel code rather than compiler Rust. For compiler internals (Rust in `lib/`) use `kestrel-pipeline` / `debug`; for test files use `write-tests`.
---

# Writing Kestrel

Statically typed, copy-by-default value semantics, explicit access modes
(borrowing / `mutating` / `consuming`), protocol-based generics (monomorphized),
second-class references, RAII. Statements end in `;`.

This file is the core. Go deeper in the side files:

| Need | Read |
|---|---|
| `&T`, `&Slice[T]`, `&StringSlice`, views, `Indirection`, decay | [references.md](references.md) |
| Closure kinds (`mutating` / `consuming` / `escaping`), captures, E624/E507 | [closures.md](closures.md) |
| Primitives, sugar, strings, existentials, `not Copyable` / `not Static` / `not Sized`, extensions, derive | [types.md](types.md) |
| Which std module has what, common call shapes (files, processes, time, collections) | [stdlib.md](stdlib.md) |
| Exact signatures of any std item | the std source the compiler loads (below), or https://kestrel-lang.com/reference/stdlib |
| Full language reference | https://kestrel-lang.com/docs |

**Check the std before you call it.** It is renamed between releases, so don't
trust a remembered spelling. `kestrel build --verbose file.ks` prints
`Loading stdlib from <dir>`; grep that directory (`rg 'func readText' <dir>`).
It is normally `<toolchain>/lib/std` next to the `kestrel` binary, or
`$KESTREL_STD` when set.

## Program skeleton

```kestrel
module Hello

@main
func main() -> Int64 {             // exit code; `()`, `!`, `ExitCode` and `T throws E` also work
    let name = "world";
    println("hello, \(name)");
    0
}
```

- **Every file that declares anything starts with a `module` line** (E414).
  Without it, its names leak into other modules and the std stops compiling.
- `@main` marks the entry point; the name `main` alone is not special.
- Every public std declaration is auto-imported, including `File`, `Process`
  and `Duration`. Write `import std.x.(A, B)` only to be explicit, and
  `import M as Alias` / `import M.(A as B)` to rename.
- Build and run: `kestrel build file.ks -o out && ./out`. `-O 2` builds with LLVM.

## The rules that bite first

1. **Single-name params are positional.** `func add(x: Int64, y: Int64)` is
   called `add(1, 2)`; `add(x: 1, y: 2)` is E113 "'x' is the parameter's name,
   not an argument label". Add an external label to require one:
   `func move(to target: Point)` → `move(to: p)`. There is no `_` label.
   This applies to std too: `File.readText(path)`, `File.writeText(path, text)`,
   `Environment.variable("HOME")`.
2. **Memberwise inits do take labels**: `Point(x: 1, y: 2)`.
3. **Subscripts and calls both use parens**: `xs(0)`, `dict("k")`, `grid(row: 1, col: 2)`.
   Square brackets are only for type arguments: `Array[Int64]`, `largest[T]`.
4. **Logical operators are words**: `and`, `or`, `not`. `!x` is bitwise not;
   `!` as a type is Never.
5. **Optional patterns are `some x` and `null`**, not `.Some(x)` / `.None`.
   `null` is also the empty value: `let n: String? = null;`.
6. **Enum cases are separated by whitespace, not `;`**: `enum D { case A case B }`.
7. **Protocol requirements have no trailing `;`.**
8. **Methods don't declare `self`.** `mutating func` / `consuming func` set
   the receiver's mode; `static func` has none.
9. **A mutating call needs a `var`** (a `let` receiver or argument is E203).
10. **A bare protocol is not a type**: write `any P` (boxed) or `some P` (opaque) — E144.

## Variables and functions

```kestrel
let x: Int64 = 42;             // immutable
var count = 0;                 // mutable, inferred Int64
count += 1;

func add(x: Int64, y: Int64) -> Int64 { x + y }        // last expression is the result
func double(x: Int64) -> Int64 = x * 2                  // expression body
func send(to recipient: String, retries: Int64 = 3) { } // label + default
func reset(mutating p: Point) { p.x = 0; }              // caller passes a var
func close(consuming f: File) { }                       // takes ownership
func identity[T](value: T) -> T = value
func maxOf[T](a: T, b: T) -> T where T: Comparable { if a > b { a } else { b } }
```

Access mode order in a parameter: `mode label name: Type`
(`func offset(mutating point p: Point, by delta: Int64)`).

## Structs and enums

```kestrel
module Shapes

struct Point: Equatable {
    var x: Int64
    var y: Int64

    func sum() -> Int64 { self.x + self.y }
    mutating func offset(by delta: Int64) { self.x += delta; }
    static func origin() -> Point { Point(x: 0, y: 0) }
    func isEqual(to other: Point) -> Bool { self.x == other.x and self.y == other.y }
}

enum Shape {
    case Circle(radius: Float64)
    case Square(side: Float64)

    var area: Float64 {
        match self {
            .Circle(radius: r) => 3.14159 * r * r,
            .Square(side: s) => s * s
        }
    }
}

indirect enum Tree {                     // recursive payloads are boxed
    case Leaf(Int64)
    case Node(left: Tree, right: Tree)
}

@main
func main() -> Int64 {
    var p = Point(x: 1, y: 2);
    p.offset(by: 3);
    println(p.sum());
    println(Shape.Square(side: 2.0).area);
    0
}
```

- No custom `init` → a memberwise init labeled by field names. A custom
  `init(x: Int64, y: Int64)` is positional; `init(atX x: Int64)` is labeled.
- Structs copy by default. `struct File: not Copyable { deinit { … } }` makes
  a move-only RAII type; using it after a move is E500.
- `@derive(Equatable, Hashable, Cloneable, Comparable)` writes the boilerplate
  (`Equatable`, `Hashable`, `Cloneable`, `Comparable`, `Defaultable`, `Formattable`). Structs with `String`/`Array` fields are
  `Cloneable`, not `Copyable`.
- **A hand-written `clone()` must clone each heap field.** Inside `clone()`,
  `self` is a bitwise copy, so `func clone() -> T { self }` aliases the buffers
  and double-frees. Prefer `@derive(Cloneable)`.

## Pattern matching and control flow

```kestrel
match value {                             // arms from several types, for illustration
    0 => "zero",
    1..<10 => "small",
    .Circle(radius: r) if r > 10.0 => "big circle",
    some x => "present",
    null => "absent",
    _ => "other"
}

if let some v = dict("key") { }
guard let some first = xs.first else { return 0; }
guard let .Ok(text) = File.readText(path) else { return 1; }
while let some line = lines.next() { }

for x in xs { }
for i in 0..<n { }
outer: for row in grid { for cell in row { if cell == 0 { break outer; } } }
loop { if done { break; } }
```

`match` arms are separated by `,`; a `match` used as a statement ends in `;`.
An `is` match (`c is Circle => …`) is never exhaustive, so add `_`.

## Collections

```kestrel
let xs = [3, 1, 2];                       // Array[Int64]
let ages: [String: Int64] = ["ana": 31];  // Dictionary
let first = xs(0);                        // panics out of bounds
let maybe = xs(checked: 5);               // Optional
let middle = xs(1..<3);                   // &Slice[Int64], no copy
let head = xs.first ?? 0;                 // first / last / isEmpty / count are properties
let evens = xs.filter(where: { it % 2 == 0 });
let doubled = xs.map { it * 2 };          // Slice.map takes its closure positionally
let total = xs.iter().fold(from: 0, by: { acc, x in acc + x });
let big = xs.iter().find(where: { it > 1 });
let sorted = xs.sorted(by: { a, b in a < b });
var names: Array[String] = [];
names.append("ana");
var d: [String: Int64] = [:];
d("bob") = 40;                            // insert or overwrite
```

Lazy iterator adapters label their closure: `iter().map(as:)`,
`iter().filter(where:)`. `Slice` methods (`map`, `filter`, `sorted`, …) are
eager and return `Array`. If a trailing closure fails with "wrong label",
spell the label.

## Errors

`T throws E` is `Result[T, E]`. `throw` returns the error; a bare success
value is promoted to `.Ok` in return and tail position.

```kestrel
module Parse

enum ParseError { case Empty case Bad(String) }

func parse(text: &StringSlice) -> Int64 throws ParseError {
    if text.isEmpty { throw ParseError.Empty; }
    guard let some n = Int64(parsing: text) else { throw .Bad(text.toOwned()); }
    n
}

func twice(text: &StringSlice) -> Int64 throws ParseError {
    let n = try parse(text);              // propagate the error
    n * 2
}

@main
func main() -> Int64 {
    match twice("21") {
        .Ok(v) => println(v),
        .Err(.Empty) => println("empty"),
        .Err(_) => println("bad")
    };
    0
}
```

`try` covers the whole postfix chain after it (`try a.b().c()`) and also works
on `Optional`. To fall back instead of propagating, drop the `try`:
`mayFail(n) ?? 0` (a `Result` coalesces like an `Optional`).

## Protocols, generics, extensions (short)

```kestrel
protocol Describe { func describe() -> String }
extend Point: Describe { func describe() -> String { "(\(self.x), \(self.y))" } }
extend Describe { func shout() -> String { self.describe().uppercased() } }   // default method

func show[T](value: T) -> String where T: Describe { value.describe() }
func showAny(value: any Describe) -> String { value.describe() }              // boxed
extend Box[T] where T: Equatable { }                                          // conditional
extend[T] Int64: Reads[T] { }       // params the target doesn't bind are declared on `extend`
```

Generic parameters are implicitly `Copyable`, `Static` and `Sized`; relax
with `where T: not Copyable` etc. Details and existential boxes are in
[types.md](types.md).

## Properties and subscripts

```kestrel
struct Rect {
    var w: Float64
    var h: Float64
    var area: Float64 { self.w * self.h }              // computed, getter shorthand
    lazy var label: String = "rect"                    // computed once, on first read
    var scale: Float64 = 1.0 {
        didSet { println("scale is now \(self.scale)"); }
    }
    subscript(index: Int64) -> Float64 {
        get { if index == 0 { self.w } else { self.h } }
        set { if index == 0 { self.w = newValue; } else { self.h = newValue; } }
    }
}
```

Failable members (`var first?: T`, `subscript(checked i: Int64)? -> T`) return
`Optional`. `ref` / `mutating ref` accessors lend in place
(see [references.md](references.md)).

## Style

The std's naming rules; match them so your APIs read like the std:

- `PascalCase` types, `camelCase` everything else, no abbreviations in public names.
- Mutating verb vs. non-mutating participle: `sort()` / `sorted()`, `trim` / `trimmed()`.
- Properties for state (`count`, `isEmpty`, `first`), methods for work (`iter()`, `collect()`).
- Labels are prepositions: `where:`, `by:`, `from:`, `of:`, `at:`, `as:`, `byKey:`.
- `to*` converts and allocates (`toOwned()`, `toArray()`); `as*` views (`asSlice()`).
- Prefer `guard` and early returns over nesting; `for` over `while`.

## Gotchas

- `get`, `set`, `as`, `protocol` are keywords; they can't be parameter names.
- `let _ = expr;` needs its `;` even as the only statement in a body.
- A closure stored in a field is called with parens outside the type: `(h.action)()`.
- Normal closures that capture can't be returned or stored (E494). Write an
  `escaping` (or `consuming`) function type where the closure is expected.
- A reference to a slice can't cross an `if`/`match` merge (E497). Bind it
  before the branch, or make each arm produce an owned value.
- `F.Type` metatypes and `async` don't exist yet.
- String length: `s.bytes.count` (O(1)) or `s.chars.count` (O(n)). There's no `s.count`.

## Maintaining this skill

Every ```` ```kestrel ```` block in these files that starts with a `module`
line is a whole program and must compile and run on the current compiler.
Fragments (no `module` line) are illustrations. When a std API or rule
changes, fix it here and in the matching side file. The skill is used
outside the Kestrel repo, so never point at repo paths: link the website or
tell the reader how to find the installed std, and don't copy long API lists.
