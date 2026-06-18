# Indirection — semantics

> The behavioural rules of the shipped feature. `compiler-arch.md` says where in
> the compiler; this says what the program *means*. Terms: **W** = the wrapper
> type (`W: Indirection`), **T** = `W.Indirection.Target` (the pointee),
> **`m`** = the accessed member.

## R1 — Receiver peel (the whole feature)

For a member access `w.m` where `w: W`:

1. Resolve `m` against `W` (instance members, then static members).
2. **If found on `W`** — use it. `W` wins. The peel never fires.
3. **If not found on `W`** and `W: Indirection` — resolve `m` against `T`
   instead, as if the access were `w.pointeeRef().m`. The access is lowered
   through `pointeeRef()` (reads) or `pointeeMutRef()` (writes).
4. **If not found on `T` either** — error (R8); if `T: Indirection`, recurse
   into `T`'s pointee first (R5).

This applies to **direct member access**: field read, field write, method call,
paren subscript `w(i)`, compound assignment `w.f += 1`, and string-interpolation
member access. It does **not** apply to operator / for-in / try forms, which are
`ProtocolCall`s and are R7's domain (the one clarification vs. the original
proposal — they funnel through the same solver seam but are explicitly excluded
from the peel).

## R2 — Wrapper-wins (no ambiguity error)

When `m` exists on **both** `W` and `T`, `W` wins **unconditionally** — step 2
short-circuits before the peel is considered. This is deliberate, not an
ambiguity:

```kestrel
acct.clone()             // RcBox.clone — refcount bump (W wins)
acct.pointeeRef().clone() // Account.clone — deep copy (explicit reach)
```

The wrapper's own identity (refcount semantics, COW, raw-pointer arithmetic)
must never be silently shadowed by a same-named pointee member. The escape hatch
is **always** `.pointeeRef()` (R3) — never a disambiguation syntax, never an
ambiguity error the user must resolve. This is why no `IndirectionAmbiguous`
diagnostic exists: the rule is total.

## R3 — `.pointeeRef()` is the explicit reach-through

`w.pointeeRef()` names the pointee view (`&T`). It is an ordinary method on `W`,
so it resolves at step 2 and never triggers the peel; member access on the
resulting `&T` then peels *eagerly* (the built-in ref arm) to `T`. So
`w.pointeeRef().m` always forces *`T`'s* `m`, even when `W` also has an `m` —
the total escape hatch for R2 shadowing. It is Kestrel's spelling of Rust's
`Rc::clone(&x)` idiom, as plain method access.

## R4 — Mutability routing

The access selects the accessor by **how the member is used**, not how it's
declared:

| Use of `w.m` | Accessor | Requirement |
|---|---|---|
| read (`let x = w.m`, rvalue, non-mutating method) | `pointeeRef()` | `Indirection` |
| write (`w.f = v`) | `pointeeMutRef()` | `MutableIndirection` |
| RMW / compound (`w.f += 1`) | `pointeeMutRef()` | `MutableIndirection` |
| mutating-method receiver (`w.mutate()`) | `pointeeMutRef()` | `MutableIndirection` |

A wrapper that conforms only to `Indirection` (no `pointeeMutRef`) is a
read-only smart pointer: reads peel fine, any write/mutate is rejected (R8 /
D2). Because `pointeeMutRef` is a `mutating` method, the wrapper itself must be a
mutable place (`var`) to write through it — exactly like value semantics
elsewhere.

`CowBox` gets copy-on-write **for free**: its `pointeeMutRef` runs the COW fork
before yielding `&mutating T`, so `cow.field = x` (which selects `pointeeMutRef`)
forks shared storage while `let x = cow.field` (which selects `pointeeRef`) does
not.

## R5 — Bounded transitivity

If `m` is not on `T` and `T: Indirection`, the peel recurses: resolve `m`
against `T`'s `Target`, lowering through `T`'s accessor too. So
`RcBox[RcBox[Account]].balance` reaches `Account.balance` through two
`pointeeRef()` projections (the recorded peel **chain**, replayed outer→inner).

The chain follows the `Indirection` conformance and **stops at the first
non-`Indirection` pointee** — never peeling through an arbitrary type. Depth is
exactly the length of the chain. Each level still obeys R2 at that level.

## R6 — The line that never moves: arguments never coerce

The peel rewrites the **receiver** of a member access and **nothing** else:

```kestrel
f(w)          // f expects T  →  TYPE ERROR. Write f(w.pointeeRef()).
let t: T = w  // initialization  →  TYPE ERROR.
return w      // function returns T  →  TYPE ERROR.
```

There is no `W → T` decay anywhere outside receiver-position member access. This
is the half of Rust's `Deref` we keep; argument-position coercion is the half we
reject. Because nothing coerces, there is no new coercion site — only the
ordinary "type mismatch" the program would already get for any unrelated types.

## R7 — Operators and conformances do NOT ride the peel

Binary operators (`==`, `<`, `+`, …), hashing, `Formattable`, and every other
protocol conformance resolve by **conformance lookup on `W`**, not member
fall-through:

```kestrel
w1 == w2      // requires `extend W: Equatable where T: Equatable` — NOT the peel
"\(w)"        // requires `extend W: Formattable where T: Formattable`
```

Mechanically: operators desugar to `HirExpr::ProtocolCall`, which the peel
excludes (`protocol_dispatch_members`, compiler-arch Layer 2). The reason is R6:
an operator takes the other operand as an *argument*, so peeling `==` would
force the operand to peel too — argument coercion through the back door. The
asymmetry is observable and intended: `w.someEquatableMethod()` peels (direct
member access), but `w == other` does not (operator needing an argument). This
is the concrete boundary between "ergonomic access" and "implicit conversion."

## R8 — Errors

- **`m` on neither `W` nor `T`** — member-not-found, reported on the pointee
  type (D1).
- **write / mutating use through a read-only `Indirection`** — E208
  `assign_through_shared_ref` (D2): the pointee is reached via a `pointeeRef()`
  `&T` place, which can't be written.
- No ambiguity error (R2 is total). No coercion error (R6 — ordinary mismatch).

## R9 — Identity, copies, drops are unaffected

`Indirection` is an *access* protocol. It does not change `W`'s copy semantics,
move behaviour, or `deinit`:

- Copying `w` copies the **wrapper** (`RcBox` bumps the refcount, `CowBox`
  shares-then-COWs, `Pointer` bit-copies the address) — copies are not member
  accesses, so the peel is irrelevant to them.
- `w` drops as a `W` (its `deinit` runs). The peel never produces an owned `T`;
  `pointeeRef`/`pointeeMutRef` yield *views*, not owned values, so no ownership
  crosses the peel.
- A value read through the peel (`let x = w.field`) is a copy-out of that field,
  governed by the field's own copy semantics via the transparent-place rule —
  identical to reading the same field off a `&W`. NonCopyable fields stay
  non-copyable through the peel.

This is what "smart pointers are a composition, not one magic trait" means in
practice: `Indirection` is bolted onto the access path and is orthogonal to the
copy/drop/identity mechanisms that make something a smart pointer.
