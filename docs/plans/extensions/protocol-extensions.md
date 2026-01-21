# Protocol Extensions Design

This document describes the design for default protocol methods via protocol extensions in Kestrel.

## Overview

Protocol extensions allow providing default implementations for protocol methods. Any type conforming to the protocol automatically gets these default implementations unless it provides its own.

## Syntax

```kestrel
// Basic protocol extension
extend Drawable {
    func drawTwice() {
        self.draw()
        self.draw()
    }
}

// Constrained protocol extension
extend Iterator where Self: Comparable, Self.Item: Equatable {
    func contains(value: Self.Item) -> Bool {
        while let item = self.next() {
            if item == value { return true }
        }
        return false
    }
}
```

### Where Clause Syntax

Protocol extension where clauses support:
- `Self: OtherProtocol` - the conforming type must also conform to another protocol
- `Self.AssociatedType: Protocol` - the associated type must conform to a protocol

The explicit `Self.` prefix is required (no implicit Self for associated types).

## Semantics

### Two Use Cases

1. **Add new methods** (not in protocol requirements):
   ```kestrel
   protocol Drawable {
       func draw()
   }

   extend Drawable {
       func drawTwice() {  // NEW method
           self.draw()
           self.draw()
       }
   }
   ```

2. **Provide defaults for required methods**:
   ```kestrel
   protocol Equatable {
       func equals(other: Self) -> Bool
       func notEquals(other: Self) -> Bool
   }

   extend Equatable {
       func notEquals(other: Self) -> Bool {  // Default for requirement
           return not self.equals(other)
       }
   }
   ```

### Method Resolution Order

When resolving a method call on a value, search in order:

1. **Concrete type's own methods** (highest priority)
2. **Type extensions** on the concrete type (specificity ordering)
3. **Protocol extensions** for protocols the type conforms to

For protocol extensions, when multiple could apply:
- **Most constrained wins** (more where clause constraints = more specific)
- **Equal constraints across different protocols = error** (ambiguous)

### Specificity Calculation

Protocol extension specificity = number of constraints in the where clause:

| Extension | Specificity |
|-----------|-------------|
| `extend Drawable` | 0 |
| `extend Drawable where Self: Fillable` | 1 |
| `extend Drawable where Self.Color: Equatable` | 1 |
| `extend Drawable where Self: Fillable, Self.Color: Equatable` | 2 |

### Dispatch

All dispatch is static (no dynamic dispatch). The compiler resolves which implementation to call at compile time based on the concrete type.

## Implementation

### Symbol Representation

Reuse `ExtensionSymbol` for both type extensions and protocol extensions. The distinction is made based on whether the target type is a struct/enum or a protocol.

`ExtensionTargetBehavior` already has:
- `target_type: Ty` - can be a protocol type
- `where_clause: WhereClause` - already supports constraints

### Registry

Use the same `ExtensionRegistry`, keyed by target `SymbolId`. For protocol extensions, the key is the protocol's SymbolId.

### Parser Changes

None required. The parser already supports:
- `extend TypeExpression { ... }`
- Where clauses with paths like `Self.Item`

### Binder Changes

1. **Extension target resolution**: When binding an extension, check if target resolves to a protocol (not just struct/enum)
2. **Protocol extension behavior**: Create appropriate `ExtensionTargetBehavior` for protocol targets
3. **Self handling**: In protocol extension where clauses:
   - `Self` refers to the conforming type (special, not a declared type parameter)
   - `Self.Item` resolves to the conforming type's associated type binding
4. **Registry**: Register protocol extensions by protocol SymbolId

### Method Resolution Changes

In `resolve_member_access` (after checking type extensions):

1. Get protocols the concrete type conforms to
2. For each protocol, query extensions from registry
3. Filter to applicable extensions (where clause satisfaction):
   - `Self: OtherProtocol` → check type conforms to OtherProtocol
   - `Self.Item: Bound` → resolve type's associated type binding, check conformance
4. Find method in applicable extensions
5. Apply specificity ordering; error on ambiguity

### Constraint Satisfaction

When checking if a protocol extension applies to concrete type `T`:

1. **`Self: Protocol`**: Check `T` conforms to `Protocol`
2. **`Self.AssociatedType: Protocol`**:
   - Find `T`'s binding for `AssociatedType` (via `ConformsToBehavior` on type alias)
   - Check that bound type conforms to `Protocol`

## Examples

### Basic Default Method

```kestrel
protocol Equatable {
    func equals(other: Self) -> Bool
}

extend Equatable {
    func notEquals(other: Self) -> Bool {
        return not self.equals(other)
    }
}

struct Point: Equatable {
    let x: Int
    let y: Int

    func equals(other: Point) -> Bool {
        return self.x == other.x and self.y == other.y
    }
}

let p1 = Point(x: 1, y: 2)
let p2 = Point(x: 1, y: 3)
p1.notEquals(p2)  // Uses default from extension
```

### Constrained Extension

```kestrel
protocol Iterator {
    type Item
    func next() -> Item?
}

extend Iterator where Self.Item: Equatable {
    func contains(value: Self.Item) -> Bool {
        while let item = self.next() {
            if item == value { return true }
        }
        return false
    }
}

struct IntRange: Iterator {
    type Item = Int
    var current: Int
    let end: Int

    func next() -> Int? {
        if self.current < self.end {
            let result = self.current
            self.current = self.current + 1
            return result
        }
        return nil
    }
}

let range = IntRange(current: 0, end: 10)
range.contains(5)  // Works because Int: Equatable
```

### Multiple Protocol Conformance

```kestrel
protocol A { }
protocol B { }

extend A {
    func foo() { print("A.foo") }
}

extend B {
    func foo() { print("B.foo") }
}

struct Thing: A, B { }

let t = Thing()
t.foo()  // ERROR: ambiguous - both A and B provide foo() with equal specificity
```

Resolution: `Thing` must provide its own `foo()` implementation.

## Open Questions

None currently - design is complete for initial implementation.

## References

- Swift protocol extensions: https://docs.swift.org/swift-book/documentation/the-swift-programming-language/protocols/#Protocol-Extensions
- Rust default trait methods: https://doc.rust-lang.org/book/ch10-02-traits.html#default-implementations
