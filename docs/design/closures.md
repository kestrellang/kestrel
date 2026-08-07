# Closure Semantics

**Status: draft — supersedes the interim model documented in `docs/memory-model/closures.md`.**

This document defines sound semantics for Kestrel closures: how they capture,
copy, and drop. It replaces the current interim implementation, in which every
closure is treated as plain data — captured values are never dropped (a
captured `FileHandle`'s `deinit` never runs), and copying a closure aliases
its stack environment.

The design rests on one idea: **a closure holds its environment the way its
kind says it does, and the kind is spelled in the type.** There are four
kinds. Two are frame-bound and hold *views* of the enclosing frame; two *own*
their captures and may outlive the frame. Everything else — capture mode,
copy class, drop rules, escape rules — follows from the kind. Internally, a
kind fixes two independent properties:

| kind | call capability | environment ownership |
|---|---|---|
| normal | shared, many-call | frame view |
| `mutating` | exclusive, many-call | frame view |
| `consuming` | one-call | unique owner |
| `escaping` | shared, many-call | shared owner |

Keeping these axes distinct matters for conversions: weakening how a value is
called does not by itself turn a frame view into an owned environment. The
implementation uses machinery the compiler already has (the copy-class fold,
the move checker, and provenance escape checking). No borrow checker and no
lifetime annotations are added.

## The Four Kinds

```
fn_type ::= ['mutating' | 'consuming' | 'escaping'] '(' param_types ')' '->' type
```

| kind | captures | body may | callable | copy class | can leave the frame |
|---|---|---|---|---|---|
| **normal** | views of the frame | read | many times, from `let` | Copyable | no |
| **`mutating`** | `&mutating` views | write back to the originals | many times; calls are exclusive (`var`) | `not Copyable` | no |
| **`consuming`** | owned (moved/copied in) | move captures out | exactly once; the call consumes it | `not Copyable` | yes |
| **`escaping`** | owned snapshots, **shared** | read and mutate its own state | many times, from `let` | Cloneable (clone retains) | yes |

Plain `(T) -> U` — the normal kind — remains the right type for the vast
majority of closures: `map`/`filter` callbacks, predicates, visitors. The
keywords appear only when an API needs write-back (`mutating`), hand-off
(`consuming`), or storage beyond the frame (`escaping`).

A capture-free closure (or a named function) is a bare function pointer: it
has no environment, satisfies every kind, and escapes freely.

## Capture Rules

### What is captured: the narrowest place

For each variable the body uses, the compiler captures the smallest
sufficient *place*, widening only when required:

```kestrel
{ self.data }          // captures the place `self.data`, not `self`
{ self.data.count }    // captures just `self.data.count`
{ self.method() }      // needs the receiver → widens to `self`
{ x + x.f }            // overlapping places merge → one capture of `x`
```

Because capture is place-based, a closure inside a method on a non-Copyable
type can freely use the receiver's fields without touching `self` itself.

### How it is captured: the kind decides

**Normal and `mutating` closures capture by view.** The environment holds
references into the frame — the same semantics as a named reference binding
(`let r = &x`): reads see later writes, nothing is copied or moved, and the
closure is frame-bound (E494). A `mutating` closure's views are `&mutating`,
so its assignments write back.

```kestrel
var x = 10;
let f = { x };     // live view of x
x = 20;
f()                // 20
```

**`consuming` and `escaping` closures capture by ownership.** The environment
must own its contents — a one-shot closure moves captures out, and an
escaping closure outlives the frame — so each captured place is copied,
cloned, or moved according to its copy class, at creation time:

| captured place is | owning capture does | source afterwards |
|---|---|---|
| Copyable | bit-copy | untouched |
| Cloneable | `clone()` | untouched |
| non-Copyable, owned by the frame | **move** | dead — later use is E500 |
| value carrying frame provenance (e.g. a view closure) | — rejected regardless of copy class | — |
| non-Copyable, not owned (e.g. borrowed `self`) | — rejected: nothing durable to own | — |

```kestrel
var x = 10;
let g: escaping () -> Int64 = { x };   // snapshots x == 10
x = 20;
g()                                     // 10
```

The type tells you which semantics you have: **view kinds see later writes;
owning kinds are snapshots.** A literal in an `escaping` or `consuming`
position is built with an owning environment; the same body in a normal or
`mutating` position is built with views.

An owning closure may not launder a frame-bound value into owned storage. In
particular, capturing an existing normal or `mutating` closure in a
`consuming` or `escaping` literal is rejected if that value carries frame
provenance. Capture-free function pointers remain freely ownable.

### Place capture consequences

- Overlapping captures within one literal merge before capture mode is
  applied. Across literals, freezes and moves are checked against overlapping
  places in the usual move-checker way.
- Owning capture of a stored projection such as `x.field` is a partial copy,
  clone, or move. Moving it leaves that projection dead and the enclosing
  aggregate partially moved; ordinary partial-drop rules drop the remaining
  live fields.
- A computed property or subscript is not itself captured storage. The
  closure captures the narrowest stored inputs needed to perform the access;
  an accessor requiring its receiver widens the capture to that receiver.
- Nested closures capture transitively. A frame-view inner closure remains
  rooted in the frame active when it is created and may be called there; it
  cannot escape merely because the outer closure owns its own environment.

### The freeze rule (view kinds only)

A view must not dangle. While any live normal or `mutating` closure value may
carry a view of a place, that place is **frozen against destruction**: it
cannot be moved, passed to a `consuming` parameter, or destroyed with
`deinit x;`. Plain reassignment stays legal — that is a write through a live
view, which references already permit.

```kestrel
let file = FileHandle(fd: 3);
let f = { file.read() };   // views file — file stays usable
f();
consume(file);             // ERROR: cannot consume `file` while `f` captures it
```

The freeze is lexical, not use-liveness-based. Capture provenance propagates
through closure copies, aggregate construction, assignments, parameters, and
control-flow joins. Every binding or temporary that may carry the view extends
the freeze to the end of its own lexical extent; copying `f` to a wider-scoped
`g` therefore extends the freeze through `g`. Reassigning or consuming a
closure does not shorten the statically chosen extent. This is deliberately
coarser than reference liveness tracking, but it is not tied only to the
literal's first binding. It is the closure analogue of E498 ("cannot consume a
value while a reference into it is live"). Owning kinds freeze nothing: their
captures are theirs.

## The Kinds in Detail

### Normal: read-only views

The default. The body reads its captures; assigning to one is an error with a
fix-it suggesting `mutating` or a value-returning fold. Normal closures bind
with `let`, get called any number of times, and copy freely — copies share
the frame's views, which is sound because nobody writes through them.

```kestrel
let double: (Int64) -> Int64 = { it * 2 };
numbers.map { it * 2 }
```

### `mutating`: write-back

Captures `&mutating` views, so assignments hit the enclosing variables.
Calling one is an exclusive use — it must be held in a `var` (or `mutating`
parameter) — and the value is `not Copyable`, which keeps mutation sound with
no aliasing analysis. Like a `mutating` method, it cannot leave its frame.

```kestrel
var total = 0;
items.forEach { total = total + it };
total    // the sum — writes went back to `total`
```

Here `forEach`'s parameter type supplies the `mutating` kind. There is no
kind-on-literal spelling in this version.

### `consuming`: one-shot hand-off

Owns its captures; calling it consumes it, and a second call is an ordinary
use-after-move (E500). Because it runs at most once, its body is the one
place allowed to move captures *out* — return them or pass them onward. This
expresses resource transfer, which no multi-call closure can soundly do. It
may escape the frame (it owns everything it holds); `not Copyable` — handing
it around is a move.

```kestrel
func onDone(consuming f: consuming () -> ()) { f(); }

let file = FileHandle(fd: 3);
onDone({ () in transfer(file) });   // file moves in, then out — exactly once
```

### `escaping`: a shared, stateful object

Owns snapshots of its captures in a **shared heap environment**. Escaping
closures are **Cloneable**, not bitwise-Copyable: duplicating a handle runs
the container's share operation (a retain, under refcounting) — `clone()` is
a *shallow share* of the environment, not an independent duplicate. The
captures are dropped exactly once when an acyclic environment reaches its
last release. Because duplication goes through the ordinary Cloneable
machinery, the aggregate fold does the right thing for free: a struct holding
an escaping closure field becomes Cloneable, so copying the struct shares
(never bit-copies) the environment handle. The body may mutate its captured
state, which persists across calls — and because copies share the environment,
an escaping closure has **reference semantics**: aliases see one another's
mutations. This is the deliberate exception to Kestrel's value semantics, and
the keyword marks it: `escaping` means the closure detaches from the frame and
lives as a shared object.

Sharing is the *semantics*; the initial mechanism is refcounting (clone =
retain, drop = release, last release destroys the environment). Refcounting
does not collect strong cycles. Programs that form a cycle of escaping
environments must break it explicitly once weak references exist; until then,
such a cycle leaks and its captures are not deinitialized. This is the same
constraint the general shared-object story for classes, `any` protocol
existentials, and `indirect enum` payloads will need to address.

A future tracing container would change observable cleanup timing, which
matters for resource-owning `deinit`s, so it is not a semantics-neutral
implementation swap. Deterministic last-release cleanup is part of the
initial model.

```kestrel
func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { () in count = count + 1; count }   // snapshots count; mutates its own copy
}

let next = makeCounter(10);
next();          // 11
let alias = next;   // retain — shares the environment
alias();         // 12
next();          // 13 — one counter, two handles

struct Button {
    let onClick: escaping () -> ()      // storable long-term; Button is Cloneable
}
```

## Passing: What Fits Where

The expected type declares both how the callee may call the closure and what
ownership representation it receives. A closure literal is built directly
for the expected kind when its body supports that kind. For existing closure
values:

| have ↓ \ expected → | normal | `mutating` | `consuming` | `escaping` |
|---|---|---|---|---|
| normal | ✓ | ✓ | ✗ frame view is not owned | ✗ frame-bound |
| `mutating` | ✗ | ✓ | ✗ frame view is not owned | ✗ frame-bound |
| `consuming` | ✗ | ✗ | ✓ | ✗ one-shot |
| `escaping` | ✓ | ✗ shared, not exclusive | ✓ | ✓ |

At the call-capability level, normal, `mutating`, and `consuming` resemble
Rust's `Fn ⊆ FnMut ⊆ FnOnce`: a read-only body tolerates exclusive or
one-shot calling, and a mutating body tolerates one-shot calling. That is not
the value-conversion lattice, because `consuming` additionally promises an
owned, returnable environment. A frame-view value cannot acquire that promise
by coercion. APIs that transfer or store a one-shot callback ask for
`consuming`; an API that merely calls a borrowed callback once still asks for
normal or `mutating`. A separate nonescaping once-call convention may be added
later if accepting both view kinds through one API proves important.

A cross-kind conversion never recompiles the closure body or grants it new
capture operations. It constructs an adapter with the expected call behavior
while preserving the source body's restrictions. Thus normal → `mutating`
creates an exclusive-call adapter over the same frame views, and `escaping` →
`consuming` creates a unique one-shot adapter that owns one retained shared
handle; it does not make the escaping body's captures movable.

An `escaping` value additionally passes wherever normal or `consuming` is
expected (it is duplicable, multi-callable, and owns its env), but not as
`mutating` — its calls are shared, not exclusive. Nothing frame-bound ever
flows into an `escaping` slot. The `escaping → normal` coercion does not
re-label the value (normal is bitwise-Copyable; bit-copying a shared handle
would skip the share operation): it produces a non-owning *view* of the
shared environment, rooted at the original handle by provenance and valid for
the frame — the same borrow shape as any other normal-kind closure.

The kind also dictates the parameter convention needed to call it: a
`mutating (T) -> U` parameter must itself be `mutating`; a
`consuming (T) -> U` parameter must be `consuming`. The compiler enforces the
pairing.

## Copy and Drop

The environment payload is an ordinary synthesized struct, so its fields use
the existing copy-class fold and drop machinery. The closure kind wraps that
payload with a representation policy; the kind, not the payload fold alone,
determines the closure value's copy class.

- **normal / `mutating`**: the environment owns nothing (views only) — no
  drops; the frame's locals drop as usual.
- **`consuming`**: the sole owner drops the environment, dropping any
  captures not already moved out by the one call.
- **`escaping`**: Cloneable — duplication is the container's share operation,
  never a bitwise copy. Environments are heap-allocated at creation; an
  acyclic environment's captures drop exactly once at the last release.

Every uniquely owned capture, and every capture in a reclaimed shared
environment, runs `deinit` exactly once. Strong reference cycles are the
documented exception under the initial refcounting implementation. A
verify-time assertion enforces that no bitwise-copyable closure representation
ever owns or retains a droppable environment, so the old bug class cannot
silently return.

Ordinary drop rules cover the remaining lifecycle seams. Reassigning an
escaping-closure `var` releases its old handle before storing the new one. A
`consuming` call owns its environment for the duration of the call and tears
down every capture not moved out on every function exit; per-slot move state
prevents moved-out captures from being dropped again. Aggregates containing
closures inherit these operations memberwise.

## Inference

Nothing is spelled on literals. An expected function type first selects the
literal's environment and call strategy; the body must then satisfy that kind.
For example, assignment through a capture is rejected against an expected
normal type rather than silently changing it to `mutating`.

Without an expected type, assignment through a capture implies `mutating`,
moving a capture out implies `consuming`, and a read-only body is normal.
Escape analysis never silently upgrades an inferred view closure to
`escaping`: attempting to return or store it produces E494 with a fix-it to
put `escaping` in the expected type, after which the literal is rebuilt with
owned captures. This keeps ownership inference local and the escaping marker
honest. Explicit kind-on-literal syntax (`{ mutating () in ... }`) and
Swift-style capture lists are reserved for future use, not shipped.

## Diagnostics

| code | today | under this design |
|---|---|---|
| E494 | closure capturing a local cannot escape | unchanged for view kinds; message gains a fix-it suggesting an `escaping` or `consuming` owning type |
| E500 | use after move | also fires on use after an owning capture moved a non-Copyable, and on calling a consumed `consuming` closure |
| E506 | cannot move a capture out of a closure | kept for normal/`mutating`/`escaping` bodies; lifted inside `consuming` bodies |
| E603 | cannot assign to a capture | kept for normal bodies, with a fix-it suggesting `mutating`/`escaping`; lifted in both |
| E212 | closures cannot capture reference bindings | retired — view capture is now the default |
| *new* | freeze violation | cannot move/consume/`deinit` a place while a live view-kind closure captures it (closure analogue of E498) |
| *new* | kind mismatch | passing against the kind table, or calling a `mutating`-kind closure without exclusive access |

## Behavior Changes from Today

1. `let f = { x }; x = 20; f()` returns **20**, not 10 — normal captures are
   views. Owning kinds (`escaping`, `consuming`) keep snapshot behavior.
2. `items.forEach { total = total + it }` works when `forEach` expects a
   `mutating` closure; the current E603 has a real answer instead of a
   workaround.
3. Capturing a non-Copyable value no longer kills the original in view kinds
   — it is merely frozen against destruction. Only owning capture moves it.
4. Returning a capturing closure becomes possible — with `escaping` (shared,
   multi-call) or `consuming` (unique, one-shot) in the return type.
5. Captured resources are released: owning environments participate in drop,
   subject to the documented strong-cycle limitation of refcounting.
6. Structs may hold view-kind closures in fields; the struct becomes
   frame-bound via the existing provenance carry. `escaping` fields keep the
   struct storable long-term.
7. Copies of an `escaping` closure share state — reference semantics, marked
   by the keyword.

## Implementation Notes

- **Front end**: the kind on function types threads through parsing,
  unification (the passing table), and the existing closure-convention
  reconciliation. The freeze rule lives in the move checker
  (kestrel-analyze) and reuses its existing place/provenance propagation;
  freeze endpoints are scope-based rather than computed from last use.
- **MIR**: two lowering tiers for `ApplyPartial` — a frame environment of
  views (current layout, minus the ownership bugs) for normal/`mutating`, and
  an owning environment (heap + the shared-object container — the
  `@lang(sharedBox)` binding, initially `RcBox`; see
  [shared-box.md](shared-box.md) — for `escaping`; stack or heap, uniquely
  owned, for `consuming`) with slots holding values. The escape provenance
  stamp follows the plan already noted in `emit_apply_partial`: an owning
  closure roots at the join of its captures' *own* roots (self-rooted
  snapshots → returnable); a view-holding closure roots at the frame, as
  today. The escape check itself never changes.
- **Copy/drop**: delete the "closures are POD" interim in `ty_query.rs`
  (`copy_behavior`/`needs_drop` for `FuncThick`). Fold the environment payload
  normally, then apply the kind's representation policy: normal is a
  bit-copyable view, `mutating` and `consuming` are non-Copyable, and
  `escaping` is Cloneable with retain/release.
- **Stdlib checkpoint**: passed; see
  [closures-stdlib-audit.md](closures-stdlib-audit.md). Add the audit's compile
  matrix before committing the syntax.

## Comparison

Rust captures by borrow under a full borrow checker and expresses call
capability as inferred, invisible traits (`Fn`/`FnMut`/`FnOnce`); Swift shares
variables through ARC-managed boxes and has no kinds at all. Kestrel takes
Rust's kind lattice as inspiration for its call-capability axis, but does not
use it to erase the distinction between frame views and owned environments.
It takes Swift's escaping model (shared, reference-semantic, keyword-marked)
for the closures that genuinely outlive their frame; and replaces both the
borrow checker and lifetime annotations with the lexical freeze rule plus the
existing provenance escape check. The costs accepted: view kinds cannot leave
the frame, `escaping` closures alias their state, and long-lived borrows remain
inexpressible — consistent with the trade-offs Kestrel's reference model
already makes.

## Pinned Edge Cases

- A normal or `mutating` closure capturing a loop iteration binding cannot
  outlive that iteration's lexical scope. An `escaping` or `consuming`
  expected type snapshots each iteration's value when accumulating closures
  for later calls.
- Capturing closures in aggregate fields preserves their provenance. A
  frame-view field makes the aggregate frame-bound; an escaping field makes
  the aggregate Cloneable.
- Reassigning a closure-containing aggregate drops or releases its old closure
  fields using the same memberwise rules as any other aggregate.

## Open Questions

1. **Exclusivity for `mutating` captures.** Kestrel has no uniqueness rule
   for `&mutating`, and this design does not add one: two `mutating` closures
   over the same place are permitted (single-threaded, deterministic).
   Decided for now; revisit if a general exclusivity rule ever lands.
2. **`consuming` in v1?** The kind is fully specified here; shipping it may
   trail the others if the schedule demands.
3. **Generic/protocol positions.** Kinds on function types in witness and
   generic contexts (e.g. a protocol method taking `consuming () -> T`) are
   expected to work like other conventions but need their own test matrix.
4. **Threading.** `escaping`'s shared mutable state is sound today because
   Kestrel is single-threaded. A future concurrency story must revisit it
   (an atomic or otherwise thread-safe container, and either a
   `Sendable`-style marker or exclusivity).
5. **The shared-object container.** Designed: see
   [shared-box.md](shared-box.md). A `SharedBox` protocol (defined entirely
   in Kestrel) states the contract; `RcBox` is the default `@lang(sharedBox)`
   binding, swappable for other containers (GC, atomic) without changing any
   client lowering. Escaping environments are the first client; classes,
   `any` existentials, and `indirect enum` payloads follow. The initial
   contract is deterministic last-release cleanup and share-on-clone; strong
   cycles leak.
