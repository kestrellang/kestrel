# Kestrel Memory Model

Kestrel's memory model is designed to be an **application language first** that scales down to **zero-cost systems programming** when needed.

## Design Philosophy

- **Ergonomic by default**: Most code "just works" without annotations
- **Gentle learning curve**: Copy-by-default semantics for application developers
- **Precise control available**: Move-only types and ownership annotations for systems programming
- **Safety without complexity**: No explicit lifetime annotations, no user-facing reference types

## Features

| Feature | Document | Status |
|---------|----------|--------|
| Access Modes | [access-modes.md](access-modes.md) | Phase 1 |
| Copy Semantics | [copy-semantics.md](copy-semantics.md) | Phase 1 |
| Cloneable Protocol | [cloneable.md](cloneable.md) | Phase 1 |
| Generics | [generics.md](generics.md) | Phase 1 |
| Closures | [closures.md](closures.md) | Phase 1 |
| Drop Semantics | [drop-semantics.md](drop-semantics.md) | Phase 1 |
| Limitations | [limitations.md](limitations.md) | - |

## The Law of Exclusivity

The foundation of Kestrel's memory safety is the Law of Exclusivity:

> If a variable is being accessed, no other overlapping access to that variable
> may occur unless both accesses are reads.

This is enforced:
- **Statically** for local variables, `inout` arguments, and value-type properties
- **Dynamically** for class properties, global variables, and escaped closures

## Ownership Model

Kestrel uses **value semantics by default** with **ownership conventions** for parameters:

```kestrel
func read(point: Point) { ... }              // Borrowing (default) - read-only access
func update(mutating point: Point) { ... }   // Mutating - read-write access  
func consume(consuming point: Point) { ... } // Consuming - takes ownership
```

There are no user-facing reference types (`&T`). Borrowing is a calling convention, not a type constructor.

## Phases

### Phase 1: Core Ownership
- Implicit Copyable structs
- `not Copyable` opt-out for move-only types
- `Cloneable` protocol for custom copy behavior
- `borrow`, `mutating`, `consuming` access modes
- Copy-by-default generics
- Law of Exclusivity enforcement
- RAII via `deinit`


