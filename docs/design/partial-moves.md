# Partial Moves

Status: **implemented** (2026-10-27), phases 1–5. Design base commit: `8c5d3191`.
Fixes fuzz **M6** and **M8**, and narrows E635 (H12). Related: E511 (C23).

## Problem

A non-Copyable field can only be moved out of an owned value in return
position. Everywhere else, the move is rejected, or treated as a move of
the whole value:

```kestrel
struct R: not Copyable { var id: Int64 }
struct P: not Copyable { var a: R; var b: R }

let p = P(a: R(id: 1), b: R(id: 2));
let x = p.a;        // E503 "cannot move … out of a borrow"          (M6)
take(p.a)           // E503                                          (M6)
let y = t.0;        // E503, tuple element of a consuming param      (M6)
return p.a          // works: the only supported shape
```

```kestrel
let f = { p.a.v };  // view closure freezes p.a.v
take(p.b)           // E507 "cannot consume 'p.a.v' …" — p.b is disjoint (M8)
```

Both bugs have the same cause. Kestrel has no partial moves:

- **MIR lowering.** The only field move-out path,
  `try_move_field_out_of_owned` (`kestrel-mir-lower/src/body/expr.rs`), is
  called from `lower_expr_for_return` only. It takes the whole slot and
  destructures it, so every sibling field becomes a loose SSA value. Any
  other consuming position reads the field as a borrow and hits the E503
  backstop in `emit_copy_value` (`body/mod.rs`).
- **Move checker.** `effective_move_place` (`kestrel-analyze/src/body/move_tracking.rs`)
  widens every field move to the whole root, to stay in lockstep with MIR.
  The widened move then overlaps the field-granular E507 freeze, which is
  M8. The widening is required today: MIR really does take all of `p`.
- **OSSA verifier.** It cannot catch a mistake here. A slot initialized by a
  whole `StoreInit` collapses to `AddrKind::Whole(Init)`
  (`kestrel-mir/src/verify.rs`, `addr_store_init`). After that, operations on
  a `FieldAddr` of the slot are silently skipped (`addr_require_init`,
  `addr_set_uninit`). So a field `Take` followed by a whole `DestroyAddr`,
  which is a double drop, verifies cleanly.

## Decision

Rust's model. A non-Copyable field, tuple element, or nested path can be
moved out of an **owned local**. The remaining fields stay in place, stay
usable, and are dropped individually at scope exit. Run-time per-field drop
flags cover moves that happen on only some paths.

### Language rules

1. **Where.** A move out of a place rooted at an owned local: a `let` or
   `var`, or a `consuming` parameter (including `consuming self`). The place
   can be `p.a`, `p.a.b`, `t.0`, `p.a.0`, and so on, through stored struct
   fields and tuple elements. Moving out of a borrow (a plain or `mutating`
   parameter, or a `&` binding) stays E503, as in Rust.
2. **Not through a `deinit`.** If any type along the path has a user
   `deinit`, the move is E511. This is unchanged from today: a `deinit`
   always sees a whole value. Rust's equivalent is E0509.
3. **After a move, the moved place and everything that contains it are
   unusable; disjoint places stay usable.** After `let x = p.a`:
   - `p.b`, `p.b.c`: fine (read, borrow, move, call methods on them).
   - `p.a`, `p.a.v`: E500 "use of moved value `p.a`".
   - `p` as a whole: reading it, borrowing it, passing it, calling a method
     on it, or capturing it whole is E500 "use of partially moved value `p`".
     The note names the moved field.
4. **Reinitialization.** `p.a = R(id: 3)` puts the field back, and `p` is
   whole again once every moved field is restored. `p = P(…)` replaces the
   whole value: the remaining fields are dropped first, as Rust does. Both
   need `p` to be a `var`, as today.
5. **Conditional moves.** A field moved on some paths is *maybe-moved*.
   Using it, or `p` as a whole, is an error. At scope exit, a run-time flag
   decides whether to drop it.
6. **Loops.** As for whole moves today: moving a field inside a loop body
   without reinitializing it before the next iteration is E500 on the second
   iteration.
7. **`match` on an owned value moves only the matched parts.** A by-value
   pattern that binds a non-Copyable field moves that field and nothing
   else. Fields the pattern ignores (`_`, unmentioned fields) stay in place
   and usable. Today the whole scrutinee is moved.
8. **A `mutating` closure captures only the places it writes.** In
   `w(p.a, { p.b = 3 })` the closure captures `p.b`, not all of `p`, so it no
   longer conflicts with `mutating p.a` (E635).
9. **Out of scope.**
   - Moves out of array elements or through accessors (get/set or `ref`
     members). Those are not stored places.

Diagnostics reuse E500, with the "partially moved" wording when the use
names a containing place. No new code is needed. Rule 3 applies to `self`
in a `consuming` method too: after moving `self.a`, calling another method
on `self` is E500, as in Rust.

### Move checker (`kestrel-analyze`)

`State.moves` is already keyed by `PlaceKey`, and the if/match/loop joins
already work per place. What is missing is recording and checking real
sub-places:

- **Record the actual place.** Delete `effective_move_place`. Record field
  moves at `let`, assignment RHS, `return`, and the body tail too. Today
  these record nothing when the RHS is not a bare local (`rhs_local`).
- **Overlap-aware use check.** A use of place `U` looks for any recorded
  move `M` with the same root where one path is a prefix of the other
  (`PlaceKey::is_prefix_of`, the same test `frozen_overlap` uses):
  - `M` is a prefix of `U`: use after move.
  - `U` is a strict prefix of `M`: use of a partially moved value.

  The `Field`/`TupleIndex` arms check their own place, instead of only
  recursing into the base local as today.
- **Reinitialization.** Assigning to place `T` clears every recorded move
  that `T` is a prefix of. Today a field target recurses with
  `is_assign_target = false` and reports E500 on the base.
- **E507 needs no change.** With real places recorded, a move of `p.b` no
  longer overlaps a freeze of `p.a.v`. That fixes M8.
- **Owning closure captures.** A non-Copyable read capture of `h.r` by an
  `escaping` or `consuming` closure moves only `h.r` (currently widened). The
  test `owning_projected_noncopyable_capture_moves_root.ks` gets
  re-baselined.
- **Match moves** (rule 7) record the bound payload places instead of
  widening to the scrutinee root, in lockstep with MIR's `emit_moveout`.
- **Unchanged:**
  - The one-diagnostic-per-local dedup (`reported`, keyed by root).
  - The nested-closure `force_whole` collapse.

### MIR lowering (`kestrel-mir-lower`)

This generalizes the init-body per-field machinery (`field_inits`,
`init_field_flags`, `store_init_self_field`, `emit_init_partial_drops`) from
"`self` in an initializer" to "any slot-bound local".

- **One move-out path.** A consuming use of a non-Copyable place rooted at
  an owned slot emits `FieldAddr` chain → `Take`. It is used in every
  consuming position: `let`, consuming argument, constructor operand,
  `return`, and the body tail. `try_move_field_out_of_owned` and its
  whole-take-and-destructure trick are deleted.
- **Owned SSA roots.** `consuming` parameters bound as owned SSA values get
  a slot when the body partially moves them. A HIR pre-scan per body finds
  the locals with field moves; those params are spilled into a slot at
  entry. Everything else keeps its current binding, so code without partial
  moves lowers exactly as before.
- **Per-field state.** `ScopeEntry::Var` gains
  `moved: Vec<(FieldPath, VarInit, Option<flag>)>`. It holds only paths that
  are moved somewhere in the body, so it is empty for almost every local.
  `FieldPath` is a list of field indices, so nested paths and tuples are
  covered. The same pre-scan supplies the paths, so each flag is allocated
  where the variable is declared. A flag must dominate every scope exit;
  this is the same constraint `maybe_alloc_var_flag` already meets.
- **Joins.** `ArmExit.var_inits`, `fold_var_inits`, `break_var_inits` and
  `snapshot_scope` carry the per-path entries, joined with `VarInit::join`
  as the whole-var state is today.
- **Scope exit.**
  - A local with no moved path destroys the whole slot, as today.
  - Otherwise it drops each remaining field individually, recursing into
    partially moved sub-paths. The order is reverse declaration order, as
    in the drop shim.
  - Each field: `FieldAddr` + `DestroyAddr{field_ty}` if it is DefInit; the
    same inside `emit_guarded_destroy` if it is MaybeUninit; nothing if it
    is DefUninit.

  This is `emit_init_partial_drops` generalized. Per-field `DestroyAddr` is
  already supported end to end: mono collects the field type's shim, and
  expand rewrites the destroy.
- **Writes.**
  - Field reinitialization (`p.a = v` on a moved field) is `StoreInit`, not
    `StoreAssign`. `StoreAssign` would drop the garbage left behind. A
    maybe-moved field gets a guarded destroy first, then `StoreInit`. This
    is `store_init_self_field` generalized.
  - A whole reassignment of a partially moved local drops the remaining
    fields per field, then does a whole `StoreInit`.
- **No user deinit is involved.** Rule 2 guarantees no type on the path has
  one, so skipping the struct's drop shim is sound. The shim only destroys
  the fields, in reverse order.

### Field-granular write captures (rule 8)

This part is independent of drop tracking, so it gets its own phase. It
changes how a view closure's environment points into the frame, not what
gets dropped.

- **Today:** `plan_slots` (`kestrel-mir-lower/src/body/closure.rs`) keeps
  a projected place as its own capture slot only for a duplicable
  read-only capture. A write capture is collapsed to the whole local.
- **Change:** a view closure (normal or `mutating`) captures a written
  place by its field address. The parent side already does this:
  `view_addr_of_place` → `try_field_addr_chain`. The closure body's
  `place_capture_map` then routes writes through that pointer as well as
  reads.
- **Analyzer:** `exclusive_args.rs` (E635) uses the captured place itself
  instead of `PlaceKey::whole(root)`. The move checker's freeze set for
  `mutating` closures uses the written place too.
- **Owning closures** (`escaping`, `consuming`) are unaffected. They own
  their environment and never write back to the frame.

### OSSA verifier (`kestrel-mir`)

The verifier has to model this precisely before lowering emits it.
Otherwise every bug in the lowering verifies cleanly, as shown above.

- **Do not collapse a struct or tuple slot to `Whole` on a whole
  `StoreInit`.** Mark every field Init instead. A field take then updates
  that field.
- Extend `AddrAliases::field_of` from `(root, field)` to `(root, path)`, so
  nested `FieldAddr` chains are tracked. Today a nested chain is untracked.
  Tuple slots use the same representation.
- **Whole-slot operations on a partially uninit slot are errors:**
  - Read, `Take`, `BeginBorrowAddr`: "slot partially moved".
  - `DestroyAddr`: "destroy of partially moved slot".
- **Check at function exit.** Every tracked slot must be fully Uninit, or
  only MaybeInit behind a flag. This catches a forgotten sibling drop
  (a leak). Today nothing checks slot state at exit.
- **Borrow conflicts stay as they are.** M8's case is a view of `p.a.v`
  alive across a `Take` of `p.b`. The exact-ValueId checks already allow
  that, and `check_destroy_while_ref_live` only matters at scope exit, after
  the view is dead. Disjoint-projection borrow checking is not required for
  this design.

### Backends and mono passes

- **Backends:** no change. `FieldAddr` is an offset, and `Take` from a
  field address is a load or memcpy. Neither backend assumes a slot is
  initialized all or nothing.
- **Optimization passes need tests, not new features:**
  - `mark_independent_takes` already roots field addresses at their slot.
  - `owned_params` already makes field-level takes.
  - `shorten_let_slots` bails on any take through the slot, so a partially
    moved slot simply isn't optimized.

## Costs

- Most of the new code is in MIR lowering: per-path state through scopes,
  joins and loop exits. Large parts of it already exist for init `self`
  and get moved and generalized.
- Verifier precision may expose existing lowering bugs once whole stores no
  longer collapse to `Whole`. That is intended, but it can produce
  unrelated failures during the change.
- Locals that are never partially moved lower exactly as today. The
  pre-scan keeps flags and per-field state off them.

## Plan

Each phase leaves the suite green:

1. **Verifier precision.** No `Whole` collapse, path-keyed fields, whole-op
   and exit checks. Fix any existing lowering bugs it exposes. No language
   change yet.
2. **MIR partial moves.** Pre-scan, per-path state and flags, the single
   move-out path, per-field scope-exit drops, field `StoreInit`. Delete
   `try_move_field_out_of_owned`. The analyzer still widens, so this phase
   only enables M6: field moves out of an untouched value.
3. **Move checker.** Delete `effective_move_place`, add the overlap-aware
   use check and field reinitialization. This fixes M8. Re-baseline the
   pinned tests listed below.
4. **Owning projected captures and match moves.** An owning closure moves
   only the captured projection. `match` moves only the matched parts
   (rule 7).
5. **Field-granular write captures** (rule 8). Narrows E635. This phase
   does not depend on the others and can be done at any point.

## Tests

Execution tests use a counting `deinit` on the field type, so every case
asserts that each field is dropped exactly once:

- `let x = p.a`; `take(p.a)`; `let y = t.0`; nested `p.a.b`; a `consuming`
  param; `consuming self`.
- Moved in one `if` arm only (flag path). Moved in a `match` arm. Moved then
  reinitialized; moved then reinitialized on one path only.
- A whole reassignment of a partially moved `var`.
- M8: a view of `p.a.v` alive while `p.b` is moved.
- A generic field type `T` (mono-dependent copy).

Diagnostics tests:

- Use of `p.a` and of `p` after `let x = p.a` (E500, with "partially moved"
  wording).
- Maybe-moved use after an `if`.
- E511 through a nested path whose middle type has a `deinit`.
- E503 for a move out of a borrowed or `mutating` param's field.

MIR tests: a scope exit with per-field `DestroyAddr`s, and a guarded
destroy for a maybe-moved field.

Verifier unit tests: a field take followed by a whole destroy is rejected;
a forgotten sibling drop is rejected.

Pinned tests to re-baseline (user approval needed per CLAUDE.md):

- `memory_model/closure_kinds/escaping/owning_projected_noncopyable_capture_moves_root.ks`
- `memory_model/copy_semantics/move_field_out_of_let_local.ks`: its MIR shape
  changes from take-and-destructure to a field take.
- `memory_model/drop_elab/14_struct_with_non_copy_field.ks`: root-only
  move paths.

## Implementation notes

Where the implementation differs from the plan above, and what stays whole:

- **No function-exit check in the verifier.** Slots of Copyable types are
  never destroyed (their drop is a no-op), so "every slot is uninit at exit"
  would flag every Copyable local. The verifier's precision is the per-path
  slot tree (`SlotState` in `kestrel-mir/src/verify.rs`): a whole read, take,
  destroy, or `StoreAssign` of a partially moved place is an error.
- **Field-move planning** is `kestrel-mir-lower/src/body/partial_move.rs`:
  one pass per body finds the projections in consuming positions (plus
  owning-closure captures and part-moving matches) and allocates a presence
  flag per moved path where the variable is declared.
- **Shared rules** live in `kestrel_type_infer::captures`, used by both MIR
  lowering and the move checker: `field_capturable_roots` (owning closures, phase 4) and
  `match_bound_places` (rule 7).
- **`let` destructuring** of a place without a type annotation matches the
  place directly (`let (x, _) = t` is `match t { … }`), so it moves only the
  bound parts; with an annotation it still goes through a temporary.
- **Still moved whole:** match patterns that reach a binding through an enum
  variant (the payload has no place of its own), an array, or an `@`
  binding; calling a `consuming` closure stored in a field (`(h.f)()`); and
  moves out of a `mutating` parameter's field (the caller owns that storage;
  still E503).

## Decisions

1. Calling a method on a partially moved `self` in a `consuming` method is
   an error (rule 3), as in Rust.
2. "Use of partially moved value" reuses E500.
3. `match` on an owned value moves only the matched parts (rule 7).
4. Field-granular write captures are included (rule 8, phase 5). They are
   a separate phase because they change closure environments, not drop
   tracking.
