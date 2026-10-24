# Closure Fields Must Own Their Captures

Status: **implemented** (2026-10-24). Extends [closures.md](closures.md).
Decisions: new code E626; no opt-out; holes 2 and 3 fixed in the same change.

## Problem

A view closure (normal or `mutating` kind) holds views into its defining
frame. The escape check (E494) follows that taint through copies, aggregate
construction, joins, and assignment into a `var`. It does not follow it
through a `mutating` call that stores the closure into its receiver:

```kestrel
struct Button {
    var onClick: () -> ()
    mutating func setHandler(h: () -> ()) { self.onClick = h }
}

func makeButton() -> Button {
    let count = 5
    var b = Button(onClick: {})
    b.setHandler({ print(count) })   // views `count`, now stored in b
    b                                // escapes: no diagnostic today
}
```

`setHandler` is legal on its own (its caller's frame outlives the call), and
the caller never learns that `b` now carries the view. The same shape stores
closures into generic containers (`self.hs.append(h)`, Perch's router).
Pinned by `view_closure_stored_via_mutating_method_then_returned_rejected.ks`.

## Decision

Two type-level rules. No new flow analysis.

### 1. A stored field cannot have a view-closure type

A struct or enum stored field whose type is a function type must be
`escaping` or `consuming`. Normal and `mutating` function types are rejected
at the field declaration (new error code, allocated in implementation).

```kestrel
struct Button {
    var onClick: escaping () -> ()   // required
}
```

An `escaping` literal owns snapshots of its captures, so storing, returning,
or copying a `Button` can never dangle. The `Button` example above is then
fine as written: `{ print(count) }` checks against `escaping () -> ()` and
copies `count`.

This reverses closures.md "What Changes" item 6 ("Structs may hold view-kind
closures in fields; the struct becomes frame-bound"). The provenance carry
for aggregates stays, for refs.

### 2. View closures are not `Static` (references stage 2c)

Function types are currently `Static` unconditionally. A normal or `mutating`
function type becomes non-`Static`, exactly like a type containing a ref, so
`Array`, `Dictionary`, `RcBox`, globals and every other `T: Static` position
reject it. This covers storage through type parameters, which rule 1 cannot
see (`var hs: [T]` instantiated at a closure type, `Optional[() -> ()]`
fields). `escaping` and `consuming` function types stay `Static`.

Rule 2 is the "view closures are values carrying refs" model: a view
closure is non-`Static` for the same reason `&T` is, and the existing
contains-ref machinery decides it.

## What stays cheap

Closures passed as **parameters** (`forEach`, `map`, `sort(by:)`, any
`f: (T) -> U`) are unaffected: still views, no allocation. Only *stored*
closures pay for an owned environment.

## Costs

- A short-lived helper struct holding a closure (`Logger(log: { print(prefix) })`)
  must write `escaping`: one heap allocation, captures copied, and the
  closure no longer sees later writes to `prefix`.
- Capture-free closures and plain functions stored in fields also need the
  spelling. A capture-free literal converts to `escaping` with no
  allocation, so only the annotation is new.

## Migration

- `lang/perch/src/router.ks`: two fields (`handler: (Request, T) -> Response`,
  lines 82 and 177) become `escaping`.
- `lang/std`: no stored view-closure fields found.
- Test suite: 19 testdata files declare a view-closure `var` field and need
  review (most should take `escaping`; tests that pin the old frame-bound
  behavior change to expect the new error).

## Related holes, closed in the same change

1. **Call results.** `register(f: { k })` returning a struct that holds `f`:
   the field must be `escaping` (rule 1), so the literal snapshots `k`.
2. **`mutating` closure / tuple arguments.** `func put(mutating slot: () -> (), f: () -> ()) { slot = f }`
   and `t.0 = f` through a `mutating` tuple parameter. MIR lowering
   (`taint_mutating_destinations`) joins the taint of every
   view-closure-carrying argument into the slot of each `mutating` argument
   whose type can hold one, the same join a direct store does. After rules
   1 and 2 only bare closures and tuples can be such a destination, so
   structs never pick up spurious taint. Returning the variable afterwards
   is E494; calling it locally is fine
   (`view_closure_stored_via_mutating_param_then_returned_rejected.ks`).
