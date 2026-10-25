# Cloneable Protocol

Kestrel provides a unified copy model where types can customize how they are copied through the `Cloneable` protocol.

## Overview

| Type | Copy Behavior |
|------|---------------|
| Simple struct (all fields Copyable) | Implicit bitwise copy |
| Struct that is `Cloneable` | Implicit copy calls `clone()` |
| `not Copyable` struct | Cannot be copied, only moved |

## The `Cloneable` Protocol

As declared in the standard library (`lang/std/core/copy.ks`):

```kestrel
@builtin(.Cloneable)
public protocol Cloneable: Copyable {
    @builtin(.Clone)
    func clone() -> Self
}
```

`Cloneable` extends `Copyable`, so cloneable values flow through generic code that asks only for `Copyable` — but every implicit copy goes through `clone()` instead of a bitwise copy. The implementation decides how deep the copy goes: `String` duplicates its buffer, while a refcounted box only bumps the count.

## Basic Usage

### Default Copy (No Cloneable)

Simple types use compiler-generated bitwise copy:

```kestrel
struct Point {
    var x: Int64
    var y: Int64
}

let a = Point(x: 1, y: 2);
let b = a;   // Bitwise copy
print(a.x);  // 1 - a is still valid
print(b.x);  // 1 - b is an independent copy
```

### Custom Copy (Cloneable)

Types that manage resources conform to `Cloneable` and implement `clone()` (methods take no explicit `self` parameter):

```kestrel
struct MyBuffer: Cloneable {
    var data: Pointer[UInt8]
    var len: Int64

    func clone() -> MyBuffer {
        let fresh = allocate(self.len);
        copyBytes(from: self.data, to: fresh, count: self.len);
        MyBuffer(data: fresh, len: self.len)
    }

    deinit {
        free(self.data);
    }
}

let a = MyBuffer(...);
let b = a;   // Implicitly calls a.clone() — deep copy
// Both a and b own independent buffers
```

## When Does Clone Happen?

Borrowing is the default parameter mode, so cloning does **not** happen on ordinary function calls:

```kestrel
func process(data: BigData) { }  // Borrows — no clone!
process(myData);                 // No clone
```

`clone()` is invoked implicitly on:

- **Assignment**: `let b = a;`
- **`consuming` parameters**: the callee receives the clone; **the original stays valid**
- **Return of a still-live value** and storing into aggregates/collections
- **Binding decay of a reference** to a Cloneable pointee (`let s = box.peekString();` clones once)

```kestrel
func take(consuming d: Data) { }

let d = Data(value: 42);
take(d);   // passes d.clone()
take(d);   // OK — d is still valid; clones again
```

This mirrors Copyable behavior exactly: `consuming` only *moves* when the type is `not Copyable`. If you want single-owner hand-off semantics, mark the type `not Copyable` — do not rely on `consuming` to move a Cloneable value.

## Derived Cloneable Is Automatic

A struct or enum whose fields are Copyable **except for at least one Cloneable member** is classified Cloneable automatically (the member fold — see [copy-semantics.md](copy-semantics.md)), and the compiler synthesizes a memberwise clone: Copyable fields are bit-copied, Cloneable fields are `clone()`d. No declaration is required:

```kestrel
struct Document {
    var title: String   // Cloneable
    var pages: Int64    // Copyable
}
// Document is implicitly Cloneable; `let b = a;` clones `title`, copies `pages`.
```

Declare the conformance explicitly **only when you want to hand-write `clone()`** — and then you must implement it (a bare `struct Document: Cloneable { ... }` without a `clone()` body is a missing-requirement error, E454):

```kestrel
struct Snapshot: Cloneable {
    var name: String
    var hits: Int64

    func clone() -> Snapshot {
        // custom behavior: clones reset the counter
        Snapshot(name: self.name.clone(), hits: 0)
    }
}
```

### Warning: `self` Inside `clone()` Is a Bitwise Copy

Inside a hand-written `clone()` body the compiler deliberately suppresses clone-insertion (otherwise `clone()` would recurse forever). Returning `self` — or a payload bound out of `self` — therefore **aliases** every heap field instead of duplicating it, and both values will free the same buffer:

```kestrel
// WRONG — aliases the String; double-free on drop
enum Token: Cloneable {
    case Plain
    case Literal(String)
    func clone() -> Token { self }
}

// RIGHT — deep-clone each heap payload; bare `self` only for payload-less cases
enum Token: Cloneable {
    case Plain
    case Literal(String)
    func clone() -> Token {
        match self { .Literal(s) => .Literal(s.clone()), _ => self }
    }
}
```

If you don't need custom behavior, don't declare the conformance — the synthesized memberwise clone is always correct.

## Cloneable vs not Copyable

These are mutually exclusive; combining them is rejected (E423, `conflicting_copyable_opt_out`):

```kestrel
// ERROR(E423): Cannot be both Cloneable and not Copyable
struct Invalid: Cloneable, not Copyable {
    func clone() -> Invalid { ... }
}
```

If you want *explicit-only* duplication for a resource type, keep it `not Copyable` and provide an ordinary method:

```kestrel
struct Resource: not Copyable {
    var handle: Int64

    // Not clone() — just a regular method
    func duplicate() -> Resource {
        Resource(handle: duplicateHandle(self.handle))
    }
}

let a = Resource(...);
let b = a.duplicate();  // Explicit duplication
let c = a;              // Move, not copy — a is now invalid
```

## Generics

`Cloneable` participates in bounds like any protocol, and generic copies dispatch through the protocol witness at monomorphization:

```kestrel
func duplicate[T](item: T) -> (T, T) {
    (item, item)   // T: Copyable is the default bound;
}                  // if the instantiating type is Cloneable, clone() is called

func deepCopy[T](item: T) -> T where T: Cloneable {
    item.clone()   // explicit clone requires the Cloneable bound
}
```

A conditionally-Copyable container (e.g. `Optional[T]`) is also conditionally *Cloneable*: `Optional[String]` satisfies a `Cloneable` bound because `String` does, and cloning it deep-clones the payload. See [generics.md](generics.md).

---

## Design Notes

- **Hidden cost**: `let b = a;` on a Cloneable type can be arbitrarily expensive. This is the accepted trade-off for value semantics; know your types.
- **`clone()` is infallible** — it returns `Self`, not a `Result`. For fallible duplication provide a separate `tryClone() -> Self throws E` method.
- **Clone/deinit symmetry**: a type with a `deinit` that releases a resource must either be `not Copyable` or make its duplication safe. Note that types with `deinit` and only-Copyable fields remain bitwise-Copyable — the compiler does not force `Cloneable` on them, so a `deinit` that frees a raw pointer field plus implicit copying is a double-free you must design away (wrap the pointer, or opt out with `not Copyable`).
- **Explicit clone is always available**: `let b = a.clone();` and `let c = a;` behave identically for Cloneable types.
