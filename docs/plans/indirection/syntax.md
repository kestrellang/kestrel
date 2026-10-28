# Indirection — syntax

> **There is no new grammar.** Indirection reuses protocol declarations,
> associated types, `-> &T` method requirements (shipped stage-2d witness
> ref-returns), conformance extensions, and member access — all of which already
> parse. This file makes the "nothing new" explicit and pins the surface shape.

## Nothing new to parse

| Surface construct | Already exists |
|---|---|
| `protocol P { type T; func f() -> &T }` | protocol decl + assoc type + `-> &T` method requirement |
| `protocol Q: P { mutating func g() -> &mutating T }` | protocol refinement + `mutating` requirement |
| `extend W: MutableIndirection { … }` | conformance extension (conditional `where` too) |
| `w.m`, `w.f = v`, `w.f += 1`, `w.method()`, `w(i)` | member-shaped access |
| `w.pointeeRef().m` | method call + member on the `&T` result |

The feature is entirely in *resolution* (semantics.md) and *lowering*
(compiler-arch.md), not in the grammar.

## The protocols, verbatim

```kestrel
public protocol Indirection {
    type Target
    func pointeeRef() -> &Target
}

public protocol MutableIndirection: Indirection {
    mutating func pointeeMutRef() -> &mutating Target
}
```

- `Target` — the pointee type (semantics "T"). Named `Target` (not `Pointee`)
  so it reads as `W.Indirection.Target`.
- `pointeeRef` / `pointeeMutRef` — the accessors. `pointeeMutRef` lives on the
  `MutableIndirection` refinement so a read-only wrapper can omit it (no
  "optional requirement" feature needed), and is `mutating` so a COW wrapper can
  fork before yielding the mutable view.

### Why a method pair, not `var pointee { ref mutating ref }`

The original design proposed a place-accessor *requirement*. Protocol-level
`ref` / `mutating ref` requirements are rejected by **E621**
(`kestrel-analyze/src/decl/place_accessor.rs`) — protocols use `get`/`set`. The
method pair sidesteps it, and costs nothing because the peel is always concrete
(compiler-arch Layer 2): the accessors are never witness-dispatched, so the
requirement never needs to be a place accessor. See the README for the full
rationale.

## Conformance shapes (all parse today)

```kestrel
// Full (read + write)
extend RcBox[T]: MutableIndirection {
    type Target = T
    public func pointeeRef() -> &T { self.valuePtr().value }
    public mutating func pointeeMutRef() -> &mutating T { self.valuePtr().mutatingValue }
}

// Conditional (COW barrier in the mutating half)
extend CowBox[T]: MutableIndirection where T: Cloneable {
    type Target = T
    public func pointeeRef() -> &T { self.valuePtr().value }
    public mutating func pointeeMutRef() -> &mutating T {
        if self.inner.isUnique() == false {
            self.inner = RcBox(self.inner.getValue().clone())
        }
        self.valuePtr().mutatingValue
    }
}

// Read-only — `Indirection` only; writes through it are E208 (D2)
extend RefView[T]: Indirection {
    type Target = T
    public func pointeeRef() -> &T { self.borrow() }
}
```

## Use sites (all parse today)

```kestrel
let acct = RcBox(Account(balance: 0));
acct.balance              // field read   → pointeeRef
acct.balance = 100        // field write  → pointeeMutRef  (acct must be `var`)
acct.deposit(100)         // mutating method → pointeeMutRef
acct.describe()           // non-mutating method → pointeeRef
acct.pointeeRef()         // the accessor itself (explicit reach-through)
acct.pointeeRef().clone() // force the pointee's member (R3)
```
