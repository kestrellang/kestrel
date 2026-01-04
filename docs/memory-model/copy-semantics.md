# Copy Semantics

Kestrel's copy semantics prioritize ergonomics for application developers while providing escape hatches for systems programming.

## Implicit Copyable

A `struct` or `enum` is automatically `Copyable` if **all** its fields are `Copyable`:

```kestrel
struct Point {
    var x: Int
    var y: Int
}
// Point is implicitly Copyable (Int is Copyable)

let p1 = Point(x: 1, y: 2)
let p2 = p1  // Copy
print(p1.x)  // OK: p1 is still valid
```

### Built-in Copyable Types

- All integer types (`Int`, `Int8`, `Int16`, etc.)
- All floating point types (`Float`, `Double`)
- `Bool`
- `Char`
- Tuples of Copyable types
- Arrays of Copyable types (copies the array)
- Optional of Copyable types

## Implicit NonCopyable

If a type contains a non-copyable field, it automatically becomes non-copyable:

```kestrel
struct FileHandle: not Copyable {
    var fd: Int
}

struct Wrapper {
    var file: FileHandle
}
// Wrapper is implicitly not Copyable because FileHandle is not Copyable
```

## Explicit `not Copyable`

You can explicitly mark a type as non-copyable to enforce uniqueness, even if all fields are copyable:

```kestrel
struct Ticket: not Copyable {
    var id: Int
    var seat: String
}

let t1 = Ticket(id: 1, seat: "A1")
let t2 = t1  // MOVE, not copy
// t1 is now invalid
print(t1.id)  // ERROR: use of moved value
```

### Use Cases for Explicit `not Copyable`

1. **Unique resources**: Tickets, tokens, capabilities
2. **RAII wrappers**: File handles, mutex guards, connections
3. **Enforcing single ownership**: Preventing accidental aliasing
4. **Performance**: Avoiding copies of large structs

## Move Semantics

For `not Copyable` types, assignment and parameter passing are **moves**:

```kestrel
struct Connection: not Copyable {
    var handle: Int
}

let c1 = Connection(handle: 42)
let c2 = c1  // Move
// c1 is invalid after this point

func use(consuming conn: Connection) { ... }

let c3 = Connection(handle: 43)
use(c3)  // Move into function
// c3 is invalid after this point
```

---

## Potential Issues

### 1. Transitive NonCopyable Can Be Surprising

Adding a non-copyable field to an existing struct silently changes its semantics:

```kestrel
// Before: Copyable
struct Config {
    var name: String
    var timeout: Int
}

// After: NOT Copyable (breaking change!)
struct Config {
    var name: String
    var timeout: Int
    var logFile: FileHandle  // Makes Config non-copyable
}
```

**Concern**: This is a silent, potentially breaking change to existing code.

**Mitigation**: 
- Compiler warning when adding non-copyable fields to previously-copyable types?
- Or accept this as intentional: adding a resource field *should* change semantics.

### 2. Copyable Protocol Conformance

Is `Copyable` a real protocol that types conform to, or a compiler intrinsic?

```kestrel
// Can you write this?
func clone[T: Copyable](value: T) -> T {
    return value  // Relies on copy
}
```

**Question**: How does `Copyable` interact with the protocol system?

**Options**:
1. `Copyable` is a marker protocol with no methods
2. `Copyable` is a compiler intrinsic, not a real protocol
3. `Copyable` has a `copy() -> Self` method (explicit copies)

### 3. Partial Moves

Partial moves are **disallowed**. You cannot move a single field out of a struct:

```kestrel
struct Pair: not Copyable {
    var first: Resource
    var second: Resource
}

var p = Pair(first: r1, second: r2)
let x = p.first  // ERROR: cannot partially move out of 'p'
```

To extract a field, you must consume the entire struct:

```kestrel
func takeFirst(consuming p: Pair) -> Resource {
    p.first  // OK: p is being consumed entirely
}
```

### 4. Copying in Generic Contexts

When `T` is Copyable, should copies be explicit or implicit?

```kestrel
func duplicate[T: Copyable](item: T) -> (T, T) {
    return (item, item)  // Two uses of item - implicit copies?
}
```

This works because `T: Copyable` is the default. But it's implicit.

**Concern**: Developers might not realize copies are happening in generic code.

### 5. Large Copyable Types

A type being Copyable doesn't mean copying is cheap:

```kestrel
struct BigData {
    var items: Array[Int]  // 10,000 elements
}
// BigData is Copyable, but copying is expensive

let a = BigData(...)
let b = a  // Copies 10,000 integers!
```

**Mitigation**: 
- Linter warnings for large Copyable types?
- Explicit `copy()` method for expensive copies?
- Accept this as a user responsibility?

### 6. `not Copyable` Syntax Verbosity

`not Copyable` is 12 characters. For types that are commonly non-copyable, this adds noise:

```kestrel
struct FileHandle: not Copyable { ... }
struct MutexGuard: not Copyable { ... }
struct Connection: not Copyable { ... }
struct Ticket: not Copyable { ... }
```

**Alternatives considered**:
- `@linear` attribute (7 chars, but new concept)
- `@unique` attribute (7 chars, intuitive)
- `@move` attribute (5 chars)

**Decision**: `not Copyable` is verbose but self-documenting and consistent with the protocol system.
