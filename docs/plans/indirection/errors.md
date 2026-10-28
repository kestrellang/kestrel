# Indirection — diagnostics

> What the shipped feature emits. The receiver-only rule (semantics R6) means
> there is **no new coercion site**, so there is almost nothing new to diagnose
> — and no new diagnostic *code* was allocated.

## D1 — Member found on neither wrapper nor pointee

When `w.m` misses on `W` and the peel into `T` also misses, the access surfaces
as the **ordinary member-not-found** error, reported against the pointee type
the lookup ended on:

```
error: no member 'foo' on type 'Account'
  --> file.ks:12:7
12 |     acct.foo
   |          ^^^ no member 'foo' on type 'Account'
```

(`acct: RcBox[Account]`; the peel reached `Account`, which also lacked `foo`.)

This is what `not_found_names_pointee.ks` pins. Naming **both** the wrapper and
its pointee ("not found on `RcBox[Account]`, nor on its pointee `Account`") is a
wording improvement that is **not yet shipped** — the current message names the
pointee only. No code change, just `member_not_found_error` wording, when it
lands.

## D2 — Write through a read-only `Indirection`

Writing / RMW / mutating-method use through a wrapper that conforms to
`Indirection` but **not** `MutableIndirection` (no `pointeeMutRef`) reuses the
existing **E208 `assign_through_shared_ref`**:

```
error: cannot assign through a shared reference [E208]
  --> file.ks:30:5
30 |     ro.balance = 5;
   |     ^^^^^^^ ...
```

Rationale: the pointee is reached via `pointeeRef()`, which yields a `&T` place —
and "you can't write through a `&T` place" is **exactly** what E208 already
means. A read-only Indirection write is the same class, so it shares the code
rather than minting `indirection_no_mutating_accessor`. Implemented in the
`Field` arm of `assignment.rs`, gated on `indirection_peels[target]` ending with
`mut_method == None`, checked before the settable check (the pointee field *is*
settable, so the plain check would let it through).

Fires at the **point of use** (the write site), not at the conformance
declaration — a read-only `Indirection` is perfectly legal.

## What does NOT get a diagnostic

- **Shadowing (`m` on both `W` and `T`)** — not an error. Wrapper-wins is total
  (R2); reach the pointee member with `.pointeeRef()`. No `IndirectionAmbiguous`.
- **Passing `w` where `T` is expected** — ordinary type mismatch (R6); the peel
  adds no coercion, so there is no Indirection-specific message. (A "did you mean
  `w.pointeeRef()`?" hint is deliberately **not** added — it would imply the
  coercion is "almost allowed," exactly the impression R6 avoids.)
- **Conforming a non-pointer-like type to `Indirection`** — not enforceable, not
  diagnosed. Intent is documentation, like Rust's `Deref` guidance.

## Summary

| Diagnostic | Code | Trigger | Mechanism |
|---|---|---|---|
| member-not-found (on pointee) | existing | `m` on neither `W` nor `T` | `member_not_found_error` (peel reaches pointee, reports there) |
| write through read-only | **E208** (reused) | mutate through `Indirection`-not-`Mutable` | `assignment.rs` Field arm |
