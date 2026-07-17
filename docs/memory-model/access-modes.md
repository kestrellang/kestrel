# Access Modes

Every parameter in Kestrel has one of three access modes that determine ownership transfer and mutability.

## Overview

| Mode | Keyword | Meaning | Ownership | Original Variable |
|------|---------|---------|-----------|-------------------|
| **Borrow** | (default) | Read-only access | Caller retains | Valid |
| **Mutating** | `mutating` | Read-write access | Caller retains | Valid |
| **Consuming** | `consuming` | Takes ownership | Caller loses* | Moved or Copied |

*If the type is Copyable, the caller passes a copy (Cloneable: a `clone()`); the original stays valid. If `not Copyable`, it is a true move and the original is invalidated.

Access modes are spelled **only** on the declaration. Call sites are unannotated — `reset(p)` looks the same whichever mode `reset` declares, and writing `&p` at a call site is rejected (E488).

For parameters, the mode prefixes the (optionally labeled) parameter name. For methods, the receiver's mode prefixes `func`:

```kestrel
func offset(mutating point p: Point, by delta: Int64) { }   // parameter mode
mutating func increment() { }                                // receiver mode
consuming func build() -> Product { }                        // receiver mode
```

## Borrow (Default)

Parameters are borrowed by default, providing read-only access:

```kestrel
func printPoint(p: Point) {
    print(p.x);
    print(p.y);
}

let p = Point(x: 1, y: 2);
printPoint(p);
print(p.x);  // OK: p is still valid
```

The callee cannot modify the borrowed value, and the caller retains full ownership. Borrowing never copies or clones, regardless of the type's copy class — it is always free to pass a `String` or a non-Copyable resource by borrow.

## Mutating

Use `mutating` for write access. The caller must pass a mutable **place** — a `var` binding, or a mutable projection of one:

```kestrel
func reset(mutating p: Point) {
    p.x = 0;
    p.y = 0;
}

var p = Point(x: 1, y: 2);
reset(p);
print(p.x);  // Prints 0
```

Passing a `let` binding, a temporary, or a call result to a `mutating` parameter is an error. Field projections must be mutable all the way down: `outer.inner.leaf` can be passed as `mutating` only if every step is a `var`.

### Mutating Methods

A method that mutates the receiver is declared with the `mutating` prefix (there is no explicit `self` parameter in Kestrel):

```kestrel
struct Counter {
    var value: Int64

    mutating func increment() {
        self.value = self.value + 1;
    }
}

var c = Counter(value: 0);
c.increment();          // OK: c is a var
let frozen = Counter(value: 0);
frozen.increment();     // ERROR: receiver must be mutable
```

A non-`mutating` method borrows `self` and cannot pass its own fields onward as `mutating`.

## Consuming

Use `consuming` to take ownership of a value:

```kestrel
func consume(consuming p: Point) {
    print(p.x);
}  // p is dropped here

let p = Point(x: 1, y: 2);
consume(p);
// If Point is Copyable: p is still valid (a copy was passed)
// If Point is not Copyable: p is now invalid (moved) — later use is E500
```

Inside the callee, a consuming parameter is owned and mutable: it may be reassigned, destructured, moved onward, or simply dropped at the end of the body.

### Consuming Methods

A method that consumes its receiver is declared with the `consuming` prefix:

```kestrel
struct Connection: not Copyable {
    var handle: Int64

    consuming func close() -> Int64 {
        self.handle   // legal: moving a field out of consumed self
    }
}
```

`consuming` methods may move non-Copyable fields out of `self` — the receiver is destructured, the extracted field is returned owned, and the sibling fields drop (see [copy-semantics.md](copy-semantics.md), "Field Move-Out"). This is the standard shape for builder `build()` methods and `into*` conversions.

`mutating` and `consuming` cannot be combined on one parameter or receiver.

## Interaction with Copy Classes

The mode says what the callee *needs*; the type's copy class says how the caller *provides* it:

| Mode | Copyable | Cloneable | not Copyable |
|------|----------|-----------|--------------|
| borrow | borrow | borrow (no clone) | borrow |
| mutating | in-place | in-place | in-place |
| consuming | bitwise copy | `clone()` then move the clone | move (original invalid) |

The same `consume(x)` call site therefore means "copy" for a `Point` and "move" for a `Connection`. This is intentional — application code shouldn't need to care, and the compiler does the right thing. When you *want* move semantics to be observable, make the type `not Copyable`.

## Interaction with Closures

Closures capture by value at creation time, so a capture never keeps a live borrow of the enclosing variable — creating a closure over `x` and later passing `x` as `mutating` or `consuming` do not conflict:

```kestrel
var x = 10;
let snapshot = { x };   // captures the VALUE 10
x = 20;                 // fine; snapshot() still returns 10
```

Closure *parameters* have access modes too, including `mutating`; the convention can be inferred from the context type (see [closures.md](closures.md)).

---

## Design Notes

- **`consuming` is invisible at the call site.** For large Copyable/Cloneable types this can hide an expensive copy. Detecting that is a linting concern; the semantics are deliberate.
- **`mutating` requires a mutable place.** The error points at the call site; if the binding is far away, follow the binding to its `let` and change it to `var` (or restructure to return a new value).
- **No `mutating Self` chaining.** Mutating methods conventionally return `()`; fluent chaining of mutations is not supported. Follow the stdlib idiom: mutating verb + non-mutating past participle (`sort()` / `sorted()`).
