# Limitations

Kestrel's memory model makes deliberate trade-offs for simplicity. This document describes what is supported, what is deliberately restricted, and the workarounds.

## References Are Second-Class

Kestrel *does* have reference types — `&T` (shared) and `&mutating T` (exclusive) — but they are deliberately **second-class**: they can travel through a computation, not live in one place indefinitely. Safety comes from provenance-based escape checking (E494 family, see [diagnostics.md](diagnostics.md)), not from lifetime annotations.

### What Works

**Returning borrowed data** (parameter-rooted):

```kestrel
struct Person {
    var age: Int64
    func ageRef() -> &Int64 { self.age }
    mutating func ageMut() -> &mutating Int64 { self.age }
}

var p = Person(age: 42);
let a = p.ageRef();     // binding a ref-returning CALL decays: `a` is an owned copy
p.ageMut() = 50;        // ref results are places: assign/compound-assign through them
```

**Named reference bindings** (`&place` in a `let` initializer names the place; reads see later writes, and bindings thread across `if`/`match` merges):

```kestrel
var x = 10;
let r = &x;             // shared view of the place
x = 20;
print(r);               // 20

let m = &mutating x;    // exclusive view
m = m + 1;              // writes the referent (no rebinding spelling exists)
```

**References in struct fields, tuples, and generic arguments** (the aggregate becomes non-`Static` — it can't outlive the borrow either):

```kestrel
struct Cursor {
    var item: &Int64
    var count: Int64
}

let opt: Optional[&Int64] = ...;   // Optional[T] declares `where T: not Static`
```

**Conformances on references** — `extend &T: P` makes `&T` satisfy protocol bounds (e.g. the stdlib's `Equatable`/`Comparable` on `&T` where the pointee conforms):

```kestrel
extend &T: Probe where T: Probe {
    public func probe() -> Int64 { self.probe() }
}
```

**Dangling references are rejected, not inexpressible** — the escape checker roots every reference and refuses any that outlives its root:

```kestrel
func bad() -> &Int64 {
    let x = 42;
    x                    // ERROR(E494): `x` dies at return
}
```

### What Is Restricted

| Restriction | Code |
|-------------|------|
| Ref types in **parameter** position — access modes are the only spelling (permanent by design) | E480 |
| `&x` at a **call site** — borrowing is the callee's convention, never spelled by the caller | E488 |
| Ref type in a `let`/`var` **annotation** — write `let r = &x;`, not `let r: &Int64 = ...` | E482 |
| Ref types inside **function types** (params or returns) | E480/E486 |
| Nested references (`&&T`) | E487 |
| Refs in **globals / static storage** — long-lived storage requires `Static` types | E505 |
| Ref-returning functions as **first-class values** | E491/E492 |
| A ref expression held open **across a control-flow merge** (hoist to a binding first) | E497 |
| Closures **capturing** a ref binding | E212 |

### When This Still Hurts

- **Zero-copy iterators yielding references** — iteration is by value; `for x in xs` copies/clones elements (ref-based iteration composes with accessors, e.g. `&arr(at: i)`, but there is no `Iterator` over `&T` elements in the stdlib's main loop path yet).
- **Long-lived views** — a struct holding `&T` cannot itself be stored long-term (non-`Static`); views are for passing down and across, not for keeping.

---

## No Lifetime Annotations

Kestrel will never have explicit lifetime annotations (`'a`). The escape checker is intentionally simpler than a full borrow checker: a reference's validity is tied to a single **root**, and multi-source returns (`if c { a.field } else { b.field }`) are rejected rather than given a lifetime union. For most application code this never surfaces; for complex borrow topologies, return owned data instead.

---

## No Self-Referential Structs

A struct cannot contain itself by value (E449/E450 at declaration), and a struct cannot hold a reference **into its own storage** — constructing one would root the ref at a local that the escape checker refuses to let escape alongside the struct. Use indices instead:

```kestrel
struct Buffer {
    var data: Array[Int64]
    var cursorIndex: Int64

    func current() -> Int64 {
        self.data(self.cursorIndex)
    }
}
```

Recursive *enums* are supported via `indirect enum` (heap indirection).

---

## Capturing Closures Cannot Escape

Closure environments are stack-allocated in the creating frame, so a closure that captures anything cannot be returned or stored beyond that frame (E494); capture-free closures are unrestricted. See [closures.md](closures.md). Heap-allocated environments are planned; the by-value capture semantics will not change.

---

## Copyable-by-Default Generics

Generic code assumes `Copyable` by default:

```kestrel
func duplicate[T](item: T) -> (T, T) {
    (item, item)
}

duplicate(myFileHandle);  // ERROR: FileHandle !: Copyable
```

Use a `not Copyable` bound for generic code that should accept move-only types (see [generics.md](generics.md)).

### Known gap: incomplete instantiation-time enforcement

The default bound is enforced at annotations, generic calls, and container construction, but **not on every inferred instantiation path**. In particular, array literals of non-Copyable elements currently slip through (`[Res(...), Res(...)]` forms an `Array[Res]`), and Copyable-default element access then bit-copies the element, producing a double-deinit at runtime. Until this is closed, keep non-Copyable values out of `Array`.

---

## Summary of Trade-offs

| Limitation | Benefit | Cost |
|------------|---------|------|
| Second-class references | No lifetime annotations; single-root escape checking | Long-lived views inexpressible |
| No lifetime annotations | Gentler learning curve | Multi-source borrows must return owned data |
| No self-referential structs | Move safety, simpler semantics | Must use indices |
| Stack-allocated closure environments | Zero-allocation closures | Capturing closures can't escape (yet) |
| Copyable-by-default generics | Application code just works | Library authors must opt out |

---

## Design Rationale

These limitations are intentional trade-offs for Kestrel's goals:

1. **Application-first**: Most application code doesn't need long-lived zero-copy views
2. **Gentle learning curve**: No lifetime language to learn — the compiler explains escapes in terms of "this value dies at return"
3. **Value semantics**: Reasoning about code is simpler when values are independent

If you consistently hit these limitations, you may be writing systems-level code that would benefit from Rust's full lifetime system. Kestrel prioritizes the common case over the complex case.
