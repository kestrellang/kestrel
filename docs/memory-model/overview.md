# Kestrel Memory Model

Kestrel's memory model is designed to be an **application language first** that scales down to **zero-cost systems programming** when needed.

## Design Philosophy

- **Ergonomic by default**: Most code "just works" without annotations
- **Gentle learning curve**: Copy-by-default semantics for application developers
- **Precise control available**: Move-only types, ownership annotations, and second-class references for systems programming
- **Safety without complexity**: No explicit lifetime annotations — dangling references and use-after-move are rejected by compiler analysis, not by a lifetime language

## Features

The memory model includes:
- Implicit Copyable structs (copy-by-default)
- `not Copyable` opt-out for move-only types
- `Cloneable` protocol for custom copy behavior
- `borrow`, `mutating`, `consuming` access modes
- Second-class references (`&T`, `&mutating T`): ref returns, ref bindings, ref struct fields — all escape-checked
- Copy-by-default generics with `not Copyable` bounds and per-instantiation conditional copyability
- RAII via `deinit` blocks, plus the explicit `deinit x;` statement for early cleanup
- A flow-sensitive move checker (E500/E501/E503/E506) and a provenance-based escape checker (E494)

| Feature | Document |
|---------|----------|
| Access Modes | [access-modes.md](access-modes.md) |
| Copy Semantics | [copy-semantics.md](copy-semantics.md) |
| Cloneable Protocol | [cloneable.md](cloneable.md) |
| MIR ABI | [abi.md](abi.md) |
| Generics | [generics.md](generics.md) |
| Closures | [closures.md](closures.md) |
| Drop Semantics | [drop-semantics.md](drop-semantics.md) |
| Diagnostics (move & escape checking) | [diagnostics.md](diagnostics.md) |
| Limitations | [limitations.md](limitations.md) |

## Ownership Model

Kestrel uses **value semantics by default** with **ownership conventions** for parameters:

```kestrel
func read(point: Point) { }              // Borrowing (default) - read-only access
func update(mutating point: Point) { }   // Mutating - read-write access
func consume(consuming point: Point) { } // Consuming - takes ownership
```

Parameters never carry reference *types* — the access mode on the signature is the whole story at a call site, and spelling `&x` at a call site is rejected (E488).

Beyond conventions, Kestrel has **second-class reference types** `&T` and `&mutating T`. They can be returned from functions and accessors, bound to local names with `let r = &place;`, and stored in struct fields — but they are not first-class values: they cannot appear in parameter types, function types, or long-lived (static/heap) storage. Instead of lifetime annotations, every reference carries a **provenance root**, and the compiler rejects any reference that would outlive its root (error E494):

```kestrel
struct Person {
    var age: Int64
    func ageRef() -> &Int64 { self.age }   // OK: rooted at the receiver
}

func bad() -> &Int64 {
    let x = 42;
    x   // ERROR(E494): the root `x` dies at return
}
```

See [limitations.md](limitations.md) for exactly where references may and may not appear, and [diagnostics.md](diagnostics.md) for the escape-checking model.
