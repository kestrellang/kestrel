# Limitations

Kestrel's memory model makes deliberate trade-offs for simplicity. This document describes patterns that are not supported and their workarounds.

## No User-Facing Reference Types

Kestrel does not have `&T` or `&mut T` reference types. Borrowing is a calling convention, not a type constructor.

### What This Prevents

**Returning borrowed data:**

```kestrel
// CANNOT EXPRESS: returning a reference to internal data
func first(list: List[Int]) -> &Int {  // No &Int type
    &list.items(0)
}
```

**Storing references in structs:**

```kestrel
// CANNOT EXPRESS: struct holding a reference
struct Iterator {
    var current: &Int  // No reference fields
}
```

### Workarounds

1. **Return owned data (copy or move):**

```kestrel
func first(list: List[Int]) -> Int {
    list.items(0)  // Returns a copy
}
```

2. **Use indices instead of references:**

```kestrel
struct Iterator {
    var collection: List[Int]
    var index: Int
    
    func current(self) -> Int {
        self.collection.items(self.index)
    }
}
```

3. **Use closures for scoped access:**

```kestrel
func withFirst(list: List[Int], f: (Int) -> Void) {
    f(list.items(0))
}
```

4. **Use reference-counted types (when available):**

```kestrel
class SharedData {
    var value: Int
}
// Multiple owners can hold the same SharedData
```

### When This Hurts

- **Zero-copy iteration** - Iterators that yield references to elements
- **Parser combinators** - Parsers returning slices of input
- **View types** - String slices, array views without copying

These patterns require either copying data or waiting for `class` types and generalized accessors.

---

## No Self-Referential Structs

A struct cannot hold data that refers to its own fields.

### What This Prevents

```kestrel
// INVALID: self-referential struct
struct Buffer {
    var data: Array[Int]
    var cursor: Int  // OK: index is fine
    // But cannot have a "pointer" to data[0]
}
```

### Why It's Problematic

Self-referential data breaks when the struct is moved:

```kestrel
var b = Buffer(...)  // imagine cursor "points" to data[0]
var c = b            // Move b to c
// The internal "pointer" would still reference the OLD location
```

### Workarounds

1. **Use indices instead of pointers:**

```kestrel
struct Buffer {
    var data: Array[Int]
    var cursorIndex: Int
    
    func current(self) -> Int {
        self.data(self.cursorIndex)
    }
}
```

2. **Compute derived data on demand:**

```kestrel
struct Container {
    var items: Array[Int]
    
    func first(self) -> Int {
        self.items(0)  // No stored reference needed
    }
}
```

### When This Hurts

- **Intrusive data structures** - Linked lists with internal pointers
- **Cached computations** - Caching references to internal data
- **Generators/iterators** - Yielding references to internal state

---

## No Lifetime Annotations

Kestrel does not have explicit lifetime annotations like Rust's `'a`.

### What This Means

The compiler cannot express complex borrowing relationships:

```kestrel
// Cannot express: "output lives as long as input a, not b"
func selectFirst(a: String, b: String) -> String {
    a  // Must return an owned copy
}
```

### Impact

For most application code, this doesn't matter - you work with owned values. For performance-critical code that needs zero-copy access, you must use other patterns (closures, indices, or future reference-counted types).

---

## Limited Polymorphic Ownership

Generic code assumes `Copyable` by default:

```kestrel
func duplicate[T](item: T) -> (T, T) {
    (item, item)  // Works because T: Copyable is assumed
}

duplicate(myFileHandle)  // ERROR: FileHandle is not Copyable
```

### Workaround

Use `not Copyable` bound for generic code that should work with move-only types:

```kestrel
func wrap[T: not Copyable](consuming item: T) -> Box[T] {
    Box(item)
}
```

---

## Summary of Trade-offs

| Limitation | Benefit | Cost |
|------------|---------|------|
| No reference types | Simpler mental model, no lifetimes | Some patterns require copies |
| No self-referential structs | Move safety, simpler semantics | Must use indices |
| No lifetime annotations | Gentler learning curve | Complex borrowing inexpressible |
| Copyable-by-default generics | Application code just works | Library authors must opt-out |

---

## Design Rationale

These limitations are intentional trade-offs for Kestrel's goals:

1. **Application-first**: Most application code doesn't need zero-copy references
2. **Gentle learning curve**: No lifetime annotations to learn
3. **Value semantics**: Reasoning about code is simpler when values are independent

If you consistently hit these limitations, you may be writing systems-level code that would benefit from Rust's full lifetime system. Kestrel prioritizes the common case over the complex case.


