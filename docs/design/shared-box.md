# The Shared Box

**Status: box bindings consolidated (0.20, 2026-11).** The protocols are
`Box` and `SharedBox: Box` (`lang/std/memory/box.ks`); every boxing feature
reads one tagged alias in `lang/std/memory/box_bindings.ks` (see
[Box Bindings](#box-bindings)). `ExistentialBox` was renamed `Box`, `isUnique`
moved onto it, and `@builtin(.SharedBox)` now tags the protocol, not `RcBox`.
The rest of this document is the original design, updated where the
bindings changed.

**Originally implemented for the closure client (2026-08-08).** Companion to
[closures.md](closures.md); answers its open question 5 (the shared-object
container). Shipped: the `SharedBox` protocol (now `lang/std/memory/box.ks`),
`RcBox`'s conformance and its `@builtin(.SharedBox)` binding
(`lang/std/memory/rcbox.ks`), and `escaping` closure environments as the first
client. The other clients — classes, `any` existentials, `indirect enum`
payloads — remain future work, and the `CowBox` generalization is **deferred**
(see [Stdlib Changes](#stdlib-changes)).

**Sibling added during implementation: `UniqueBox`.** The consuming tier needs
the opposite contract — one owner, no refcount, payload movable *out* — so it
does not use a shared box at all. `@builtin(.UniqueBox)` →
`std.memory.UniqueBox[T]` is a single allocation holding a liveness word plus
the payload, with `takeValue()` (move the payload out, mark the block empty)
and `destroy()` (drop the payload if still present, then free). It is not a
`SharedBox` conformer and imposes none of this document's constraints; see
[the plan's D5](../plans/closure-kinds/closure-kinds-plan.md) for why reusing
`RcBox` with a pinned count of 1 was rejected (its release runs
`dropInPlace` on the whole environment, double-freeing slots the one-shot body
moved out).

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
/// A box: a handle onto one `Target` behind a pointer, built from a value.
/// Every box conforms (`RcBox`, `CowBox`, `OwnedBox`); `any[B] P` needs this.
@builtin(.Box)
public protocol Box: Indirection {
    /// Take ownership of `value` and move it into managed storage,
    /// returning the first handle.
    @builtin(.BoxInit)
    init(consuming value: Target)

    /// `true` only when this handle is provably the sole owner. Backs
    /// copy-on-write forking. A single-owner box always answers `true`;
    /// implementations without cheap uniqueness information (a tracing GC)
    /// may conservatively return `false`; callers must treat `false` as
    /// "fork before mutating".
    var isUnique: Bool { get }
}

/// A box whose handles alias: the shared-ownership contract behind escaping
/// closure environments, class storage and plain `any` values.
///
/// Conformers are handles: small, fixed-layout values pointing at managed
/// storage that owns one `Target`. Duplicating a handle (`clone`) shares the
/// storage; dropping the last handle destroys the value exactly once.
@builtin(.SharedBox)
public protocol SharedBox: Box, Cloneable, MutableIndirection {

    /// Mutable access to the payload through a *shared* (non-mutating)
    /// handle — the interior-mutability primitive. This is the marked
    /// exception to value semantics; sound while Kestrel is single-threaded.
    func sharedMutRef() -> &mutating Target

    /// Do the two handles refer to the same managed storage? Backs class
    /// identity (`===`). Required instead of exposing an address so that a
    /// moving collector can still conform.
    func isIdentical(to other: Self) -> Bool
}
```

Most of the contract is inherited rather than invented:

| operation | comes from | notes |
|---|---|---|
| share | `Cloneable.clone()` | the aggregate copy fold already routes struct/enum copies through it — a struct holding a handle becomes Cloneable and shares on copy for free |
| release | the conformer's own `deinit` | ordinary drop rules; last release destroys the payload |
| read payload | `MutableIndirection.value` (its `ref`) | also gives transparent member access (`box.field`) |
| create | `Box.init(consuming:)` | protocol requirement |
| shared mutation | `sharedMutRef()` | protocol requirement, see below |
| identity | `isIdentical(to:)` | protocol requirement |
| uniqueness | `Box.isUnique` | protocol requirement, may be conservative |

`sharedMutRef` is the one genuinely new operation, and it is the honest name
for what escaping closures and classes do: mutate through a shared handle
with no copy-on-write barrier. It needs no compiler magic — `RcBox` already
mutates through a non-`mutating` receiver by going through `Pointer`
(`self.valuePtr().mutatingValue`), which is the sanctioned unsafe escape
hatch. The protocol standardizes that surface; implementing it is an
`unsafe`-flavored act the stdlib performs so user code never has to.

## Box Bindings

The compiler must *instantiate* a box at synthesized, unnameable payload
types (a closure's environment struct, a class's storage struct), so it reads
a binding: a generic type alias carrying a feature tag, which denotes its
(chained) alias target. All of them live in one file,
`lang/std/memory/box_bindings.ks`:

```kestrel
public type Shared[T] = RcBox[T]                   // role: shared ownership
@builtin(.OwnedBox) public type Owned[T] = OwnedBox[T]   // role: single owner

@builtin(.ClosureBox)          public type ClosureBox[T]          = Shared[T]
@builtin(.ClassBox)            public type ClassBox[T]            = Shared[T]
@builtin(.AnyBox)              public type AnyBox[T]              = Shared[T]
@builtin(.IndirectBox)         public type IndirectBox[T]         = Owned[T]
@builtin(.ConsumingClosureBox) public type ConsumingClosureBox[T] = UniqueBox[T]
```

- **Role aliases** (`Shared`, `Owned`) name the box std uses for a kind of
  ownership. Editing one retargets every feature that points at it.
- **Feature aliases** name the box one compiler feature uses. Each normally
  points at a role; pointing one at another box splits that feature off
  (`ClassBox = GcBox[T]` for weak refs while `any` stays on `RcBox`) with no
  compiler change.
- `ResolveBuiltin` follows alias chains to the struct, and lowering then
  touches it only through protocol requirements (`BoxInit`,
  `SharedBoxSharedMutRef`, `ConsumingIndirectionConsumeValue`), never by name
  or shape. `ConsumingClosureBox` still uses `UniqueBox`'s own member tags,
  and indirect-enum unboxing still calls `OwnedBox.takeValue()` by its tag;
  both go away when consuming closures move onto `Owned`.
- Each feature tag carries the protocols its box must provide
  (`Builtin::box_binding_bound`), checked at the alias by **E470**:

| binding | must conform to | why |
|---|---|---|
| `ClosureBox`, `ClassBox` | `SharedBox` | aliases share and mutate state; `===` |
| `AnyBox` | `Box` | `init(consuming:)` and the peel |
| `IndirectBox`, `Owned` | `Box`, `ConsumingIndirection` | matching moves the payload out |
| `ConsumingClosureBox` | (none yet) | bespoke `UniqueBox` |

**Overriding.** Bindings are global: std decides them, users do not. Users
pick a box per site instead: `any[B] P` today, and a per-declaration
`@box(GcBox) class Node { ... }` is the planned escape hatch for classes.
One switch per role keeps a closure stored in an `any`, or a class field
holding a closure, from mixing boxes with different promises by accident.

An override is a **semantic** choice, not a transparent one. The default
`RcBox` binding promises deterministic last-release cleanup — `deinit`s run
at a predictable point — and does not collect strong cycles. A tracing
collector inverts both. These are properties of the *box*, documented
there; the protocols deliberately promise neither.

**Naming.** The mechanism-neutral names belong to the protocols and roles;
each conformer says what it actually does: `RcBox` (non-atomic refcount,
default), future `ArcBox` (atomic), `GcBox` (traced). Code that wants shared
ownership without naming a mechanism writes `Shared[T]`; code that
specifically wants refcounting names `RcBox` and keeps it regardless of the
binding. The thing a box holds is its **value** (`Target` is its type) in
names and prose.

## Clients

| client | binding | payload | handle semantics | operations used |
|---|---|---|---|---|
| `escaping` closure (**shipped**) | `ClosureBox` → `Shared` | synthesized environment struct | reference — aliases share state | `init`, `clone`, drop, `sharedMutRef` |
| class (future) | `ClassBox` → `Shared` | synthesized storage struct | reference | the above + `isIdentical` for `===` |
| `any P` (**shipped**, 0.17) | `AnyBox` → `Shared` | the concrete `T` | reference — copies share the value | `init`, `clone`, drop, `value` |
| `indirect enum` (**shipped**, 0.19) | `IndirectBox` → `Owned` | a recursive case's whole payload | **value** — one owner per case; a copy clones the payload | `init`, `clone`, drop, `takeValue`; reads project the box pointer in place |
| `consuming` closure (**shipped**) | `ConsumingClosureBox` → `UniqueBox` | synthesized environment struct | one owner, taken once | `UniqueBox` `init`, `takeValue`, `destroy` |

### Escaping closures

The first client, specified in [closures.md](closures.md). An `escaping`
literal builds its owned environment struct `E` (snapshots per the owning
capture table), then wraps it: `Box(consuming: env)` where
`Box = ClosureBox` at `E`. The closure value is `{ fn_ptr, handle }`;
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
`ClassBox[C.Storage]`. Field reads go through `value`, field writes and
`mutating` methods through `sharedMutRef` (classes are the other sanctioned
reference-semantics feature), `===` is `isIdentical(to:)`. Class values are
Cloneable handles: assignment shares, and the existing copy fold propagates
that through aggregates unchanged.

### `any P` existentials

`any P` is `AnyBox[rawany P]`, and `AnyBox` is `Shared`, i.e. `RcBox`: the
existential *is* a shared box over an unsized payload, so copies share it
(reference semantics; `any[CowBox] P` gives value semantics). The payload's
drop, size, alignment and clone travel in the function table carried by the
box's pointer, so `clone`/drop/`value` on `RcBox[rawany P]` are the
ordinary box operations. A box only needs `Box` (`init(consuming:)`,
`isUnique` + `Indirection`) to hold an existential; `SharedBox` refines it. See [`docs/plans/existentials/existentials-design.md`](../plans/existentials/existentials-design.md)
§3 and the user guide [`docs/language/existentials.md`](../language/existentials.md).

### `indirect enum`

The `indirect` payload is boxed to break layout recursion — the enum's
payload slot holds a fixed-size handle instead of the infinite type. Enums
are value types and must stay that way. As shipped (0.19), each boxed case
holds an `IndirectBox` — `Owned`, a single-owner `OwnedBox` — not the shared
binding: a copy clones the payload, and matching a case moves it out
(`takeValue`). The original plan, a CoW layer over the shared box, was not
needed; `Box.isUnique` keeps that option open.

## Constraints on a Box Implementation

The compiler relies on these for every box a binding names (E470 checks the
protocol side; the rest is structural):

1. **Fixed handle layout.** The handle's size and alignment must not depend
   on `Target`. Existentials type-erase the handle, and indirect enums rely
   on it to make recursive layout computable (`enum E { case Node(Box[E]) }`
   works only because the handle is pointer-sized no matter the payload).
   Checked structurally: every stored field's layout must be
   `Target`-independent — in practice, pointers only. `RcBox` is one
   `Pointer[RcBoxStorage[T]]` plus its allocator `A`: ✓. For the boxes the
   compiler builds itself (closure environments, indirect enum payloads)
   the handle must also be one word: one field leading to the pointer, and
   any other field an empty struct (the default `GlobalAllocator`), which
   the compiler drops when it forgets the handle down to its word and
   rebuilds when it turns the word back into a handle.
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

- **Add `protocol SharedBox`** (`std.memory`), as above. — **shipped**
  (`lang/std/memory/box.ks`, with `Box` since 0.20).
- **Conform `RcBox`**: it already has `init(consuming:)`, `clone`,
  `isUnique`, and `MutableIndirection`; it gains `sharedMutRef()` (via
  `valuePtr()`, like `modify`) and `isIdentical(to:)` (storage pointer
  compare), plus the lang-item attribute. — **shipped**; the attribute is
  spelled `@builtin(.SharedBox)` (the binding rides the existing `@builtin`
  machinery rather than a new `@lang` namespace), and the conformance
  `extend RcBox[T]: SharedBox` lives in `rcbox.ks` because it reads the
  private `ptr` field.
- **Generalize `CowBox`** over any `SharedBox` instead of hard-coding
  `RcBox` — **deferred.** No v1 client needs it (escaping closures use the box
  directly), and indirection peel does not work on bare type parameters, so a
  `CowBox[T, B: SharedBox]` would lose the `box.field` sugar internally.
  `CowBox` continues to name `RcBox`. `RcBox`'s refcount-specific extras
  (`deepClone`, `setValue`, `refCount`) stay inherent API outside the
  protocol — a box needs none of them to conform.
- `Shared[T]` alias for the current binding. — **shipped** (0.20, with the
  other bindings in `box_bindings.ks`).

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
