# For Loops Design

## Overview

Add `for` loop syntax to Kestrel that desugars to `while let` loops, following Rust's approach. This provides ergonomic iteration over any type that conforms to the `Iterable` protocol.

## Syntax

```kestrel
// Basic for loop
for item in collection {
    // body
}

// With pattern destructuring
for (key, value) in pairs {
    // body
}

// With mutable binding
for var item in collection {
    item = transform(item)
    process(item)
}

// With label for break/continue
outer: for item in collection {
    for inner_item in other {
        if condition {
            break outer
        }
    }
}

// Wildcard pattern (execute N times)
for _ in 0..<10 {
    doSomething()
}
```

## Desugaring

The for loop:
```kestrel
label: for pattern in expression {
    body
}
```

Desugars to:
```kestrel
{
    var iter = expression.iter()
    label: while let .Some(pattern) = iter.next() {
        body
    }
}
```

Key aspects:
- **Block scope**: The iterator `iter` is scoped to the desugared block, not leaking to surrounding scope
- **Mutable iterator**: `var iter` because `next()` is a `mutating` method
- **Pattern preserved**: The user's pattern is nested inside `.Some(pattern)`
- **Label transferred**: The label attaches to the `while let`, enabling `break label` and `continue label`

## Semantic Behavior

### Type Constraints

The expression must conform to `Iterable`:
```kestrel
@builtin(.IterableProtocol)
public protocol Iterable {
    type Item
    type Iter: Iterator where Iter.Item = Item

    @builtin(.IterableIterMethod)
    func iter() -> Iter
}
```

The iterator must conform to `Iterator`:
```kestrel
@builtin(.IteratorProtocol)
public protocol Iterator {
    type Item

    @builtin(.IteratorNextMethod)
    mutating func next() -> Optional[Item]
}
```

### Pattern Requirements

- Patterns must be **irrefutable** (enforced by `while let`)
- Full pattern syntax supported: bindings, tuples, structs, wildcards, etc.
- Mutability controlled by `var` keyword: `for var x in items`

### Return Type

- For loops always return `()` (unit type)
- No break-with-value support (same as `while` loops)

### Control Flow

- `break` - exits the for loop
- `continue` - skips to next iteration
- `break label` / `continue label` - targets labeled loop
- `return` - returns from enclosing function

### Move Semantics

- `expression.iter()` borrows or consumes the collection based on `iter()` signature
- Standard library `iter()` methods typically borrow (non-mutating)
- The iterator itself is consumed by the loop (owned by the desugared block)

## New Builtins

### Iterator/Iterable Builtins

| Builtin | Kind | Purpose |
|---------|------|---------|
| `IteratorProtocol` | Protocol | Marks the `Iterator` protocol |
| `IteratorNextMethod` | ProtocolMethod | Marks `next()` for resolution |
| `IterableProtocol` | Protocol | Marks the `Iterable` protocol |
| `IterableIterMethod` | ProtocolMethod | Marks `iter()` for resolution |

### Optional Builtins

| Builtin | Kind | Purpose |
|---------|------|---------|
| `OptionalEnum` | Enum | Marks the `Optional` enum |
| `OptionalSomeCase` | EnumCase | Marks `Some` case for pattern construction |
| `OptionalNoneCase` | EnumCase | Marks `None` case |

## Standard Library Changes

### Update `iter/iterator.ks`

```kestrel
@builtin(.IteratorProtocol)
public protocol Iterator {
    type Item

    @builtin(.IteratorNextMethod)
    mutating func next() -> Optional[Item]
}

@builtin(.IterableProtocol)
public protocol Iterable {
    type Item
    type Iter: Iterator where Iter.Item = Item

    @builtin(.IterableIterMethod)
    func iter() -> Iter
}

// Allow iterating an iterator directly
extend Iterator: Iterable {
    type Item = Item
    type Iter = Self

    func iter() -> Self { self }
}
```

### Update `result/optional.ks`

```kestrel
@builtin(.OptionalEnum)
public enum Optional[T] {
    @builtin(.OptionalSomeCase)
    case Some(T)

    @builtin(.OptionalNoneCase)
    case None
}
```

## Error Cases

| Condition | Error Message | Source |
|-----------|---------------|--------|
| Expression doesn't implement `Iterable` | Type `X` does not conform to `Iterable` | Type checker |
| Refutable pattern | Pattern may not match all values | `while let` irrefutability check |
| Break/continue outside loop | `break`/`continue` outside of loop | Loop context check |
| Unknown label | Unknown loop label `name` | Label resolution |
| Type mismatch in pattern | Expected `X`, found `Y` | Pattern type inference |

## Edge Cases

1. **Empty iterator**: Loop body never executes, returns `()`
2. **Infinite iterator**: Loop runs forever (or until break/return)
3. **Nested for loops**: Each has its own iterator scope, labels disambiguate control flow
4. **Iterator that panics**: Panic propagates normally
5. **Pattern with guards**: Not supported (guards are for `match`, not `for`)

## Interactions with Other Features

- **Ranges**: `for i in 0..<10` works because `Range` implements `Iterable`
- **Arrays/Collections**: Work when they implement `Iterable`
- **Closures**: Can capture loop variables (each iteration gets fresh binding)
- **Async**: No special async-for syntax (future consideration)

## Open Questions (Resolved)

1. **Where to desugar?**
   - Resolution: Binder phase, preserving source fidelity for error messages

2. **Full pattern support?**
   - Resolution: Yes, reuse existing pattern infrastructure

3. **Labels?**
   - Resolution: Yes, same as while/loop

4. **Iterator self-iteration?**
   - Resolution: Yes, `Iterator: Iterable` with `iter() -> Self`

5. **Optional as builtin?**
   - Resolution: Yes, with `Some` and `None` cases as builtins too

6. **Refutable patterns?**
   - Resolution: No, require irrefutable (enforced by while let)
