# kestrel-ownership — architecture

This crate owns Kestrel's MIR-level ownership story: **move-checking** (E500 / E501)
and **drop elaboration**. It runs after `kestrel-mir-lower` produces the initial
MIR and before the final verifier and codegen passes.

```
                   kestrel-mir-lower
                          │
                          ▼
                ┌──────────────────────┐
                │ MovePathSet::build   │  per-function move-path interning
                └──────────┬───────────┘
                           ▼
                ┌──────────────────────┐
                │ dataflow::run        │  forward init / maybe-init fixed point
                └──────────┬───────────┘
                           ▼
        ┌──────────────────┴──────────────────┐
        ▼                                     ▼
   move_check::run                       drop_elab::run
   (E500/E501 emit)                  (Drop / DropIf insertion)
        │                                     │
        └──────────────────┬──────────────────┘
                           ▼
                    final MIR → codegen
```

The crate exposes a single entry point that runs all three passes in order:
`kestrel_ownership::run(&mut MirModule) -> Diagnostics`.

---

## Module map

| File | Responsibility |
|------|---------------|
| `lib.rs` | Public entry point, `Diagnostics` collector |
| `move_path.rs` | `MovePathId`, `MovePathSet`, per-function path interning |
| `dataflow.rs` | `InitState`, forward bit-set dataflow with worklist fixed-point |
| `move_check.rs` | Walk reads against dataflow state, emit `MoveDiag` |
| `drop_elab.rs` | Insert `Drop` / `DropIf` at scope-exit and reassign sites |

The three passes share `MovePathSet` and `dataflow::DataflowResult` — built once
per function, consumed by both move-check and drop-elab.

---

## Move paths

A *move path* is the unit at which ownership is tracked. Stage 7's implementation
is root-local-granular: each non-parameter local with a non-`Copy` type gets one
`MovePathId`. `Place::Local(l)` and any projection chain rooted at `l`
(`l.f.0`, `l.field`) all fold to the same path.

This is sufficient for the canonical move idioms: full-local moves, scope-exit
drops, reassignment pre-drops. Field-level partial moves (where `s.a` is moved
but `s.b` is not) currently surface as full-local moves — a known precision
loss tracked for a future stage.

Paths are interned per function and never escape `MovePathSet`. There's no
global / cross-function path identity.

**Skip rules** (`MovePathSet::build`): a local is *not* given a path when its
type's `copy_behavior(module) != None` (the type is trivially copyable or
Cloneable — moving is illegal at the type level), or when its type contains
`MirTy::TypeParam`, `MirTy::SelfType`, `MirTy::AssociatedProjection`, or
`MirTy::Error`. The unresolved cases are conservatively treated as "not move
paths" because we can't decide their copy semantics without monomorphization;
treating them otherwise produced false positives in generic stdlib code.

---

## Dataflow

`dataflow::InitState` tracks two parallel bit-sets per program point:

- `def_init` — paths *DefinitelyInit* (initialized on every CFG path leading
  here).
- `may_init` — paths *MaybeInit* (initialized on at least one CFG path).

Plus a `move_sites: HashMap<MovePathId, Span>` carrying the kill-site of each
moved-out path, used as the secondary label on E500 / E501 diagnostics.

### Transfer functions

| Operation | Kill | Gen |
|-----------|------|-----|
| `dest = move p` | `path(p)` | `path(dest)` |
| `dest = copy p` | — | `path(dest)` |
| `dest = ref p` / `ref var p` | — | `path(dest)` |
| `call(...args)` with `Value::Move(p)` arg | `path(p)` | `path(dest)` if any |
| `call(...args)` with `Value::RefMut(p)` arg | — | `path(p)` (out-param init) |
| `Drop(p)` / `DropIf(p, flag)` | — (compiler-inserted) | — |

The `RefMut`-as-init rule promotes the path to `DefinitelyInit` *after* the
call — this models Kestrel's out-parameter idiom where `File.init(ref var %t,
fd)` writes into an as-yet-uninit `%t`. The arg is checked-as-read by
`move_check` *before* the transfer, so an `RefMut` of an already-init place
trips a read on a moved value as expected.

### Worklist

Standard forward worklist:

1. Seed the entry block's entry with parameters marked `DefinitelyInit`.
2. Pop a block, apply statements + terminator transfer to derive its exit
   state.
3. Store the new exit; for each successor:
   - If the successor has never been visited, seed its entry with this exit
     verbatim.
   - Otherwise, join (intersect `def_init`, union `may_init`).
   - If the successor's entry changed, push it onto the worklist.
4. Loop until the worklist is empty.

**Termination invariant:** the lattice is finite and monotone in the bit-sets
(intersection only shrinks `def_init`, union only grows `may_init`), so each
block's entry can change at most `|paths|` times. Fixed-point is reached in
`O(|blocks| × |paths|)` worst-case iterations.

**First-visit propagation** (subtle, ex.-bug-driven): propagation is *not*
guarded on `state != blocks[bi].exit`. The default exit is `InitState::empty()`,
so a block whose statements happen to touch only un-tracked locals would
compute an empty exit, match the default, and never seed its successors —
leaving the rest of the CFG stuck at default-uninit. Termination is
governed by per-successor `entry_changed` instead.

---

## Move-check (E500 / E501)

For each block, `check_block` threads the block's entry state through its
statements, applying the transfer function statement-by-statement. Before
applying each statement's transfer, it walks every place the statement *reads*
and consults the current dataflow state:

- Path is `DefinitelyInit` → silently pass.
- Path is `MaybeInit` but not `DefinitelyInit` → emit **E501** (`value 'X' may
  have been moved`). The kill-site span comes from the move-site map.
- Path is neither (`Uninit`) → emit **E500** (`use of moved value 'X'`).

### What counts as a read

The split between read and non-read at the MIR level:

| `Rvalue` / `Value` | Counts as read? |
|---|---|
| `Copy(p)` | Yes |
| `Move(p)` | Yes |
| `Ref(p)` | Yes (immutable borrow of uninit is nonsensical) |
| `RefMut(p)` | **Yes iff `p` is currently MaybeInit** — uninit case is the out-param init |
| `Const(_)` | No |
| `Op1` / `Op2` / `Op3` / `Tuple` / `Construct` / `EnumVariant` / `ArrayLiteral` / `ApplyPartial` | Recurses into operands |

The `RefMut` split is a heuristic recovering what `&out T` would tell us
explicitly: a mutable borrow of an uninit place is an initialization site,
not a read. Once MIR distinguishes `&out T` from `&var T`, this heuristic
collapses to "RefMut is always a read".

### One-per-path

A `reported: HashSet<MovePathId>` shared across all blocks of one body
ensures at most one diagnostic per path per function — matching the prior
HIR tracker's "one error per local" rule, so a chain of reads after a move
doesn't produce a wall of cascading errors.

### Diagnostic shape

```rust
pub struct MoveDiag {
    pub kind: MoveDiagKind,          // UseAfterMove | MaybeMoved
    pub local_name: String,           // for "use of moved value 'X'"
    pub use_site: Span,               // primary label
    pub move_site: Option<Span>,      // secondary label (where it was moved)
}
```

The wording is frozen at `use of moved value 'X' [E500]` / `value 'X' may have
been moved [E501]`. The test-suite harness in `kestrel-test-suite` converts
these to `AnalyzeDiagnostic` for the existing `// ERROR:` annotation matcher.

---

## Drop elaboration

`drop_elab::run` inserts `Drop` / `DropIf` statements at the points where
owned values go out of scope or get reassigned.

The current implementation is "drops at return" granularity: for each affine
local that is `MaybeInit` at the `Return` terminator, insert either an
unconditional `Drop(p)` (if `DefinitelyInit`) or a `DropIf(p, flag)` (if
`MaybeInit`). For the `MaybeInit` case, a synthetic boolean flag local
`_init_<path>: Bool` is allocated and stamped to `true` after each gen and
to `false` after each kill. The flag's runtime value at the drop site
decides whether the deinit actually runs.

The "drops at scope-exit" granularity from the original plan is the natural
next step but defers to a future stage — Kestrel's current MIR doesn't carry
a scope tree, so all drops fold to return-time. The model is correct (no
leaks, no double-drops) but less precise than ideal (drops live longer than
strictly necessary).

### Per-path drop flags

The flag pattern is the standard "MaybeInit needs a runtime flag" trick:

```text
let p: Foo;
let _init_p: Bool = false;
if cond {
    p = Foo();              // also: _init_p = true
}
// ...
DropIf(p, _init_p)           // deinit only if the conditional path took
```

DropElab walks the dataflow result, identifies paths where
`MaybeInit \ DefinitelyInit` at any point, and allocates flag locals for
each. The `move_check.rs` `OWNERSHIP_TRACE` env vars were the original
investigation harness used while building this out; they're gone now,
having served their purpose.

---

## Verifier invariants (Stage 8)

`kestrel-mir::passes::verify` enforces two ownership-related invariants
post-drop-elab:

1. **Panic-edge:** any block whose terminator is `Panic(_)` or `Unreachable`
   must contain no `Drop` / `DropIf` statements. Per architecture, panic
   = abort, so drops on a panicking path are unreachable code — and their
   presence indicates a DropElab placement bug.
2. **Module-init absence:** `__init$<name>` functions (synthetic static-init
   bodies) must contain no `Drop` / `DropIf`. Module / static deinit is
   explicitly out of scope, so emitting drops there is a lowering bug.

Pre-drop-elab MIR is independently verified to contain *zero* `Drop` /
`DropIf` — `kestrel-mir-lower` must never emit them; that's strictly
DropElab's job.

---

## What this crate does *not* own

- **`CopyBehavior` / `DeinitBehavior` computation.** Those are computed by
  `kestrel-mir-lower::{struct_lower, enum_lower}` from
  `kestrel_semantics::NominalCopySemantics` and the user `deinit` method.
  This crate consumes them via `MirTy::copy_behavior(module)`.
- **`Place` lookup / type tracking.** All type queries go through the
  `MirModule` borrow handed to `run`.
- **Region / lifetime checking.** `&T` and `&var T` are Bitwise-Copy here;
  borrow-region analysis is a Phase 2 problem.
- **Linear types.** Currently the only ownership grade is "affine, with
  drop". A linear-type extension would slot in as an extra category in
  `CopyBehavior` and a stricter rule in `drop_elab`, but is not built.
