# Closures and Capture

Closures in Kestrel are first-class values that capture their environment **by value, at creation time**. Capture interacts with the copy classes and with escape checking.

## Basic Closure Syntax

```kestrel
let add = { (a: Int64, b: Int64) in a + b };
let result = add(1, 2);                       // 3

let double: (Int64) -> Int64 = { it * 2 };    // single param → `it` shorthand
numbers.map { it * 2 }                         // trailing closure
```

Closures are ordinary values: they can be bound to `let`/`var`, passed as arguments, stored in struct fields, and returned from functions — subject to the escape rule below.

```kestrel
struct Callback {
    let action: () -> Int64
}

func constant() -> () -> Int64 {
    { 42 }                     // no captures: freely returnable
}

let cb = Callback(action: { 7 });
let n = (cb.action)();
```

## Capture Semantics

### Snapshots, Not Views

A capture copies (or clones, for Cloneable types) the value when the closure is **created**:

```kestrel
var x = 10;
let f = { x };   // snapshots x == 10
x = 20;
f();             // still 10
```

Because captures are snapshots, a closure never holds a live borrow of the enclosing variable — later `mutating`/`consuming` uses of `x` do not conflict with `f`. Captured values are immutable inside the body; assigning to a captured variable is an error:

```kestrel
var x = 10;
let bad = { x = 20; x };   // ERROR: cannot assign to captured variable
```

### Captures Are Place-Based

The compiler captures the narrowest *place* the body actually uses. A closure reading `self.cap` (a Copyable field of a non-Copyable receiver) captures just that `Int64`, not the whole `self` — so borrowing methods on `not Copyable` types can freely use closures over their own Copyable fields.

### Non-Copyable Captures Are Moves

If a closure captures a whole non-Copyable value, the value **moves into the closure's environment** (it cannot be copied). Two rules follow:

1. The original is moved — later use of it is **E500**:

```kestrel
let r = Res(id: 7);                // Res: not Copyable
let f = { () in r.peek() };        // r moves into f's environment
let x = r.peek();                  // ERROR(E500): use of moved value
```

2. The closure owns *one* value but may be called many times, so the body may **borrow** the capture freely but may not move it out — returning it or consuming it is **E506** (`move_captured_out_of_closure`):

```kestrel
let s = Res(id: 2);
let g = { () in consume(s) };      // ERROR(E506): cannot move captured value out
let h = { () in s };               // ERROR(E506)
```

## Parameter Conventions

Closure parameters carry access modes like function parameters. The convention may be written on the literal or **inferred from the expected type** — including a `let` binding's annotation:

```kestrel
struct Counter { var n: Int64 }

func bump(mutating c: Counter, with f: (mutating Counter) -> ()) { f(c); }

// literal omits `mutating`; inferred from the annotation
let f: (mutating Counter) -> () = { (x) in x.n = x.n + 10; };
var c = Counter(n: 0);
bump(c, with: f);      // c.n == 10
```

Conventions are checked contravariantly: a `mutating`-param closure cannot be passed where a plain (borrowing) closure is expected. A plain closure parameter stays immutable in the body.

## Escape Rule: Captures Must Outlive the Closure

A closure's environment is currently **stack-allocated in the frame that creates it**. The provenance escape checker (the same E494 machinery used for references — see [diagnostics.md](diagnostics.md)) roots the closure at the join of its captures' roots. A closure with **no captures** escapes freely; a closure that captures **frame-bound state cannot leave the frame**:

```kestrel
func makeAdder(n: Int64) -> (Int64) -> Int64 {
    { it + n }   // ERROR(E494): captures local `n`, which does not outlive the call
}
```

The check is provenance-based, not syntactic — laundering through a binding or a struct field is still caught:

```kestrel
func makeAdder2(n: Int64) -> (Int64) -> Int64 {
    let f = { (x: Int64) in x + n };
    f            // ERROR(E494) — root tracked through the binding
}

struct Holder { var f: () -> Int64 }

func make() -> Holder {
    let n: Int64 = 41;
    Holder(f: { () in n + 1 })   // ERROR(E494) — root tracked through the field
}
```

Heap-allocated environments (which would make capturing closures returnable) are planned future work; the by-value snapshot semantics are already fixed, so lifting the restriction will not change what captured values mean.

A closure also cannot capture a named **reference binding** (`let r = &x;`) — the environment would outlive the borrow (E212).

## `return` Inside a Closure

`return` in a closure body returns **from the closure**, not from the enclosing function.

## Loop Variables

Loop variables are captured by value like everything else — each iteration's closure snapshots that iteration's value.

---

## Design Notes

- **Exclusivity**: because captures are snapshots, two closures over the same `var` never alias it; there is no interleaved-mutable-capture hazard.
- **Nested closures** capture transitively: an inner closure using an outer function's local forces the outer closure to capture it too.
- **Recursive closures** (`let f = { ... f(...) ... }`) are not expressible — the binding is not in scope inside its own initializer. Use a named function.
