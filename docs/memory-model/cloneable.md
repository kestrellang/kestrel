# Cloneable Protocol

Kestrel provides a unified copy model where types can customize how they are copied through the `Cloneable` protocol.

## Overview

| Type | Copy Behavior |
|------|---------------|
| Simple struct (all fields Copyable) | Implicit bitwise copy |
| Struct implementing `Cloneable` | Implicit copy calls `clone()` |
| `not Copyable` struct | Cannot be copied, only moved |

## The `Cloneable` Protocol

```kestrel
protocol Cloneable: Copyable {
    func clone(self) -> Self
}
```

`Cloneable` extends `Copyable`. If a type is `Cloneable`, it is automatically `Copyable`, but copies go through the custom `clone()` implementation.

## Basic Usage

### Default Copy (No Cloneable)

Simple types use compiler-generated bitwise copy:

```kestrel
struct Point {
    var x: Int
    var y: Int
}

let a = Point(x: 1, y: 2)
let b = a  // Bitwise copy
print(a.x) // 1 - a is still valid
print(b.x) // 1 - b is independent copy
```

### Custom Copy (Cloneable)

Types that manage resources implement `Cloneable`:

```kestrel
struct MyString: Cloneable {
    var buffer: Pointer[Char]
    var len: Int
    
    func clone(self) -> Self {
        let newBuffer = allocate(self.len)
        memcpy(newBuffer, self.buffer, self.len)
        MyString(buffer: newBuffer, len: self.len)
    }
    
    deinit {
        free(self.buffer)
    }
}

let a = MyString("hello")
let b = a  // Implicitly calls a.clone() - deep copy
// Both a and b have independent buffers
```

## Implicit Clone Behavior

When a `Cloneable` type is copied, `clone()` is called automatically:

```kestrel
func process(s: MyString) {  // s is copied (consuming not specified)
    print(s)
}

let original = MyString("hello")
process(original)  // Implicitly calls original.clone()
print(original)    // Still valid - original was cloned, not moved
```

This applies to:
- Assignment: `let b = a`
- Parameter passing (non-consuming)
- Return values
- Storing in collections

## Derived Cloneable

If all fields of a struct are `Cloneable` (or simple `Copyable`), the compiler can derive `Cloneable`:

```kestrel
struct Document: Cloneable {
    var title: MyString
    var body: MyString
    
    // Compiler-derived clone():
    // func clone(self) -> Self {
    //     Document(
    //         title: self.title.clone(),
    //         body: self.body.clone()
    //     )
    // }
}
```

**Question**: Should derived `Cloneable` be automatic, or require explicit declaration?

Options:
1. Automatic if all fields are Cloneable
2. Require `struct Document: Cloneable { ... }` to opt in
3. Automatic, but allow override

## Cloneable vs not Copyable

These are mutually exclusive:

```kestrel
// ERROR: Cannot be both Cloneable and not Copyable
struct Invalid: Cloneable, not Copyable {
    func clone(self) -> Self { ... }
}
```

A `not Copyable` type cannot implement `Cloneable`. It can only be moved.

If you want explicit-only copying for a resource type:

```kestrel
struct Resource: not Copyable {
    var handle: Int
    
    // Not clone() - just a regular method
    func duplicate(self) -> Resource {
        Resource(handle: duplicateHandle(self.handle))
    }
}

let a = Resource(...)
let b = a.duplicate()  // Explicit duplication
let c = a              // Move, not copy
// a is now invalid
```

---

## Potential Issues

### 1. Hidden Performance Costs

Implicit cloning can be expensive:

```kestrel
struct BigData: Cloneable {
    var items: Array[Int]  // 1 million elements
    
    func clone(self) -> Self {
        // Copies 1 million integers!
        BigData(items: self.items.clone())
    }
}

let a = BigData(...)
let b = a  // Silently copies 1 million integers
```

**Concern**: No visual indication of expensive operation.

**Counterargument**: This is the trade-off for ergonomics. Developers should know their types.

**Mitigation**: Linter warnings for large Cloneable types? Profiling tools?

### 2. When Does Clone Happen?

Borrow is the default parameter mode, so cloning does NOT happen on regular function calls:

```kestrel
func process(data: BigData) { ... }  // Borrows - no clone!

process(myData)  // No clone, just borrows
```

Cloning happens on:
- **Assignment**: `let b = a`
- **`consuming` parameters**: `func take(consuming data: BigData)`
- **Escaping closure captures**: when a closure outlives its scope
- **Storing in data structures**: `array.push(item)`

### 3. Cloneable Fields in Copyable Structs

What if a struct has a mix of simple and Cloneable fields?

```kestrel
struct Mixed {
    var x: Int        // Simple copy
    var s: MyString   // Needs clone()
}

let a = Mixed(x: 1, s: MyString("hello"))
let b = a  // Is Mixed Copyable? Cloneable?
```

**Expected behavior**: `Mixed` is implicitly `Cloneable`. Copy calls `clone()` on the `MyString` field.

### 4. Clone Cycles

Cloneable types with cyclic references:

```kestrel
struct Node: Cloneable {
    var value: Int
    var next: Optional[Box[Node]]
    
    func clone(self) -> Self {
        Node(
            value: self.value,
            next: self.next.clone()  // Recursively clones the chain
        )
    }
}
```

**Concern**: Deep clone of a long chain or cycle could stack overflow or be very slow.

**Mitigation**: Same as any recursive algorithm - developer responsibility.

### 5. Clone in Generic Contexts

How does `Cloneable` interact with generics?

```kestrel
func duplicate[T](item: T) -> (T, T) {
    (item, item)  // Two uses - needs copy
}

// If T is Cloneable, clone() is called
// If T is simple Copyable, bitwise copy
```

**Expected**: The compiler dispatches to `clone()` if available at monomorphization time.

**Question**: What if you need to *require* Cloneable?

```kestrel
func deepCopy[T: Cloneable](item: T) -> T {
    item.clone()  // Explicit clone call
}
```

### 6. Partial Clone Failures

What if `clone()` can fail?

```kestrel
struct Resource: Cloneable {
    func clone(self) -> Self {
        let handle = tryDuplicateHandle(self.handle)
        if handle == -1 {
            // What now? Cannot return error from clone()
            panic("clone failed")
        }
        Resource(handle: handle)
    }
}
```

**Constraint**: `clone()` must be infallible (returns `Self`, not `Result[Self, Error]`).

**Workaround**: For fallible duplication, use a separate method:

```kestrel
func tryClone(self) -> Result[Self, Error] { ... }
```

### 7. Clone and Deinit Symmetry

If a type has `deinit`, it probably needs custom `clone()`:

```kestrel
struct Handle: Cloneable {
    var fd: Int
    
    func clone(self) -> Self {
        Handle(fd: duplicate(self.fd))  // Must duplicate, not share!
    }
    
    deinit {
        close(self.fd)
    }
}
```

**Concern**: Forgetting to implement `clone()` for a type with `deinit` could lead to double-free.

**Mitigation**: 
- Compiler warning if a type has `deinit` but uses default copy?
- Require explicit `Cloneable` implementation for types with `deinit`?

### 8. Cloneable Standard Library Types

Which standard library types should be Cloneable?

```kestrel
// These should be Cloneable:
struct String: Cloneable { ... }
struct Array[T: Cloneable]: Cloneable { ... }
struct HashMap[K, V: Cloneable]: Cloneable { ... }

// These might not be:
struct File: not Copyable { ... }  // Resource, not cloneable
```

### 9. Explicit Clone Syntax

Sometimes you want to be explicit about cloning:

```kestrel
let a = MyString("hello")
let b = a.clone()  // Explicit - always works for Cloneable types
let c = a          // Implicit - same behavior
```

Both should work. Explicit `clone()` call is always available for Cloneable types.

### 10. Cloneable and Consuming

How does Cloneable interact with `consuming`?

```kestrel
func take(consuming s: MyString) { ... }

let a = MyString("hello")
take(a)  // Does this clone or move?
```

**Expected**: `consuming` means transfer ownership. For Cloneable types, this *could* be:
1. Clone then move the clone (original stays valid)
2. Move the original (original invalid)

**Recommendation**: `consuming` should move, not clone. If you want the original to stay valid, don't use `consuming`.

```kestrel
take(a)       // Moves a (a is invalid after)
take(a.clone()) // Explicit clone, then move the clone
```
