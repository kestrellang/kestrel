# Access Modes

Every parameter in Kestrel has one of three access modes that determine ownership transfer and mutability.

## Overview

| Mode | Keyword | Meaning | Ownership | Original Variable |
|------|---------|---------|-----------|-------------------|
| **Borrow** | (default) | Read-only access | Caller retains | Valid |
| **Mutating** | `mutating` | Read-write access | Caller retains | Valid |
| **Consuming** | `consuming` | Takes ownership | Caller loses* | Moved or Copied |

*If the type is Copyable, the caller receives a copy. If `not Copyable`, it is a true move.

## Borrow (Default)

Parameters are borrowed by default, providing read-only access:

```kestrel
func printPoint(p: Point) {
    print(p.x)
    print(p.y)
}

let p = Point(x: 1, y: 2)
printPoint(p)
print(p.x)  // OK: p is still valid
```

The callee cannot modify the borrowed value, and the caller retains full ownership.

## Mutating

Use `mutating` for write access. The caller must pass a mutable binding (`var`):

```kestrel
func reset(mutating p: Point) {
    p.x = 0
    p.y = 0
}

var p = Point(x: 1, y: 2)
reset(p)
print(p.x)  // Prints 0
```

### Mutating Self

Methods can declare `mutating self` to modify the receiver:

```kestrel
struct Counter {
    var value: Int
    
    func increment(mutating self) {
        self.value = self.value + 1
    }
}
```

## Consuming

Use `consuming` to take ownership of a value:

```kestrel
func consume(consuming p: Point) {
    print(p.x)
}  // p is dropped here

let p = Point(x: 1, y: 2)
consume(p)
// If Point is Copyable: p is still valid (copy was passed)
// If Point is not Copyable: p is now invalid (moved)
```

### Consuming Self

Methods can consume their receiver:

```kestrel
struct Connection: not Copyable {
    var handle: Int
    
    func close(consuming self) {
        // self is consumed, will be dropped after this method
    }
}
```

---

## Potential Issues

### 1. Implicit Borrow May Hide Performance Costs

For large Copyable types, borrowing avoids copies. But the implicit nature means developers might not realize when copies occur elsewhere:

```kestrel
func process(p: Point) { ... }      // Borrow - no copy
func store(consuming p: Point) { ... }  // If called with let binding, copies

let p = Point(x: 1, y: 2)
process(p)  // No copy
store(p)    // Implicit copy happens here for Copyable types
```

**Concern**: The `consuming` keyword at the call site is invisible. Developers may not realize copies are happening.

**Mitigation**: Linter warnings for large Copyable types passed to `consuming` parameters?

### 2. Mutating Requires `var` at Definition Site

```kestrel
let p = Point(x: 1, y: 2)
reset(p)  // ERROR: cannot pass 'let' binding to mutating parameter
```

This is correct behavior, but the error might be confusing if `reset` is called far from `p`'s definition.

**Mitigation**: Clear error messages pointing to both the call site and the binding.

### 3. Interaction with Closures

What happens when a closure captures a value that's later passed as `mutating` or `consuming`?

```kestrel
var x = 10
let closure = { print(x) }  // Captures x
reset(x)  // Can this happen while closure exists?
```

**Concern**: Need clear rules about when captures conflict with access modes.

**See also**: [closures.md](closures.md)

### 4. Method Chaining with Mutating

Mutating methods typically return `Void`, breaking fluent chains:

```kestrel
// This doesn't work:
counter.increment().increment()

// Must be:
counter.increment()
counter.increment()
```

**Question**: Should mutating methods be allowed to return `mutating Self` for chaining?

```kestrel
func increment(mutating self) -> mutating Self {
    self.value = self.value + 1
    return self
}
```

This adds complexity but enables ergonomic APIs.

### 5. Consuming + Copyable Ambiguity

The behavior of `consuming` differs based on whether the type is Copyable:

```kestrel
func take(consuming x: T) { ... }

take(myValue)  // Move or copy? Depends on T's Copyable conformance
```

**Concern**: Same syntax, different semantics. Could be surprising.

**Counterargument**: This is intentional—application code shouldn't need to care. The `consuming` keyword indicates intent, and the compiler does the right thing.
