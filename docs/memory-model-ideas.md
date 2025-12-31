# Kestrel Memory Model Design

## Overview

This document describes Kestrel's memory model, designed to be an **application language first** that scales down to **zero-cost systems programming**.

The design prioritizes ergonomics and gentle learning curve (Copy-by-Default) while allowing precise control over resources (Move-Only) when needed.

---

## Core Concepts

### Access Modes

Every value access has one of three modes:

| Mode | Keyword | Meaning | Ownership | Original Variable |
|------|---------|---------|-----------|-------------------|
| **Borrow** | (default) | Read-only access | Caller retains | Valid |
| **Mutating** | `mutating` | Read-write access | Caller retains | Valid |
| **Consuming** | `consuming` | Takes ownership | Caller loses* | Moved (Invalid) or Copied |

*\*If the type is Copyable, "losing ownership" effectively means receiving a copy. If NonCopyable, it is a true move.*

### Copy Semantics (Application First)

Kestrel aims for "it just works" behavior for application developers.

1.  **Implicit Copyable**: A `struct` or `enum` is automatically `Copyable` if **all** its fields are `Copyable`.
    *   *Example:* `struct Point { x: Int, y: Int }`
    *   *Behavior:* Can be assigned to multiple variables. Passes by copy.

2.  **Implicit NonCopyable**: If a type contains a non-copyable field, it automatically becomes `not Copyable`.
    *   *Example:* `struct Wrapper { f: FileHandle }`

3.  **Explicit Opt-Out**: You can mark a type as `not Copyable` to enforce uniqueness.
    *   *Syntax:* `struct Ticket: not Copyable { ... }`
    *   *Behavior:* Can only be **moved**. Assignment invalidates the original.

---

## Generics: Copy-by-Default

To maintain the "Application Language" feel, generics follow the rule of least surprise for high-level code.

### The Rule
A generic type parameter `T` is assumed to be **Copyable** by default.

```kestrel
// Application code is simple:
func duplicate<T>(item: T) -> (T, T) {
    return (item, item) // OK! T is assumed Copyable.
}

let p = Point(x: 1, y: 1)
duplicate(p) // Works

let f = FileHandle(...)
duplicate(f) // ERROR: FileHandle does not conform to Copyable
```

### The "Systems" Opt-In (`not Copyable`)

Standard library authors and systems programmers can write code that works with **any** type (including non-copyable resources) by relaxing the constraint using `not Copyable`.

```kestrel
// List can hold Points AND FileHandles.
// "T: not Copyable" means "This generic does not require Copyable conformance."
struct List<T: not Copyable> { ... }

func push<T: not Copyable>(mutating list: List<T>, item: consuming T) {
    // We can move 'item' into the list memory.
    // We CANNOT copy 'item' because we don't know if it's Copyable.
}
```

**Summary of Generic Bounds:**
*   `<T>`: Must be Copyable (Default). Easy to use.
*   `<T: not Copyable>`: Accepts anything (Copyable or NonCopyable). More flexible, but cannot copy values.

---

## Parameter Passing

### Default: Borrow
Parameters are borrowed by default (read-only):
```kestrel
func printPoint(p: Point) {
    print(p.x)
} // p is borrowed
```

### Mutating Parameters
Use `mutating` for write access. The caller *must* pass a mutable binding (`var`).
```kestrel
func reset(mutating p: Point) {
    p.x = 0
}

var p = Point(x: 1, y: 2)
reset(p)  // p is mutably borrowed
```

### Consuming Parameters
Use `consuming` to take ownership.
```kestrel
func consume(consuming p: Point) {
    print(p.x)
} // p is dropped here

let p = Point(...)
consume(p) 
// If Point is Copyable: p is still valid (it was copied).
// If Point is not Copyable: p is now invalid (it was moved).
```

---

## References & Lifetimes

Kestrel supports references with inferred lifetimes, allowing safe borrowing without explicit lifetime annotations.

### Reference Types

| Syntax | Meaning |
|--------|---------|
| `&T` | Immutable reference to T |
| `&var T` | Mutable reference to T |

### Lifetime Model

Every value in Kestrel has a lifetime:

1. **Parameters**: Lifetime tied to the function call
2. **Locals**: Lifetime tied to their scope
3. **References**: Lifetime of what they refer to
4. **Return types**: Lifetime inferred from inputs

### Lifetime Inference

The compiler infers return lifetimes automatically. When a function has a single reference input, the output lifetime is tied to it:

```kestrel
func first(list: &List[Int]) -> &Int {
    &list.items[0]  // Lifetime inferred from 'list'
}
```

For multiple inputs, the compiler uses the minimum lifetime:

```kestrel
func pick(a: &Int, b: &Int, useFirst: Bool) -> &Int {
    if useFirst { a } else { b }  // Lifetime = min(a, b)
}
```

### Structs with Reference Fields

Structs may contain reference fields. The struct's lifetime becomes the minimum of all its reference fields' lifetimes:

```kestrel
struct MutexGuard[T] {
    var mutex: &var Mutex[T]

    func get(self: &Self) -> &T {
        &self.mutex.value
    }

    func get_mut(self: &var Self) -> &var T {
        &self.mutex.value
    }

    deinit {
        self.mutex.unlock()
    }
}
```

**Usage:**
```kestrel
var m = Mutex(value: 42)
var guard = m.lock()      // guard's lifetime ≤ m's lifetime
guard.get_mut() = 100     // borrow through guard
// guard dropped here, calls deinit, unlocks mutex
```

### No Deref Traits

Kestrel does not have implicit dereferencing. Smart pointer types like `MutexGuard` provide explicit accessor methods (`.get()`, `.get_mut()`) rather than overloading `*` or auto-forwarding field access. This keeps borrows visible in source code.

---

## Closures & Escape Analysis

Closures are critical for Kestrel's memory safety.

### Capture Lists
Closures capture their environment. You can control *how* they capture:

```kestrel
let closure = { [x, mutating y, consuming z] in
    // x: borrowed
    // y: mutably borrowed
    // z: owned (moved/copied)
}
```

### Escape Analysis
1.  **Non-Escaping**: Captures by `borrow`/`mutating`. Stack-allocated. Safe.
2.  **Escaping**: Captures by `consuming`. Heap-allocated (usually). Owns data.

---

## Drop Semantics (RAII)

### `deinit`
Types can define cleanup logic:
```kestrel
struct FileHandle: not Copyable {
    var fd: Int
    deinit { close(self.fd) }
}
```

### Drop Rules
1.  **Scope Exit**: Values are dropped at end of scope.
2.  **Move Semantics**: If a value is **moved**, its `deinit` is **NOT** called. The new owner is responsible.

---

## Limitations

### No Explicit Lifetime Annotations
Lifetimes are always inferred. Complex patterns requiring explicit lifetime parameters (e.g., a struct holding references to two different sources with independent lifetimes) are not expressible. The compiler will reject code where lifetimes cannot be unambiguously inferred.

### No Self-Referential Structs
A struct cannot hold a reference to its own fields:
```kestrel
struct Bad {
    var data: [Int]
    var first: &Int  // Cannot point to data[0]
}
```

---

## Feature Roadmap

### Phase 1: The Application Layer (Implemented)
- Implicit Copyable Structs.
- `not Copyable` opt-out syntax.
- `borrow`, `mutating`, `consuming` modes.
- Copy-by-Default Generics.

### Phase 2: The Systems Layer
- `not Copyable` generic bound (relaxing constraints).
- Enforcement of Move semantics for `not Copyable` types.
- References (`&T`, `&var T`) with inferred lifetimes.
- Structs with reference fields.

### Phase 3: Advanced Systems (if needed)
- Explicit lifetime annotations for complex borrowing patterns.
