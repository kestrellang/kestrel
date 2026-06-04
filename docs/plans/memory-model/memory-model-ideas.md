# Kestrel Memory Model — as built

This document captures Kestrel's memory model as it stands at the end of the
compiler rewrite. It supersedes the original design notes. The user-facing model
is Copy-by-default with opt-out for owning types; the compiler enforces
ownership via MIR-level move-paths, init/maybe-init dataflow, and scope-aware
drop elaboration.

For internal architecture details, see
[`lib/kestrel-ownership/docs/architecture.md`](../../../lib/kestrel-ownership/docs/architecture.md).

---

## User-facing model

### Three type-level "shapes"

| Shape | How declared | Behavior |
|---|---|---|
| **Copyable** *(default)* | No marker | Implicit memcpy at every use. No `deinit`. |
| **Cloneable** | `: Cloneable` (conformance) | Explicit `clone()` required; otherwise treated as affine. |
| **Owning** / **Affine** | `: not Copyable` | Linear-ish: one owner at a time, `deinit` runs at scope exit. |

The compiler computes a type's *copy behavior* structurally from its fields:
a struct with any `not Copyable` field is itself `not Copyable`. Explicit
declarations override the structural rule (`: not Copyable` on a leaf-typed
struct still makes it affine).

### Access modes — encoded as parameter types

Function parameters carry their access mode in the *type*, not as a separate
modifier:

| Source syntax | MIR parameter type | Caller semantics |
|---|---|---|
| `f: Foo` *(default)* | `&Foo` | Borrow; caller retains ownership |
| `var f: Foo` *(mutating)* | `&var Foo` | Mutable borrow; caller retains |
| `consuming f: Foo` | `Foo` | Move; caller loses ownership at the call |

No more separate `ParamMode` / `PassingMode` / `CallArg.mode` axes — the type
*is* the mode. `Value::{Copy, Move, Ref, RefMut}` at call sites carry the
runtime "how do we pass it" choice.

### Aliasing is allowed

`&var T` does **not** carry Rust-style exclusivity. Multiple `&var T`s to the
same place coexist at the type-system level; mutating through any of them
does what mutating through any pointer does. Region / lifetime analysis is
explicitly Phase 2 and not implemented.

This is the deliberate "application-first" tradeoff: callers can take
multiple mutable views without proving uniqueness. The price is no aliasing
optimizations at codegen time — but Kestrel's optimizer isn't aliasing-aware
either, so it's a wash.

### Deinit and drop placement

A type marked `: not Copyable` can declare a `deinit` method that runs on
last use. The compiler also synthesizes field-by-field structural drops for
nested non-Copyable fields. User `deinit` runs *before* structural drops, so
`deinit` sees the type still fully initialized.

Drop placement is **scope-driven** with reverse declaration order: locals
declared later drop first. Field drops run in declaration order top-down
(matching constructor order: build inner-then-outer, destroy outer-then-inner).

Today's implementation runs all drops at the `Return` terminator (the "drops
live until return" granularity) rather than at the exact scope-exit
boundary. The model is correct — no leaks, no double-drops — but values
stay live longer than they strictly need to. Tighter scoping is a future
refinement.

### Panic = abort

Panic terminates the program with no cleanup. No drops run on the panic
path. The MIR verifier enforces this: any block ending in `Panic` /
`Unreachable` must contain zero `Drop` statements; static-init bodies
(`__init$<name>`) must also contain zero drops.

This is non-negotiable for the current model. Recoverable / unwind-style
panic with cleanup is a Phase 2 question, deliberately left out.

---

## Compiler architecture

```
   source  →  lex  →  parse  →  AST (ECS)  →  name-res  →  HIR
                                                            │
                                                            ▼
                                                       type-infer
                                                            │
                                                            ▼
                                                    kestrel-mir-lower
                                                            │
                                                            ▼
                                                     kestrel-ownership
                                                     ┌──────────────┐
                                                     │  move-paths  │
                                                     │  dataflow    │
                                                     │  move-check  │  → E500 / E501
                                                     │  drop-elab   │  → Drop / DropIf
                                                     └──────┬───────┘
                                                            ▼
                                                  verify (post-drop-elab)
                                                            │
                                                            ▼
                                                     codegen → object
```

### Key types

```rust
// On StructDef / EnumDef, computed once at MIR lowering time:
pub enum CopyBehavior { None, Bitwise, Clone(Entity) }
pub struct DeinitBehavior { user_method: Option<Entity>, field_drops: Vec<FieldId> }

// `MirTy::copy_behavior(module)` recursively folds across type args, so
// `Result[Thing, Int64]` (with Thing: not Copyable) reports None even
// though the unparameterized `Result` declares Bitwise.

// Enriched Value (replaces the old Value + CallArg + PassingMode trio):
pub enum Value { Copy(Place), Move(Place), Ref(Place), RefMut(Place), Const(Immediate) }

// Rvalue stays flat with top-level Copy / Move / Ref / RefMut; inner
// operand positions use the same Value enum. Call.args: Vec<Value>.
// No more CallArg.

// Statement drop forms:
pub enum StatementKind {
    Assign { dest: Place, rvalue: Rvalue },
    Call { dest: Option<Place>, callee: Callee, args: Vec<Value>, ... },
    Drop { place: Place },                     // unconditional
    DropIf { place: Place, flag: LocalId },    // gated on a runtime flag
    // (no more Deinit / DeinitIf / SetDeinitFlag — collapsed at Stage 7)
}
```

### Move-paths and dataflow

Per-function move paths intern at *root-local granularity*: each non-`Copy`
local gets one path. `s.f.0`, `s.field`, and `s` all fold to `s`'s path.
Field-level partial moves (precise tracking of `s.a` vs `s.b`) is a known
precision gap, currently surfacing as full-local moves.

Forward bit-set dataflow maintains two sets per program point:
- `def_init`: paths initialized on every CFG path leading here.
- `may_init`: paths initialized on at least one CFG path.

The dataflow worklist is a standard forward fixed-point with per-successor
`entry_changed` termination. Move-sites are recorded per path for E500 / E501
secondary labels.

### Diagnostics

| ID | Meaning | Source |
|---|---|---|
| E500 | Use of moved value | `kestrel-ownership::move_check`, when path is fully uninit at a read |
| E501 | Value may have been moved | Same source, when path is `MaybeInit \ DefinitelyInit` |
| E502 | *(reserved — move-out-of-borrow)* | Planned, not yet emitted |

The HIR-level `MoveTrackingAnalyzer` was retired at Stage 7 (commit
`ec815a70`). The MIR check is the sole emitter.

### Verifier rules

Pre-drop-elab MIR must contain zero `Drop` / `DropIf` (lowering must never
emit them — that's strictly DropElab's job).

Post-drop-elab MIR must satisfy:
- `Value::Move(p)` ⇒ `p.ty.copy_behavior(module) == None`
- `Value::Move(p)` ⇒ `p` is rooted in an owned local, not a `Deref(&T)` /
  `Deref(&var T)`
- Blocks ending in `Panic(_)` / `Unreachable` contain no drops
- `__init$<name>` static-init bodies contain no drops

---

## What's not done

The rewrite intentionally deferred several things to Phase 2 or later:

- **Region / lifetime checking for `&T` / `&var T`.** References are
  Bitwise-Copy and aliasing is permitted; there's no `'a` / `'b` machinery.
- **Field-level partial moves.** `s.a` moved while `s.b` is read trips a
  full-local E500 instead of the precise "field `.b` is still live"
  diagnostic.
- **Tightening drop scope.** Drops currently live until `Return`; ideally
  they'd run at the precise scope-exit edge.
- **Panic-unwind / recoverable panic.** Panic = abort, period.
- **Linear types.** Affine + Drop is the only owning grade.
- **`Box`-like owning indirection.** The `OwningIndirection` marker
  required to ship owning smart pointers isn't implemented.
- **Discard intrinsic.** FFI handoff uses `consuming` parameters; there's
  no separate "tell the type system this is gone" intrinsic.
- **Module / static deinit.** Verifier asserts these never appear.

Each item slots into the existing model as a small additive extension — the
architecture was sized to allow them, not to bake their absence in.

---

## Reference

- `lib/kestrel-ownership/docs/architecture.md` — crate-level internals.
- `lib/kestrel-ownership/src/move_path.rs` — `MovePath`, `MovePathSet`.
- `lib/kestrel-ownership/src/dataflow.rs` — `InitState`, fixed-point.
- `lib/kestrel-ownership/src/move_check.rs` — E500 / E501 emission.
- `lib/kestrel-ownership/src/drop_elab.rs` — `Drop` / `DropIf` insertion.
- `lib/kestrel-mir/src/ty.rs` — `MirTy::copy_behavior` implementation.
- `lib/kestrel-mir/src/passes/verify.rs` — Stage 8 verifier invariants.
- `lib/kestrel-semantics/src/lib.rs` — `NominalCopySemantics` query.
