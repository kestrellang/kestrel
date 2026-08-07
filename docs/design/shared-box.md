# The Shared Box

**Status: draft — companion to [closures.md](closures.md); answers its open
question 5 (the shared-object container).**

Several language features need the same thing: a value moved to the heap,
owned collectively by any number of handles, destroyed exactly once when the
last handle goes away. Escaping closures need it first. Classes, `any`
protocol existentials, and `indirect enum` payloads will need it next. This
document defines the mechanism once: a **`SharedBox` protocol** that states
the contract, a stdlib refcounting implementation (`RcBox`) as the default,
and a lang-item binding that lets the implementation be swapped — for a
tracing collector, an arena, an atomic refcount — without touching the
compiler's lowering of any client feature.

Design rules:

1. **Boxes are written entirely in Kestrel.** The compiler never emits
   retain/release/alloc sequences of its own; it instantiates a stdlib type
   and calls its methods.
2. **The compiler codes against the protocol, never a concrete type.** Every
   implicit-boxing site uses only `SharedBox` requirements, so one lang-item
   swap retargets every client feature at once.
3. **Lifecycle is ordinary value semantics.** Sharing is `clone()`,
   releasing is `deinit`. No new compiler hooks — the existing copy fold and
   drop machinery move handles around like any other Cloneable value.

## The Protocol

```kestrel
/// The contract for a shared-ownership container the compiler may use for
/// implicit boxing: escaping closure environments, class storage, `any`
/// payloads, `indirect enum` payloads.
///
/// Conformers are handles: small, fixed-layout values pointing at managed
/// storage that owns one `Target`. Duplicating a handle (`clone`) shares the
/// storage; dropping the last handle destroys the payload exactly once.
public protocol SharedBox: Cloneable, MutableIndirection {
    /// Take ownership of `value` and move it into managed storage,
    /// returning the first handle.
    init(consuming value: Target)

    /// Mutable access to the payload through a *shared* (non-mutating)
    /// handle — the interior-mutability primitive. This is the marked
    /// exception to value semantics; sound while Kestrel is single-threaded.
    func sharedMutRef() -> &mutating Target

    /// Do the two handles refer to the same managed storage? Backs class
    /// identity (`===`). Required instead of exposing an address so that a
    /// moving collector can still conform.
    func isIdentical(to other: Self) -> Bool

    /// `true` only when this handle is provably the sole owner. Backs
    /// copy-on-write forking. Implementations without cheap uniqueness
    /// information (a tracing GC) may conservatively return `false`;
    /// callers must treat `false` as "fork before mutating".
    func isUnique() -> Bool
}
```

Most of the contract is inherited rather than invented:

| operation | comes from | notes |
|---|---|---|
| share | `Cloneable.clone()` | the aggregate copy fold already routes struct/enum copies through it — a struct holding a handle becomes Cloneable and shares on copy for free |
| release | the conformer's own `deinit` | ordinary drop rules; last release destroys the payload |
| read payload | `MutableIndirection.pointeeRef()` | also gives transparent member access (`box.field`) |
| create | `init(consuming:)` | protocol requirement |
| shared mutation | `sharedMutRef()` | protocol requirement, see below |
| identity | `isIdentical(to:)` | protocol requirement |
| uniqueness | `isUnique()` | protocol requirement, may be conservative |

`sharedMutRef` is the one genuinely new operation, and it is the honest name
for what escaping closures and classes do: mutate through a shared handle
with no copy-on-write barrier. It needs no compiler magic — `RcBox` already
mutates through a non-`mutating` receiver by going through `Pointer`
(`self.valuePtr().mutatingValue`), which is the sanctioned unsafe escape
hatch. The protocol standardizes that surface; implementing it is an
`unsafe`-flavored act the stdlib performs so user code never has to.

## The Lang-Item Binding

The compiler must *instantiate* a box at synthesized, unnameable payload
types (a closure's environment struct, a class's storage struct), so the
binding is a generic type marked as a lang item — not a conformance search:

```kestrel
@lang(sharedBox)
public struct RcBox[T]: Cloneable, SharedBox { ... }   // stdlib default
```

Every implicit-boxing site lowers to the `@lang(sharedBox)` type applied to
the payload type, then touches it only through `SharedBox` requirements.

**Overriding.** v1 has a single global binding, like a global allocator:
supplying an alternate prelude/build configuration moves `@lang(sharedBox)`
to another conforming type (`GcBox`, `ArenaBox`), and escaping closures,
classes, existentials, and indirect enums all retarget together. Because
lowering is protocol-driven, a later per-declaration override
(`@box(GcBox) class Node { ... }`) is only "instantiate a different
conformer here" — the design keeps that door open without committing to it.

An override is a **semantic** choice, not a transparent one. The default
`RcBox` binding promises deterministic last-release cleanup — `deinit`s run
at a predictable point — and does not collect strong cycles. A tracing
collector inverts both. These are properties of the *binding*, documented
there; the protocol deliberately promises neither.

**Naming.** The mechanism-neutral name belongs to the protocol; each
conformer says what it actually does: `RcBox` (non-atomic refcount, default),
future `ArcBox` (atomic), `GcBox` (traced). A user-facing alias
`Shared[T]` — "whatever the build's box is", i.e. the current lang-item
binding — may be added later for code that wants shared ownership without
naming a mechanism. Code that specifically wants refcounting names `RcBox`
and keeps it regardless of the binding.

## Clients

| client | payload | handle semantics | operations used |
|---|---|---|---|
| `escaping` closure (now) | synthesized environment struct | reference — aliases share state | `init`, `clone`, drop, `sharedMutRef` |
| class (future) | synthesized storage struct | reference | the above + `isIdentical` for `===` |
| `any P` (future) | the concrete `T`, when not inline | value facade over shared storage | `init`, `clone`, drop, `pointeeRef` |
| `indirect enum` (future) | the recursive payload | **value** — copy-on-write | via the CoW layer: `clone` on copy, `isUnique` + fork on mutation |

### Escaping closures

The first client, specified in [closures.md](closures.md). An `escaping`
literal builds its owned environment struct `E` (snapshots per the owning
capture table), then wraps it: `Box(consuming: env)` where
`Box = @lang(sharedBox)` at `E`. The closure value is `{ fn_ptr, handle }`;
capture-free closures stay bare function pointers and never allocate.

- Calling the closure projects the environment with `sharedMutRef()` — the
  body may mutate captured state, and aliases observe it (the reference
  semantics the `escaping` keyword marks).
- Duplicating the closure clones the handle (retain under `RcBox`); the
  aggregate fold makes any struct holding the closure Cloneable.
- Dropping the last handle runs the box `deinit`, which drops `E`, which
  drops each capture — the "captures released exactly once" guarantee falls
  out of ordinary drop semantics.

### Classes

A `class C` is sugar for a hidden storage struct plus a handle type bound to
`Box[C.Storage]`. Field reads go through `pointeeRef`, field writes and
`mutating` methods through `sharedMutRef` (classes are the other sanctioned
reference-semantics feature), `===` is `isIdentical(to:)`. Class values are
Cloneable handles: assignment shares, and the existing copy fold propagates
that through aggregates unchanged.

### `any P` existentials

Payloads too large for inline storage (or non-Copyable) are boxed. The
existential carries the protocol witness table plus a type-erased handle.
Erasure is the interesting part: the compiler must call `clone`/drop/project
on `Box[T]` without knowing `T`, so monomorphization synthesizes a small
per-`T` shim table (clone fn, drop fn, project fn) carried alongside the
witness table — the same machinery existentials already need for their
payload operations. No protocol change; purely an implementation concern.

`any` stays value-semantic from the user's point of view: mutation of a
boxed existential must fork (via the CoW layer) rather than write through
`sharedMutRef`.

### `indirect enum`

The `indirect` payload is boxed to break layout recursion — the enum's
payload slot holds a fixed-size handle instead of the infinite type. Enums
are value types and must stay that way, so indirect enums lower through the
**CoW layer**, not the raw box: copies share (`clone`), and any mutation of
the payload runs the uniqueness check and forks when shared. A box whose
`isUnique` conservatively returns `false` degrades to fork-always — slower,
still correct.

## Constraints on a Box Implementation

The compiler enforces these when a type is bound as `@lang(sharedBox)`:

1. **Fixed handle layout.** The handle's size and alignment must not depend
   on `Target`. Existentials type-erase the handle, and indirect enums rely
   on it to make recursive layout computable (`enum E { case Node(Box[E]) }`
   works only because the handle is pointer-sized no matter the payload).
   Checked structurally: every stored field's layout must be
   `Target`-independent — in practice, pointers only. `RcBox` is one
   `Pointer[RcBoxStorage[T]]`: ✓.
2. **No self-dependence.** The box implementation may not itself require
   implicit boxing — no escaping closures, classes, `indirect enum`s, or
   `any` types inside it. `RcBox` today uses only normal frame-view closures
   (`ptr.with { ... }`), which is fine; this becomes a stated rule.
3. **Handles are never bitwise-Copyable.** The `Cloneable` bound already
   says it; the verify-time assertion from closures.md ("no bitwise-copyable
   representation owns a droppable environment") generalizes to: an implicit
   box handle is always Cloneable, never Copyable. A bit-copied handle would
   skip the share operation — the old closure bug class.
4. **The recursion knot.** Computing an indirect enum's layout needs the
   handle's size before `RcBoxStorage[Enum]` can be instantiated (the
   storage block mentions the enum, which mentions the handle, …).
   Constraint 1 is what unties it: the compiler uses the guaranteed fixed
   handle layout when laying out the enum, and instantiates the storage
   block's interior afterwards. Layout computation must be structured to
   exploit this rather than recurse.

## Stdlib Changes

- **Add `protocol SharedBox`** (`std.memory`), as above.
- **Conform `RcBox`**: it already has `init(consuming:)`, `clone`,
  `isUnique`, and `MutableIndirection`; it gains `sharedMutRef()` (via
  `valuePtr()`, like `modify`) and `isIdentical(to:)` (storage pointer
  compare), plus the `@lang(sharedBox)` attribute.
- **Generalize `CowBox`** over any `SharedBox` instead of hard-coding
  `RcBox`, so the CoW layer (used by String/Array/Dictionary today, indirect
  enums tomorrow) follows the binding. `RcBox`'s refcount-specific extras
  (`deepClone`, `setValue`, `refCount`) stay inherent API outside the
  protocol — a box needs none of them to conform.
- Later, optionally: `Shared[T]` alias for the current binding.

## Resolved Questions

1. **Protocol shape: associated type.** `Target` comes from
   `MutableIndirection`'s associated type, as written above — it composes
   with the existing `Indirection` machinery `RcBox` already conforms to,
   and one box type only ever manages one payload scheme. A generic
   `SharedBox[Element]` protocol was considered and rejected.
2. **Creation is `init(consuming value: Target)`.** A protocol init
   requirement, matching `RcBox`'s existing init exactly. (If init
   requirements hit an implementation wall in this position, the fallback
   spelling is `static func create(consuming value: Target) -> Self` —
   fallback only, not the design.)
3. **Threading: deferred to the concurrency design.** Single-threaded
   soundness (for `sharedMutRef` and the non-atomic refcount) is a
   current-language invariant. When concurrency lands, the protocol is the
   hook point — an atomic requirement, a `Sendable`-style marker, or an
   atomic conformer bound for threaded builds — but nothing is committed
   now. (Mirrors closures.md open question 4.)
4. **Weak references: deferred.** Strong cycles leak under the default
   binding, as closures.md documents. A `WeakBox` companion protocol is
   added when classes land — that is when back-references actually appear —
   and a GC conformer satisfies it trivially. Nothing here blocks it.
5. **Existentials are boxed-only in this document.** Whether small `any`
   payloads bypass the box entirely (inline buffer optimization) is decided
   in the future existential design; this document supplies only the boxed
   path and keeps `SharedBox`'s contract small.
