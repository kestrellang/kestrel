# Kestrel Semantics Guide

This guide details the semantic model of Kestrel, focusing on memory management, ownership, and type system behavior.

> **Note**: Features marked as *(Future)* are planned but not yet fully implemented.

## Memory Model

Kestrel uses a unique memory model designed to be "application-first" while scaling to systems programming needs.

### Copy by Default
Unlike Rust, **most types in Kestrel are copy-by-default**. This includes:
- Primitives (`Int`, `Bool`, `Float`)
- Structs (unless opted out)
- Enums

When you pass a struct to a function or assign it to a new variable, it is copied.

```kestrel
struct Point {
    var x: Int;
    var y: Int;
}

func main() {
    let p1 = Point(x: 1, y: 2);
    let p2 = p1; // p1 is copied to p2
    // Both p1 and p2 are valid and independent
}
```

### Non-Copyable Types
Types can opt-out of copy semantics by conforming to `not Copyable` (or implicitly by containing a non-copyable field). These types behave with **move semantics** (similar to Rust).

```kestrel
struct File: not Copyable {
    // ...
}

func main() {
    let f1 = File.open("data.txt");
    let f2 = f1; // f1 is moved to f2
    // f1 is no longer valid
}
```

### RAII and Deinit
Resources are managed via Resource Acquisition Is Initialization (RAII). Types can define a `deinit` block that runs when the value goes out of scope.

```kestrel
struct File {
    var handle: Int;
    
    deinit {
        // Close file descriptor
    }
}
```

You can explicitly destroy a non-copyable value using the `deinit` statement.

```kestrel
deinit file; // Explicit early drop
```

## Function Parameter Modes

Kestrel functions explicitly declare how they handle parameters using access modes:

1.  **Borrowing (Default)**: Read-only access.
    ```kestrel
    func print(p: Point) { ... } // Reads p
    ```

2.  **Mutating**: Read-write access. The caller must pass a mutable variable.
    ```kestrel
    func offset(mutating p: Point, by: Int) {
        p.x += by;
    }
    ```

3.  **Consuming**: Takes ownership of the value.
    ```kestrel
    func close(consuming f: File) {
        // f is destroyed here
    }
    ```

### Method Receivers

Methods defined inside `struct`, `enum`, or `extension` blocks have implicit access to `self`. Unlike Rust, **methods do not declare `self` as an explicit parameter**.

```kestrel
struct Counter {
    var count: Int;
    
    // `self` is implicit - do NOT write `func increment(self)` or `func increment(&self)`
    mutating func increment() {
        self.count = self.count + 1;
    }
}
```

The method modifier determines how `self` can be accessed:
- **No modifier**: `self` is borrowed (read-only)
- **`mutating`**: `self` can be modified
- **`consuming`**: `self` is moved/consumed

## Generics and Monomorphization

Generics in Kestrel are **monomorphized**. This means a separate version of the function or type is generated for each concrete type argument used. This ensures zero-cost abstractions but increases binary size.

```kestrel
// A separate copy of `identity` is compiled for Int and String
let n = identity[Int](42);
let s = identity[String]("hello");
```

Generic types assume `Copyable` by default. To support non-copyable types, use `where T: not Copyable`.

## Error Handling

Kestrel uses **Typed Errors** via the `Result[T, E]` enum.

-   **Exceptions are Values**: Errors are regular values, not unchecked exceptions.
-   **Propagation**: The `try` keyword is used to propagate errors up the call stack *(Future)*.
-   **Exhaustive Handling**: You must handle the error case (e.g., via `match`) or propagate it.

```kestrel
enum FileError: Error {
    case NotFound
    case PermissionDenied
}

func read() -> Result[String, FileError] {
    // ...
}
```

## Protocols

Protocols define a set of requirements (methods, properties) that a type must fulfill.

-   **Explicit Conformance**: Types must explicitly declare conformance via `extension Type: Protocol`.
-   **Static Dispatch**: Calls to protocol methods on concrete types are statically dispatched (resolved at compile time).
-   **Attributes**: Protocols can use attributes like `@builtin(.Copyable)` to interface with the compiler.

## Closures

Closures capture their environment **by value** (copy).
-   Captured variables are immutable inside the closure by default.
-   Closures are first-class values and can be passed around.
-   `{ it }` syntax is sugar for single-argument closures.

## Type Resolution

-   **Local Type Inference**: Kestrel infers types within function bodies.
-   **Implicit Member Access**: If the expected type is known (e.g. an Enum), you can use `.Case` instead of `Enum.Case`.
