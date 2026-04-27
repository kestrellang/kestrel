# Version 0.16 Scope: Boxing & Existentials

Scoping notes for the four 0.16 features. Captures difficulty, ambiguity, and
decisions made so far. Each feature still owes a real design doc before
implementation.

## Features at a glance

| Feature             | Difficulty   | Ambiguity      | Status            |
| ------------------- | ------------ | -------------- | ----------------- |
| Indirect enums      | Easy-Medium  | Low            | Ready to scope    |
| Existentials        | Hard         | Low (resolved) | Ready to scope    |
| Opaque types        | Medium       | Medium         | Field rule open   |
| Escaping closures   | Medium       | Medium         | Parked            |

Suggested implementation order: indirect enums → opaque types → escaping
closures → existentials. Each builds infrastructure (heap-boxing, vtables,
Rc'd environments) the next benefits from.

## Indirect enums

Smallest item. Parser already accepts `indirect case`; semantic model and type
system are done. Remaining work is MIR/codegen: heap-box payloads of
`indirect` cases via `GlobalAllocator`, free in the enum's drop path.

Open: niche optimization (null-tag the box pointer?). Defer.

## Opaque types (`some Protocol`)

Position determines semantics:

- **Return position** — caller-opaque, callee-known. One concrete type per
  function (unified across all `return`s).
- **Parameter position** — sugar for a generic. `func f(x: some P)` ≡
  `func f[T: P](x: T)`. Static dispatch, monomorphized.
- **Protocol requirement** — sugar for an associated type.
  `var x: some P { get }` ≡ `type X: P; var x: X { get }`.
- **Field position** — like return: external code sees `some P`, internal
  code sees the concrete type. **Open question:** what pins the concrete
  type? Recommended rule: a single concrete type per struct definition,
  inferred from the field's default initializer or required to match across
  all `init`s. Diagnose conflicts. Alternative (each `init` pins
  independently) makes the struct implicitly generic — probably not what we
  want.

Edge case to pin: `func f[T](x: T) -> some P` — does each `T` get its own
opaque type, or one opaque per `f`? Swift goes per-`T`. Match that.

## Escaping closures

Parked. Captures live in `Rc<Env>`, which collapses most of the open design
space:

- "Always box on escape" is fine — Rc handles cycles.
- `not Copyable` captures clone the Rc.
- Recursive escaping closures work for free (Weak references are a 0.20
  problem).

Remaining call: is escaping a *type* property (`@escaping (Int) -> Int`) or
inferred per-use? Type-level matches Swift and is more honest; inferred is
ergonomic. Pick one before writing code.

## Existentials (`any Protocol`)

Boxed via `GlobalAllocator`, vtable carries drop/size/align + protocol
methods. Non-Copyable by default; `any P: Cloneable` requires `P: Cloneable`.

Decisions:

- **Implicit coercion:** `T → any P` anywhere a `P` is expected.
- **Associated types must be constrained:** `any Iterator[Item = X]`.
- **Generic protocols must be constrained:** `any Container[Int]`.
- **Composition:** `any P & Q` is in scope.
- **Downcasting (`as?`):** out of scope for 0.16. Defer to 0.20 when classes
  bring RTTI.
- **Self-referential methods** on `any P` (e.g. `func eq(other: Self)`):
  forbid `any P` when `P` has Self-position requirements (Swift pre-5.7
  rule). Without RTTI we can't soundly dispatch them. Relax later.

Deferred: inline small-value optimization in the box (always heap for v1).
