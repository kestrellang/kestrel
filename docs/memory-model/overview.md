# Kestrel Memory Model

Kestrel's memory model is designed to be an **application language first** that scales down to **zero-cost systems programming** when needed.

## Design Philosophy

- **Ergonomic by default**: Most code "just works" without annotations
- **Gentle learning curve**: Copy-by-default semantics for application developers
- **Precise control available**: Move-only types and ownership annotations for systems programming
- **Safety without complexity**: No explicit lifetime annotations, no user-facing reference types

## Features

The memory model includes:
- Implicit Copyable structs (copy-by-default)
- `not Copyable` opt-out for move-only types
- `Cloneable` protocol for custom copy behavior
- `borrow`, `mutating`, `consuming` access modes
- Copy-by-default generics with `where T: not Copyable` bounds
- RAII via `deinit` blocks

| Feature | Document |
|---------|----------|
| Access Modes | [access-modes.md](access-modes.md) |
| Copy Semantics | [copy-semantics.md](copy-semantics.md) |
| Cloneable Protocol | [cloneable.md](cloneable.md) |
| Generics | [generics.md](generics.md) |
| Closures | [closures.md](closures.md) |
| Drop Semantics | [drop-semantics.md](drop-semantics.md) |
| Limitations | [limitations.md](limitations.md) |

## Ownership Model

Kestrel uses **value semantics by default** with **ownership conventions** for parameters:

```kestrel
func read(point: Point) { ... }              // Borrowing (default) - read-only access
func update(mutating point: Point) { ... }   // Mutating - read-write access  
func consume(consuming point: Point) { ... } // Consuming - takes ownership
```

There are no user-facing reference types (`&T`). Borrowing is a calling convention, not a type constructor.


