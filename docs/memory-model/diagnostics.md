# Move and Escape Diagnostics

Two compiler analyses back the memory model's guarantees: the **move checker**
(flow-sensitive, over function bodies) and the **escape checker** (provenance-based,
in MIR verification). This page catalogs their user-facing diagnostics.

## The Move Checker

The move checker tracks, per control-flow path, whether each non-Copyable
binding still owns its value. A value is moved by: assignment to another
binding, a `consuming` argument or receiver, storing into an aggregate literal
(struct/enum/tuple/array), being captured whole into an **owning** closure
environment (`consuming` / `escaping` — view kinds capture by address and move
nothing), or an explicit `deinit x;` statement. Moved `var`s may be
reinitialized by assignment.

### E500 — `use_after_move`

Any use of a binding after a move on **every** path reaching the use:

```kestrel
let c1 = Connection(handle: 1);   // Connection: not Copyable
let c2 = c1;
print(c1.handle);                 // ERROR(E500)
```

### E501 — `maybe_moved`

A use reachable from a move on **some but not all** paths (including loop
back-edges — a move inside a `while` body flags uses on the next iteration):

```kestrel
let c = Connection(handle: 1);
if flag { use(c); }               // moves c on this path
look(c);                          // ERROR(E501): may have been moved
```

### E503 — `move_out_of_borrow`

Moving a non-Copyable value out of storage you only borrow — a borrowed
parameter's field, a borrowed `self`'s field, a `match` payload of a borrowed
scrutinee, or the pointee of a `&T` reference:

```kestrel
func leak(w: Wrap) -> Connection {   // w is borrowed (default mode)
    w.inner                          // ERROR(E503)
}
```

The legal counterpart is moving a field out of **`consuming self`** (or a
consuming parameter), which destructures the owned value — see
[copy-semantics.md](copy-semantics.md).

### E506 — `move_captured_out_of_closure`

A multi-call closure holds each captured value once, so a non-Copyable capture
may be borrowed inside the body but never moved out (returned as the body's
value, or passed to a consuming parameter):

```kestrel
let r = Res(id: 1);
let f = { () in consume(r) };   // ERROR(E506)
```

E506 is lifted inside a **`consuming`** body — a one-shot closure may move its
captures out. Owning capture (`consuming` / `escaping`) also moves the original
into the environment, so a later use of `r` in the enclosing scope is a plain
E500; view capture moves nothing and only *freezes* the place (E507). See
[closures.md](closures.md).

## The Escape Checker (Provenance)

Kestrel has no lifetime annotations. Instead, every reference (`&T` /
`&mutating T`) and every closure carries a **provenance root** — the storage
its validity depends on:

- a **parameter** (or borrowed receiver): outlives the call → returnable;
- a **local** (or temporary): dies at return → must not escape;
- `Pointer`-derived: unsafe, programmer-asserted;
- for **view-kind closures** (normal / `mutating`), the join of all captures'
  roots — frame-bound. An **owning** closure (`consuming` / `escaping`) holds
  self-rooted snapshots and is returnable; a capture-free closure has no root
  and escapes freely at every kind.

Provenance is tracked through bindings, field projections, struct
construction (a struct value carries the join of stored roots, recursively),
and control-flow merges — laundering a frame-bound root through a `let` or a
field does not clear it.

### E494 — escape error (the main rule)

Returning a reference, or a capturing closure, whose root is frame-bound:

```kestrel
func bad() -> &Int64 {
    let x = 42;
    x                    // ERROR(E494): root `x` dies at return
}

func makeAdder(n: Int64) -> (Int64) -> Int64 {
    { it + n }           // ERROR(E494): captures local `n`
}
```

For closures the diagnostic carries a fix-it note: spell an owning kind in the
return/expected type — `escaping (Int64) -> Int64` or
`consuming (Int64) -> Int64` — and the literal is rebuilt with owned captures.

Parameter-rooted references are fine — this is the supported accessor shape:

```kestrel
struct Person {
    var age: Int64
    func ageRef() -> &Int64 { self.age }   // OK: rooted at borrowed self
}
```

### The rest of the family

| Code | Rule |
|------|------|
| E495 | `-> &mutating T` must root at *mutable* storage (a `mutating` receiver/param) |
| E496 | a returned ref may not root at a `consuming` param/receiver (it dies in this call) |
| E497 | a ref *expression* used as a place may not stay live across a control-flow merge (v1 limit — hoist the control flow into a binding first; named `let r = &x` bindings thread across merges legally) |
| E498 | consuming a value while a borrow of it is still live |
| E499 | `&<rvalue>` — a borrow must name an existing place, not a temporary (`&5`, `&f()`) |
| E491/E492 | a ref-returning function is not a first-class value (cannot be stored, captured, or leak into inferred generic arguments) |
| E504 | *warning*: returning `Pointer(to: local).value` — the storage dies at return |
| E505 | globals/`static` members must have `Static` (reference-free) types |
| E507 | the closure freeze rule: destroying a place a live view-kind closure captures, or letting such a view outlive its referent (E498's closure analogue) |
| E212 | *retired* — view-kind closures may capture named ref bindings; only owning kinds still reject them (E624) |

Positions where reference *types* may appear at all (returns, bindings,
fields yes; parameters, function types, annotations no) are cataloged in
[limitations.md](limitations.md).
