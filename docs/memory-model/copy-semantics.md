# Copy Semantics

Kestrel's copy semantics prioritize ergonomics for application developers while providing escape hatches for systems programming.

## The Classification

Every type has exactly one of three copy classes:

| Class | Meaning |
|-------|---------|
| **Copyable** | Duplicated by bitwise copy; uses never invalidate the value |
| **Cloneable** | Duplicated by calling `clone()`; still usable like a Copyable type |
| **NotCopyable** | Cannot be duplicated; assignment and consuming passes are moves |

The classification is computed by a single decision tree shared by every compiler stage (the `kestrel-copy-fold` crate). For aggregates it is a fold over the members: **NotCopyable dominates; otherwise any Cloneable member makes the aggregate Cloneable; otherwise it is Copyable.**

## Implicit Copyable

A `struct` or `enum` is automatically `Copyable` if **all** its fields are `Copyable`:

```kestrel
struct Point {
    var x: Int64
    var y: Int64
}
// Point is implicitly Copyable (Int64 is Copyable)

let p1 = Point(x: 1, y: 2);
let p2 = p1;  // Copy
print(p1.x);  // OK: p1 is still valid
```

### Built-in Copyable Types

- All integer types (`Int8` … `Int64`, `UInt8` … `UInt64`, and the `Int`/`UInt` aliases)
- All floating point types (`Float16`, `Float32`, `Float64`, `Float`)
- `Bool`, `Char`
- `lang.ptr[T]` raw pointers (copying copies the address)
- References `&T` / `&mutating T` (copying copies the alias, never the pointee)
- Tuples of Copyable types (the fold above)

Heap-owning stdlib types (`String`, `Array`, `Dictionary`, …) are **Cloneable**, not bitwise-Copyable — copies go through `clone()` (see [cloneable.md](cloneable.md)). Generic containers such as `Optional[T]` and `Array[T]` follow their arguments per-instantiation (see [generics.md](generics.md)).

## Implicit NotCopyable

If a type contains a non-copyable field, it automatically becomes non-copyable:

```kestrel
struct FileHandle: not Copyable {
    var fd: Int64
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
    var id: Int64
    var seat: String
}

let t1 = Ticket(id: 1, seat: "A1");
let t2 = t1;  // MOVE, not copy
// t1 is now invalid
print(t1.id); // ERROR(E500): use of moved value
```

Declaring both `Cloneable` and `not Copyable` on one type is a conflict (E423).

### Use Cases for Explicit `not Copyable`

1. **Unique resources**: Tickets, tokens, capabilities
2. **RAII wrappers**: File handles, mutex guards, connections
3. **Enforcing single ownership**: Preventing accidental aliasing
4. **Performance**: Avoiding copies of large structs

## Move Semantics

For `not Copyable` types, assignment and `consuming` parameter passing are **moves**:

```kestrel
struct Connection: not Copyable {
    var handle: Int64
}

let c1 = Connection(handle: 42);
let c2 = c1;  // Move
// c1 is invalid after this point

func use(consuming conn: Connection) { }

let c3 = Connection(handle: 43);
use(c3);  // Move into function
// c3 is invalid after this point
```

Moves are tracked flow-sensitively by the move checker. Using a value after a definite move is **E500** (`use_after_move`); using it after a move on only *some* control-flow paths is **E501** (`maybe_moved`). Moving into aggregate literals (`[c1]`, `(c1, x)`, `Wrapper(file: c1)`, `.Some(c1)`) counts as a move of the operand. A moved `var` can be **reinitialized** by assigning a fresh value, after which it is usable again. See [diagnostics.md](diagnostics.md) for the full catalog.

## Field Move-Out

Whether you may move a non-Copyable field *out* of a struct depends on how you hold the struct:

**Out of a borrowed value — rejected (E503).** A borrow does not own the value, so extracting a non-Copyable field would either alias it or steal it from the owner:

```kestrel
struct Wrap {
    var inner: Connection
}

func leak(w: Wrap) -> Connection {
    w.inner   // ERROR(E503): cannot move out of a borrow
}
```

**Out of `consuming self` — legal.** The method owns the whole value, so it may destructure it: the requested field is moved out, the *sibling* fields are dropped, and the whole-value `deinit` does not run again for the moved field:

```kestrel
struct Wrap: not Copyable {
    var inner: Connection
    consuming func intoInner() -> Connection { self.inner }
}

let w = Wrap(inner: Connection(handle: 1));
let c = w.intoInner();   // OK — exactly one Connection exists afterwards
```

The same applies to consuming free-function parameters. Reading a *Copyable* field through a borrowed non-Copyable container is always fine — only the non-Copyable payload itself cannot leave a borrow.

## Copyable and Cloneable as Protocols

`Copyable` is a real protocol — a marker protocol in the standard library, tagged for the compiler:

```kestrel
// lang/std/core/copy.ks
@builtin(.Copyable)
public protocol Copyable {}

@builtin(.Cloneable)
public protocol Cloneable: Copyable {
    @builtin(.Clone)
    func clone() -> Self
}
```

Conformance is synthesized implicitly by the classification fold; `not Copyable` is the opt-out. Because it is a protocol, it participates in bounds:

```kestrel
func copyIt[T](value: T) -> T where T: Copyable {
    value   // relies on copy
}
```

Generic parameters are `Copyable`-bounded **by default**; see [generics.md](generics.md) for opting out with `not Copyable` and for conditional conformance (`extend Box[T]: Copyable where T: Copyable`).

---

## Design Notes

### Transitive NotCopyable Is a Semver Hazard

Adding a non-copyable field to an existing struct silently changes its semantics:

```kestrel
// Before: Copyable
struct Config {
    var name: String
    var timeout: Int64
}

// After: NOT Copyable (breaking change!)
struct Config {
    var name: String
    var timeout: Int64
    var logFile: FileHandle  // Makes Config non-copyable
}
```

This is accepted as intentional: adding a resource field *should* change semantics. Library authors should treat a field's copy class as part of their public API.

### Copies in Generic Code Are Implicit

```kestrel
func duplicate[T](item: T) -> (T, T) {
    (item, item)  // Two uses of item — implicit copies
}
```

This works because `T: Copyable` is the default bound. Developers should be aware that copies (or `clone()` calls, for Cloneable arguments) happen silently in generic code.

### Copyable Does Not Mean Cheap

```kestrel
struct BigData {
    var items: Array[Int64]  // 10,000 elements
}
// BigData is Cloneable (Array is), and copying it copies 10,000 integers
```

There is no compiler-enforced size limit; expensive-copy detection is a linting concern, not a semantic one.

### `not Copyable` Spelling

`not Copyable` was chosen over attribute spellings (`@linear`, `@unique`, `@move`) because it is self-documenting and composes with the protocol system — the same `not` form is used for bounds (`where T: not Copyable`) and for the `Static` protocol (`where T: not Static`, see [limitations.md](limitations.md)).
