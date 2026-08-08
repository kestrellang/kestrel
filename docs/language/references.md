# References

References (`&T`) let a function hand out direct access to a value it owns — an array element, a struct field — without copying it and without transferring ownership. Kestrel references are deliberately *second-class*: they name a place, they are created only in a few well-defined positions, and they never outlive the value they point into. There are **no lifetime parameters and no annotations** — the compiler tracks where every reference comes from (its *root*) and rejects any use that could dangle, with a plain diagnostic instead of a lifetime puzzle. If your code compiles, no reference in it dangles.

## Taking a Reference

The borrow operators `&` and `&mutating` appear in exactly one expression position: as the initializer of a `let` binding.

```kestrel
var x: Int64 = 10;

let r = &x;             // shared reference: read-only view of x
println("value: \(r)"); // reads through the reference

let m = &mutating x;    // mutable reference: requires x to be a `var`
m += 5;                 // writes through the reference
println("x is now \(x)");  // 15
```

Rules for borrow expressions:

- Only in a `let` initializer. `var r = &x` is rejected (E209) — the *binding* is immutable even when the referent is mutable.
- `&mutating` requires a mutable place; borrowing a `let` mutably is E210.
- You can only borrow named places. Borrowing a temporary (`let r = &5;` or `let r = &makeThing();`) is E499 — there is no value for the reference to outlive.
- Anywhere else, `&expr` is rejected (E488). In particular you never write `&` at a call site: **arguments borrow by signature**. A parameter `x: T` already borrows, and `mutating x: T` mutably borrows — see [Functions](functions.md).

## Reference Types

`&T` is a shared reference type and `&mutating T` is a mutable reference type. A `&mutating T` coerces to `&T`, never the other way. Reference types may appear in:

- **Function and method return types** — `-> &Int64`, `-> &mutating T`
- **Struct fields and tuple elements** — `var item: &Int64;`, `(&Int64, Int64)`
- **Generic arguments** — `Optional[&T]`, iterator `Item` types

They may **not** appear in:

- **Parameter types** (E480). Spell the convention instead: `x: T` borrows, `mutating x: T` mutably borrows. This is a permanent design decision, not a gap.
- **`let`/`var` type annotations** (E482). Write `let r = &x;` and let the type be inferred.
- **Subscript return types** (E481) — subscripts use `ref` accessors instead (below).
- **Function types** (E486) — `() -> &Int64` as a value is rejected.
- Nested references (`&&T`, `&mutating &T`) are rejected (E487).

## Ref-Returning Functions and Methods

A function or method may declare a reference return type. The body's result is a *place expression* — you name the place to lend out, with no `&` needed:

```kestrel
struct Box {
    var value: Int64;

    // lend read access to a field
    func peek() -> &Int64 {
        self.value
    }

    // lend write access; requires a mutating receiver
    mutating func slot() -> &mutating Int64 {
        self.value
    }
}
```

Callers can read, write, and compound-assign through the call directly:

```kestrel
var b = Box(value: 41);
println("peek: \(b.peek())");

b.slot() += 1;     // read-modify-write in place
b.slot() = 100;    // plain store through the reference
```

The returned reference must be rooted in something that outlives the call:

- **Methods** root at the receiver — always fine.
- **Free functions** must have exactly one non-`consuming` parameter the reference can root at; with two or more candidates the declaration is ambiguous (E493).
- Returning `&mutating` requires a mutable root: a `mutating` receiver or parameter (E495 otherwise), and a reference can never root at a `consuming` parameter — the value is destroyed when the call returns (E496).

## `ref` Accessors for Properties and Subscripts

Subscripts and computed properties never *declare* a reference type — the declared type stays `T`. Instead of `get`/`set` (which copy in and out), they can supply `ref` / `mutating ref` accessor bodies that lend the place directly:

```kestrel
struct Buffer {
    var storage: Pointer[Int64];

    subscript(at index: Int64) -> Int64 {
        ref { self.storage.offset(by: index).value }
        mutating ref { self.storage.offset(by: index).mutatingValue }
    }
}

// buf(at: 0) = 5;   buf(at: 0) += 2;   — all in place, no copies
```

`ref` provides reads (borrowing receiver), `mutating ref` provides writes (mutating receiver). You can also mix: a `get` for reads plus a `mutating ref` for in-place writes. A member with only `ref` is read-only — assigning through it is E201/E207. Duplicate providers are rejected (`get`+`ref` is E619, `set`+`mutating ref` is E620, a write provider without any read provider is E622), and ref accessors are not allowed in protocols (E621).

Types that only define `get`/`set` still support `+=` and friends through an automatic get → modify → set *writeback*, so `ref` accessors are an optimization and aliasing choice, not a requirement. See [Subscripts](subscripts.md) and [Computed Properties](computed-properties.md).

## Reference Decay

In a *value context* — anywhere an owned value is expected — a reference quietly copies its target out ("decays") instead of erroring. This makes ref-returning APIs pleasant to consume:

```kestrel
var b = Box(value: 41);

let copied = b.peek();  // binding a call result decays: `copied` is an owned Int64
b.slot() = 7;
println("\(copied)");   // still 41 — a copy, not a view
```

Decay applies to: bindings of ref-returning call results, assignment right-hand sides, `return`ed values in owned positions, `match`/`if` result merges, `??` right-hand sides, string interpolation holes, and array/tuple/dictionary literal elements. Note the asymmetry: `let r = &x;` *holds* a reference, while `let v = b.peek();` *copies* — holding requires the explicit `&`.

Decay is a copy, so decaying a reference to a non-`Copyable`, non-`Cloneable` value is rejected as a move out of a borrow (E503).

There is no decay between `Optional[&T]` and `Optional[T]`: construction is type-driven. An unannotated `Optional.Some(r)` snapshots the pointee (giving `Optional[Int64]`); with an `Optional[&T]` annotation the reference itself is stored.

## References in Structs and Tuples

A struct field or tuple element may hold a reference:

```kestrel
struct Cursor {
    var item: &Int64;
    var count: Int64;
}

var a: Int64 = 1;
let ra = &a;
let c = Cursor(item: ra, count: 3);
println("through field: \(c.item)");
```

Wrapping a reference into an aggregate taints the aggregate with the reference's root: a `Cursor` is subject to the same escape rules as the reference it carries, and it cannot be stored in globals, statics, or other places that outlive its root. Filling a `&T` slot requires an actual reference — an owned value does not auto-borrow into a ref-typed tuple element or field.

## Conformances: `extend &T: P`

Reference types can be extension targets, so references participate in protocol dispatch. The standard library ships **`Equatable` and `Comparable` for both `&T` and `&mutating T`** (when the pointee conforms) — comparisons go through to the pointee:

```kestrel
var a: Int64 = 1;
let ra = &a;
let rb = &a;
println("\(ra == rb)");  // true — compares pointees, Optional[&T] == also works

let xs = [10, 20, 30];
match xs.refs().min() {                    // needs &T: Comparable
    .Some(m) => println("min: \(m)"),      // min: 10
    .None => println("empty")
};
println("\(xs.refs().contains(20))");      // true — needs &T: Equatable
```

You can conform references to your own protocols the same way the stdlib does:

```kestrel
extend &T: MyProtocol where T: MyProtocol {
    public func describe() -> String { self.describe() }  // receiver peels to the pointee
}
```

Mutability is exact: `&mutating T` does not inherit conformances declared for `&T`; each spelling conforms only via its own extension. `Hashable` and `Matchable` for references are not yet supported and reject cleanly.

## Standard Library Surface

- **`Array.refs()`** — iterator of shared `&T` over the elements, in place, no copies. Writing through an item is rejected (E208).
- **`Array.mutableRefs()`** — mutating iterator of `&mutating T`; runs the copy-on-write barrier once up front, then yields in-place slots:

  ```kestrel
  var xs = [1, 2, 3];
  for x in xs.mutableRefs() {
      x += 10;
  }
  // xs is now [11, 12, 13]
  ```
- **`Array(at: i)`** — the labeled `at:` subscript lends elements via `ref`/`mutating ref` accessors.
- **`Pointer.value` / `Pointer.mutatingValue`** — the low-level bridge from raw pointers to references. Pointer-derived references are *your* contract: the compiler does not verify them.

## The Escape Rule (E494) — No Lifetimes

Every reference has a *root*: the parameter, receiver, or `Pointer` it was ultimately derived from. The one rule is: **a reference must not outlive its root.** The compiler tracks provenance through field accesses, struct wrapping/unwrapping, `Optional[&T]` and other carriers, and closures — and rejects escapes at compile time:

```kestrel
func bad() -> &Int64 {
    let x: Int64 = 42;
    x   // error[E494]: cannot return this reference: it borrows local `x`,
        // which does not outlive the call
}
```

The same rule catches indirect escapes:

- Returning a field of a local, or a local laundered through a ref-carrying struct field (`Cursor(item: &local)`) — E494 ("this value: it carries a reference that borrows…").
- A returned **view-kind** closure (normal or `mutating`) capturing anything local — E494 ("this closure: it captures…"). Owning kinds (`escaping` / `consuming`) snapshot their captures and are returnable; see [Closures](closures.md).
- Wrapping a reference to a local into `Optional[&T]` from an iterator's `next()` — E494.

Related enforcement: a reference cannot stay live across a conditional block merge or loop back-edge (E497 — hoist it into a binding first), and the owner cannot be consumed while a reference into it is live (E498). A view-kind closure *may* capture a named ref binding — its environment is frame-bound, so the view cannot outlive the borrow — but an owning closure cannot (E624), and a place viewed by a live closure is frozen against destruction (E507). `return Pointer(to: local).value` earns a warning (E504) rather than an error, because pointer-derived references are unverified by contract.

That's the entire model. There are no lifetime parameters, no `'a`, no borrow annotations on types or functions — and there never will be; storable long-lived references were rejected permanently in the design. Provenance is inferred from the code you already wrote.

## Not Yet Supported

- **Dictionary reference APIs** and `arr(checked: i) -> Optional[&T]` — planned, pending dictionary storage work. Interim: `Dictionary.modify(key) { (mutating v) in ... }` for in-place updates.
- **`Hashable`/`Matchable` conformances for `&T`** — reject cleanly for now.
- **Storable references** (escaping the root's scope, lifetime annotations) — permanently out of scope by design.

## Diagnostics You May Hit

| Code | Meaning |
|---|---|
| E480 | Reference type in parameter position — spell the convention (`x: T` borrows, `mutating x: T`) instead |
| E481 | Declared reference return on a subscript — use `ref`/`mutating ref` accessors |
| E482 | Reference type in a `let`/`var` annotation — write `let r = &x;` unannotated |
| E486 | Function type with a reference return (`() -> &Int64`) used as a value |
| E487 | Nested reference (`&&T`, `&mutating &T`) |
| E488 | Borrow expression outside a `let` initializer (e.g. `f(&x)` at a call site) |
| E493 | Ambiguous borrow source: free function returning `&T` with multiple candidate parameters |
| E494 | Escape: returned reference/carrier/closure is rooted at a local that dies with the call |
| E495 | Returning `&mutating` from a non-mutable root |
| E496 | Returned reference rooted at a `consuming` parameter |
| E497 | Reference held live across a block merge or loop back-edge |
| E498 | Owner consumed while a reference into it is live |
| E499 | Borrow of a temporary (`&5`, `&makeThing()`) |
| E201 | Assignment to a read-only place (e.g. a `ref`-only member) |
| E207 | Mutating operation through a shared `&T` |
| E208 | Plain assignment through a shared reference |
| E209 | Ref binding declared `var` — ref bindings must be `let` |
| E210 | `&mutating` borrow of an immutable place |
| E211 | `&` binder pattern outside a match arm |
| E212 | *Retired* — view-kind closures may capture ref bindings; an owning (`escaping`/`consuming`) closure that tries is E624 |
| E503 | Decay would move a non-Copyable value out of a borrow |
| E504 | *Warning:* returning a pointer-derived reference to a local of the same function |
| E505 | Module-level/static value of a non-storable (reference-carrying) type |
| E619–E622 | Malformed `ref` accessor combinations (duplicate read/write provider, ref accessor in a protocol, write-only member) |

## See Also

- [Functions](functions.md) — parameter access modes (`borrowing`/`mutating`/`consuming`), which replace reference parameters
- [Subscripts](subscripts.md) and [Computed Properties](computed-properties.md) — `get`/`set` vs `ref` accessors
- [Structs](structs.md) — copy vs move semantics of the values references point into
- [Extensions](extensions.md) — the `extend &T: P` conformance mechanism
- [Closures](closures.md) — capture rules
