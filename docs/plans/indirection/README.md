# Indirection — transparent member access for smart pointers

> **Status: ✅ SHIPPED 2026-06-16** (20/20 tests green, cranelift + llvm).
> This is the decision record — the *why*. The *how* is split across
> [`compiler-arch.md`](compiler-arch.md) (mechanism + real anchors),
> [`semantics.md`](semantics.md) (the rules R1–R9), [`errors.md`](errors.md),
> [`syntax.md`](syntax.md), and [`tests.md`](tests.md). Indirection is a single
> feature, so these live flat here, not under `stageN/` like `references/`.

## The gap it closed

Kestrel already had smart pointers — `RcBox[T]` (`memory/rcbox.ks`), `CowBox[T]`
(`memory/cowbox.ks`), and the raw `Pointer[T]` (`memory/pointer.ks`) — but they
exposed their pointee explicitly:

```kestrel
let r = RcBox(account);
r.getValue().balance          // or r.value.something on a Pointer
r.modify { (mutating a) in a.deposit(100) }
```

You can now write `r.balance` and `r.deposit(100)` directly: member access on
the wrapper reaches through to the pointee, the way every other language's
smart pointers behave. No manual `extend RcBox[T]` re-exposing each member, no
threading `.getValue()` / `.value` / `.modify` through every use site.

## Why this is *not* a new special case

`&T` was already a smart pointer with transparent member access. On a `&String`
you write `r.byteCount` — no `*`, no `.value` — because member resolution
**peels the ref** to the pointee. That peel is one arm in `solve_member`
(`kestrel-type-infer/src/solver.rs`): a `TyKind::Ref` receiver requeues a
`Constraint::Member` with the *pointee* as the receiver, so field / method /
operator / subscript / for-in / compound-assign all see `T`.

The only reason `RcBox[T]` didn't get that behaviour is that the peel was
hard-wired to the built-in `TyKind::Ref`. This feature **generalizes that one
arm** from "the built-in reference type" to "any type that opts in," via a new
`Indirection` protocol. It is the same mechanism `&T` already uses, lifted to a
user-extensible protocol — exactly the move `extend &T: P` made for ref
*conformances*. The difference: the `&T` peel is **eager** (a ref has no members
of its own); the `Indirection` peel is **lazy** (it fires only after the
wrapper's own members miss, so the wrapper always wins a name clash).

## The protocol

```kestrel
public protocol Indirection {
    type Target
    func pointeeRef() -> &Target
}

public protocol MutableIndirection: Indirection {
    mutating func pointeeMutRef() -> &mutating Target
}
```

A type opts into transparent access by conforming and returning a reference to
its pointee. **Reads** peel through `pointeeRef()` (`&Target`); **writes**, RMW,
and **mutating-method receivers** peel through `pointeeMutRef()`
(`&mutating Target`). A read-only smart pointer conforms to `Indirection` only;
writing through it is the D2 error. `pointeeMutRef` is `mutating` so a
copy-on-write wrapper can fork its storage before handing out the mutable view.

The stdlib conformers (`core/indirection.ks` + `memory/*.ks`):

```kestrel
extend Pointer[T]: MutableIndirection {
    type Target = T
    public func pointeeRef() -> &T { self.value }
    public mutating func pointeeMutRef() -> &mutating T { self.mutatingValue }
}

extend RcBox[T]: MutableIndirection {
    type Target = T
    public func pointeeRef() -> &T { self.valuePtr().value }
    public mutating func pointeeMutRef() -> &mutating T { self.valuePtr().mutatingValue }
}

extend CowBox[T]: MutableIndirection where T: Cloneable {
    type Target = T
    public func pointeeRef() -> &T { self.valuePtr().value }
    public mutating func pointeeMutRef() -> &mutating T {
        if self.inner.isUnique() == false {            // COW barrier — for free
            self.inner = RcBox(self.inner.getValue().clone())
        }
        self.valuePtr().mutatingValue
    }
}
```

`CowBox` gets copy-on-write **for free**: the barrier lives in its
`pointeeMutRef`, so `cow.field = x` triggers COW exactly like `cow.modify` does.

### Why a method pair, not `var pointee`

The original design proposed a place-accessor requirement,
`var pointee: Target { ref mutating ref }`. That **does not compile**:
protocol-level `ref` / `mutating ref` accessor requirements are rejected by
**E621** (`kestrel-analyze/src/decl/place_accessor.rs`) — protocols use
`get`/`set`, and witness-dispatched reference returns were out of scope for
stage 1.5.

The method pair sidesteps E621 entirely, and the key insight is that it costs
**nothing** in expressiveness: **the peel only ever fires on a concrete
receiver** (`solve_member` defers until the receiver type is known). So the
`pointeeRef`/`pointeeMutRef` accessors are always resolved and called
*concretely* (`Callee::Direct`) — never witness-dispatched. The protocol
requirement only needs to *exist* (to drive conformance + the `Target` assoc
type); it never needs to be a place accessor. And `-> &T` method requirements
already work — they're the shipped stage-2d witness-ref-return support. The
two-protocol split (`MutableIndirection` refines `Indirection`) gives the
optional mutating half a name to query without needing "optional requirements."

## The line that never moves

**The receiver peels. Arguments never coerce.**

- ✅ `r.foo`, `r.bar()`, `r.foo = 5`, `r.mutate()` — member access on the
  wrapper resolves through to the pointee.
- ❌ `f(r)` where `f` expects a `T` — **type error**, not a silent conversion.
  You write `f(r.pointeeRef())`.

This is the same split `&T` already enforces. Argument-position coercion is
*the* part of Rust's `Deref` that people regret (the invisible `&String`→`&str`
at call boundaries); member access is local and visible at the use site. We
take the member half and leave the coercion half — and because nothing coerces,
there is no new coercion site to diagnose.

Binary operators and protocol conformances (`==`, `<`, hashing, `Formattable`)
do **not** ride member fall-through either — they forward via ordinary
`extend RcBox[T]: Equatable where T: Equatable` (the established `extend &T: P`
pattern). Operators desugar to a `HirExpr::ProtocolCall`, and the peel
explicitly excludes those: routing `==` through the peel would reopen the "does
the *argument* also peel?" question — argument coercion through the back door.
Member fall-through only ever peels the **receiver**.

## Worked end-to-end

```kestrel
let acct = RcBox(Account(balance: 0));
acct.balance              // RcBox has no `balance` → peel → acct.pointeeRef().balance
acct.deposit(100)         // mutating method → acct.pointeeMutRef().deposit(100)
acct.clone()              // RcBox HAS clone → refcount bump (wrapper wins)
acct.pointeeRef().clone() // force Account.clone (deep copy) — the escape hatch
sendTo(acct)              // ERROR if sendTo wants Account — write sendTo(acct.pointeeRef())
acct == other             // via extend RcBox[T]: Equatable, NOT fall-through
```

## What this is explicitly NOT

- **Not implicit deref coercion.** No argument-position conversion, no `&T`-style
  decay for user wrappers. Receiver peel only.
- **Not automatic for every wrapper.** Conforming to `Indirection` is the opt-in;
  a struct with a `value` field gets nothing for free.
- **Not for code reuse / inheritance.** Documented (as Rust's own `Deref` docs
  are) as "for genuine pointer-like wrappers." Member fall-through for "I want to
  inherit `Array`'s API into my newtype" is a smell, not the use case.
- **Not operators.** `==`/`<`/hash/format forward via `extend ...: Protocol`.

## Relationship to references

This is the user-wrapper completion of the same idea the references work built
for `&T`:

- `&T` member access — peels **eagerly** in `solve_member`. **Shipped.**
- `extend &T: P` — ref *conformances* forward to the pointee via synthetic
  `lang.&` entities. **Shipped 2026-06-12.**
- `W: Indirection` member access — peels **lazily** for *user* wrappers.
  **Shipped 2026-06-16 (this doc).**

All three share one principle: **the pointee is reached by peeling the receiver,
never by coercing the language around it.** Smart pointers in Kestrel are a
composition (foundation `Pointer` + place accessor + `deinit` + copy-semantics +
`Indirection` for ergonomics), not one magic trait — and `Indirection` supplies
only the access ergonomics, deliberately not the coercion.

## Settled questions

- **Name `Indirection` / accessors `pointeeRef`/`pointeeMutRef`.** A smart
  pointer *is* a layer of indirection — literal, not metaphorical, and free of
  Deref-coercion baggage. The category-noun morphology follows the Swift
  `Sequence`/`Collection` lane. `pointeeRef` reuses the stdlib's `pointee` term.
- **Transitivity depth — chain through `Indirection`.** `RcBox[RcBox[T]].field`
  reaches `T` through two peels; the chain stops at the first non-`Indirection`
  pointee. Implemented and tested (`nested/two_levels.ks`).
- **Static members — instance only.** Forwarding a wrapper's *static* members to
  the pointee's statics is dubious; not done.
- **`&T` as an intrinsic `Indirection`.** The eager `TyKind::Ref` peel could in
  principle be re-expressed as `&T: Indirection` with `Target = T`, unifying the
  two arms. Left as a refactor-after-it-works; the two are the same shape (one
  eager, one lazy).
