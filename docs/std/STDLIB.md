# Standard Library Principles

## Protocol-Driven Design

Functionality comes from protocols, not inheritance. Types opt into capabilities:

```kestrel
extend MyType: Equatable { ... }  // Now supports ==, contains, etc.
extend MyType: Hashable { ... }   // Now usable as dictionary key
extend MyType: Iterable { ... }   // Now works with for-in loops
```

## Naming Conventions

| Pattern | Meaning | Example |
|---------|---------|---------|
| `verb()` | Mutates in place | `sort()`, `reverse()`, `clear()` |
| `verbed()` | Returns new value | `sorted()`, `reversed()`, `filtered()` |
| `isX` / `hasX` | Boolean property | `isEmpty`, `isSorted`, `hasPrefix` |
| `asX()` | Cheap conversion/view | `asSlice()`, `asPointer()` |
| `toX()` | Potentially expensive conversion | `toDict()`, `toString()` |

## Error Handling Patterns

| Pattern | Use When |
|---------|----------|
| `func x() -> T` | Failure is a bug (panic) |
| `func x() -> T?` | Failure is normal (Optional) |
| `func x() -> Result[T, E]` | Caller needs error details |
| `func tryX() -> Result[T, E]` | Variant of X with early exit |

```kestrel
arr(0)           // Panics if empty
arr.first()      // Returns Optional
file.read()      // Returns Result[String, IoError]
iter.tryFold()   // Folds with early exit on error
```

## Conditional Extensions

Methods only exist when constraints are met:

```kestrel
extend Array[T] where T: Comparable {
    func sort() { ... }  // Only exists for comparable elements
}
```

This means no runtime errors for missing capabilities - it's a compile-time check.

## Consistency Rules

1. **Symmetric pairs**: If `first()` exists, `last()` exists
2. **Predicate variants**: `firstIndex(of:)` pairs with `firstIndex(where:)`
3. **Checked variants**: Dangerous ops have safe alternatives (`arr(i)` vs `arr(checked: i)`)
4. **Iterator parity**: `DirectIterable` mirrors `Iterator` methods

## Zero-Cost Abstractions

High-level constructs compile to efficient code:

- Iterators inline to loops
- Generics monomorphize (no boxing)
- COW avoids copies until mutation
- Optionals have no overhead vs nullable pointers
