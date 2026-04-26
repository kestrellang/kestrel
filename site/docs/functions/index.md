# Functions

Functions are Kestrel's basic unit of behavior. They take parameters (positional or labeled), return a value (or none), and can be passed around as values themselves.

## Definitions

A function is declared with `func`, a name, parameters, and a return type:

```swift
func add(x: Int, y: Int) -> Int {
    x + y
}
```

The body is a block of statements. The last expression in the block is the return value — no `return` keyword needed unless you're returning early. For one-liners, you can use the expression-bodied form:

```swift
func double(x: Int) -> Int = x * 2
```

A function that returns nothing has no return type:

```swift
func greet(name: String) {
    println("Hello, \(name)!")
}
```

## Parameters

A parameter has a **bind name** (used inside the body) and an optional **label** (used at the call site). The two forms are:

```swift
//        bind name
func add(x: Int, y: Int) -> Int { x + y }   // positional — no labels

//        label  bind name
func send(to recipient: String, body content: String) { /* ... */ }   // labeled
```

A bare `name: Type` declares a positional parameter — there's no label, and the call site doesn't write one:

```swift
add(3, 4)
```

A `label name: Type` declares a labeled parameter. The call site uses the label:

```swift
send(to: "alice@example.com", body: "hello")
```

That's the rule: one name = positional, two names = label then bind name. There's no "label and name are the same" shorthand — if you want both to be `to`, you write `to to: String`.

## Overloading

Functions can be overloaded by their labels (or by parameter types when positional):

```swift
func find(by name: String) -> Optional[User] { /* ... */ }
func find(by id: Int) -> Optional[User] { /* ... */ }

find(by: "alice")  // calls the String version
find(by: 42)       // calls the Int version
```

## Return Types

The arrow `->` separates the parameter list from the return type:

```swift
func square(x: Int) -> Int = x * x
func nameOf(user: User) -> String { user.name }
```

A function that never returns — because it loops forever, calls a panic, or always throws — has the **never type**, written `!`:

```swift
func crash(message: String) -> ! {
    panic(message)
}
```

`!` is a *bottom* type. It's compatible with every other type, which is why this is allowed:

```swift
let port: Int = if config.has(key: "port") {
    config.get(key: "port")
} else {
    crash(message: "port required")   // never returns, so the if's type is just Int
}
```

## Choosing labels

Labels are how Kestrel call sites stay readable. Pick labels that make the call read naturally:

```swift
// Good — reads like English
func move(from start: Point, to end: Point) { /* ... */ }
move(from: home, to: office)

// Less good — call site is just type-shaped, no clarity gained
func move(start: Point, end: Point) { /* ... */ }
move(home, office)   // positional; have to remember which is which
```

When the parameter is the obvious main argument and a label adds nothing, leave it positional:

```swift
func describe(user: User) -> String { /* ... */ }
describe(alice)
```

A heuristic: if your call site reads like prose (`send(to: alice, body: ...)`), labels are pulling weight. If labels just repeat the type name, positional is cleaner.

## Subpages

- [Access Modes](access-modes.md) — `mutating` and `consuming` parameters
- [Methods](methods.md) — functions defined on a type
- [Closures](closures.md) — anonymous functions and capture
- [Operator Overloading](operator-overloading.md) — defining `+`, `==`, etc. on your own types

---

[← Values & Variables](../values-and-variables.md) · [↑ The Kestrel Language](../index.md) · [Access Modes →](access-modes.md)
