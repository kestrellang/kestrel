# Closure Fields Must Own Their Captures

Status: **proposal, for review** (2026-10-24). Extends [closures.md](closures.md).

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

## Holes this does not close

These need decisions before the escape check is sound for closures:

1. **Call results.** `register(f: { k })` returning a struct that holds `f`.
   Under rule 1 the field must be `escaping`, so the literal snapshots `k`
   and nothing dangles. Closed by rule 1, provided the parameter's type
   converts only at the literal (a view-closure *variable* passed to an
   `escaping` parameter is already an E624 kind mismatch).
2. **Tuples.** A tuple element may be a view closure, and a `mutating`
   tuple parameter can store into it: `t.0 = f`. Proposal: tuples follow
   the same rule as Static (a tuple with a view-closure element is
   non-`Static`), and a `mutating` parameter whose type contains a view
   closure gets the existing scope-depth check (E507) against the caller's
   argument. Needs confirming.
3. **`mutating` closure-typed parameters.** `func set(_ slot: mutating () -> (), f: () -> ()) { slot = f }`
   writes a view into the caller's variable. The analyzer's call-argument
   outlives check (`check_call_arg_outlives`, E507) already treats `mutating`
   arguments as destinations; returning the caller's variable afterwards
   needs the same taint. Needs a test.

## Open questions

1. New error code for rule 1, or extend E624 (closure kind mismatch)?
2. Allow a view-closure field in a struct that is itself declared
   `not Static` (an explicit "this struct is a frame view")? Default: no,
   keep the rule simple.
3. Should holes 2 and 3 be part of this change, or follow it?
