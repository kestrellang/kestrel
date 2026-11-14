# Closures root-cause analysis (no fixes)

Base: `/home/user/kestrel` @ `3fea2041eef543c99ac62c296e1fbba8b30af401`, prebuilt `target/release/kestrel`.
All repros were re-run at this commit, and the behavior matches REPORT.md (C9 is the exception: see below).

## Group E: view-closure escape holes (C18, C19, C20, C21, C22)

These five share one cause. The **stage-2b escape-taint plumbing was written for refs, gated on
`ty_arena.contains_ref(ty)`, and was only partly widened to closures for #174**. The widened sites are
copy/move (`carry_ref_taint`), struct/tuple (`collect_by_value_escape_taint`) and slot store/load
(`taint_slot_from_store` / `inherit_slot_taint`). Every site still gated on `contains_ref`, or never
widened, drops the `apply_partial` taint, so the returned value looks self-rooted and `check_escapes`
(E494, `lib/kestrel-mir/src/verify.rs:2384`) skips it. `contains_ref(FuncThick)` is false
(`lib/kestrel-mir/src/ty.rs:252`). The right predicate already exists: `module.escape_carry(ty).any()`.
A second gap in the verifier: `merged_roots` (verify.rs:2408) runs a fixpoint over block params
only. It does not flow through `take`/`load`/`copy_value`/`begin_borrow_addr` of a slot, or
through a store into a slot. So a taint that arrives only over a back edge can never reach the returned load.

| ID | Where the taint is lost | Location |
|---|---|---|
| C18 `.Some({k})` | `emit_enum_variant` only joins taint for **ref slots**. It has no `collect_by_value_escape_taint` call on by-value payloads, although its siblings `emit_struct` and `emit_tuple` have one. | `lib/kestrel-mir-lower/src/body/mod.rs:2526-2553` (`emit_enum_variant`) |
| C19 if-join | (a) Block-param threading of the closure **slot** `Pointer[@thick..]` drops the slot taint, because the `carries_taint` gate is `contains_ref(ty) \|\| Pointer(contains_ref)`. The copy inside the arm is then self-rooted. (b) The `if` result-param join is gated on `contains_ref(result_ty)`. | `body/mod.rs:1935-1953` (`rebind_scope_values`); `body/control.rs:162` (`lower_if`); same gate at `body/pattern.rs:278` (`lower_match`) |
| C20 if-reassign | `store_assign` into the in-arm slot param taints that param. The join-block slot param is rebound through the same `contains_ref` gate, so the trailing `take` is untainted. | `body/mod.rs:1935` |
| C20 loop-reassign | The store happens in the body after the loop header param was created. The taint reaches the exit param only over the back edge. Lowering-time stamping is single-pass, and verify's `merged_roots` does not propagate into the `take %exit_slot` that is returned (MIR: `return %v122 = take %v46`). | `body/mod.rs:1935` + `lib/kestrel-mir/src/verify.rs:2408-2445` (`merged_roots`) |
| C21 `[{k}]` | **Primary:** an un-annotated array literal takes its default `Array[?E]` through `ctx.named` with **no `emit_static_wellformedness`**, so the implicit `T: Static` on Array is never checked against `E = () -> Int64`. `let a: Array[() -> Int64] = [{k}]` *is* rejected (E107). The trace `KESTREL_DEBUG=static-wf` shows the Static wf firing only for the annotated form. **Secondary (MIR):** `PtrWrite`/`Array.init` do not taint, and the `subscript` call result is unstamped (call gate below). | `lib/kestrel-type-infer/src/solver/defaults.rs:433-463` (`default_literal_var`); compare `solver/lower.rs:248` |
| C22 call result | An owned call result is stamped with the join of its borrowable args only when `contains_ref(ty)`. `pick({k},{0}) -> () -> Int64` is therefore self-rooted. | `body/mod.rs:3317,3327` (`emit_call_inner`) |

Confidence: high for C18, C19, C20 and C22, all checked against MIR dumps. High for C21's front-end
miss. Medium on whether the MIR backstop alone would be enough for C21.

Fix shape (one change for the group): replace every escape-taint `contains_ref` gate with
`escape_carry(ty).any()` (rebind_scope_values, lower_if, lower_match, emit_call_inner), add the
by-value arm to `emit_enum_variant`, and make verify's `merged_roots` a whole-body forward fixpoint that
also flows through slot store→load/take/copy/borrow_addr (this covers loops). Separately for C21, call
`emit_static_wellformedness` in `default_literal_var`.

## A1: returning an `escaping` param as a normal closure is rejected (E494)

- Root cause: an `Escaping→Normal` `coerce_fn_kind` deliberately re-roots the truncated view at
  `Local(operand)` for every source root except `Static`, which includes a borrowed `Param(i)`. The
  returned view therefore always reads as frame-local. A plain normal-closure param keeps `Param(i)`
  and is accepted.
- Location: `lib/kestrel-mir-lower/src/body/expr.rs:279-293` (the coerce-fn-kind lowering; see the comment
  "even when that backing is a borrowed parameter owned by the caller").
- Why it is missed: the rule was intentionally conservative, and nothing distinguishes a borrowed param, whose handle is still owned by the caller, from a local.
- Confidence: high on the mechanism. Medium that the language intends this to be valid. A view of a caller-owned box is sound under the Param-root rule.
- Fix shape: keep `Param(i)` when the source root is a borrow-convention param, and stamp `Local` only for local or consuming roots.

## C7: closure with a borrowed param that is consumed in its body → heap corruption

- Root cause: a closure literal lowers every non-`MutBorrow` param, `Borrow` included, as an
  **@owned, `Consuming`** SSA param. Indirect calls pass every arg `@borrow`
  (`lower_call_args_default`). In `{ it + "b" }`, `Addable.add` consumes `%v1` (`it`) while the caller
  still owns and destroys it, which is a double free (`malloc(): unaligned tcache chunk`). Both repros
  share this closure body.
- Location: `lib/kestrel-mir-lower/src/body/closure.rs:315-326` (`lower_closure_expr` param loop, the `_ =>` arm),
  against `lib/kestrel-mir-lower/src/body/call/mod.rs:1060-1080` (`lower_indirect_call`). The thunk inherits
  the owned signature (`thunk(%v0, %v1: @owned String)`).
- Why it is missed: OSSA verify sees a self-consistent callee, because the param is "owned". The ABI mismatch spans two functions, so no check sees both sides.
- Confidence: high.
- Fix shape: bind `Borrow` params as `@guaranteed` with a `Borrow` ParamDef, mirroring the function-param `Borrow` arm at `body/mod.rs:620-655`. A consuming use then emits `copy_value`.

## C8: `consuming` closure loses writes to its captures

- Root cause: the unique (`consuming`) tier destructures the boxed env into per-capture values and
  binds each as `LocalBinding::Ssa` (or `PlaceCapture::Value`). A write to a captured var then lowers
  against an immutable SSA value. `c.n = 5` becomes `copy_value %v7` and the write lands in the copy
  (`r=0`). `parts.append` becomes `begin_mut_borrow %v7` on an SSA value, so the write is not
  persisted and the closure then reads the stale value (`n=1`). The outer `parts.count == 2` is a
  downstream effect of that lost-write mutation on a COW handle. The capture is properly `clone()`d
  post-mono, so the mechanism behind the outer count is only medium confidence.
- Location: `lib/kestrel-mir-lower/src/body/closure.rs:372-392` (`lower_closure_expr`, consuming tier). The escaping tier (`:393-407`) binds `Var(addr)` and works.
- Why it is missed: the design comment assumed captures are only read or moved out ("each slot is now an ordinary local") and that writes are covered by the move machinery. A `let`-style SSA local is not writable.
- Confidence: high for the lost write. Medium for the exact outer-count mechanism.
- Fix shape: materialize each destructured capture into a frame slot (`uninit` + `store_init`) and bind it as `LocalBinding::Var` / `PlaceCapture::Addr`, at least for Write captures.

## C9: not a bug (expected behavior)

- `{ () in a.id }` captures the **place `a.id`** (Int64, Copyable), not `a` (closures.md §"What is
  captured: the narrowest place", and the owning-capture table: a Copyable place is bit-copied and its
  source is untouched). MIR: `copy_value` of the `a.id` field into the RcBox env. `a` is then dropped at
  `mkf` exit, so "drop 1" before "call 1" is correct, and the closure reads its own copy (1). No
  read-after-free happens. Confidence: high. Recommend reclassifying.

## C15: owning capture of an already-moved value is never checked (loop and straight-line)

- Root cause: the `HirExpr::Closure` arm of the move checker records each non-Copyable owning
  capture with `record_move`. That function checks the freeze and E506 but **never checks whether the
  place is already moved**. The closure body itself is analyzed with `State::empty()`, so its use of
  `a` cannot see outer moves either. The loop back-edge re-analysis (#163) seeds the moves correctly,
  but nothing consults them at the capture site. The bug is wider than loops: `take(a); let f:
  consuming () -> () = { take(a) }` also passes analysis and then ICEs in OSSA ("address already
  uninit"). In the loop form it runs with a triple drop.
- Location: `lib/kestrel-analyze/src/body/move_tracking.rs:807-811` (`analyze_expr`, Closure arm), `:762` (`State::empty()`), `:1391` (`record_move`).
- Why it is missed: the ordinary read path (`emit_use_after_move`, `:2247`) is reached only through a `Local` read in the *enclosing* state, and a capture is not walked as such a read.
- Confidence: high (verified with the straight-line variant).
- Fix shape: for each captured place, view kinds included, run the E500/E501 moved-state check against the enclosing `state` before `record_move`.

## M8: E507 blocks consuming the disjoint field `p.b` while `p.a.v` is captured

- Root cause: the freeze set is field-granular (`p.a.v`), but the *move* is widened to the whole root
  before the freeze check. `record_operand_move` → `effective_move_place` = `PlaceKey::whole(root)`, because MIR
  "moves a non-Copyable projection by taking and destructuring its whole root". So `p` overlaps `p.a.v`.
  The diagnostic even names the frozen place ("cannot consume 'p.a.v'") at the `p.b` site.
- Location: `lib/kestrel-analyze/src/body/move_tracking.rs:1065-1081` (`record_operand_move` / `effective_move_place`) → `check_freeze` `:1500` / `frozen_overlap` `:1489`.
- Why it is missed: this is a deliberate lockstep with a MIR limitation, not a freeze bug. Narrowing only the analyzer would be unsound, because MIR would still take the whole of `p` under a live view of `p.a`.
- Confidence: high.
- Fix shape: MIR partial moves (field-granular take/drop flags), then drop the `effective_move_place` widening.

## M9: E507 is still active after an immediately-invoked closure

- Root cause: the Closure arm inserts view freezes into `state.frozen`, and they are released only at
  **block** exit (`restore_frozen` in `analyze_block`). A temporary closure (`{ r.v }()`) carried by
  no binding therefore keeps `r.v` frozen to the end of the enclosing block. closures.md says the
  freeze runs to the end of each carrying binding's *or temporary's* lexical extent, which for a
  temporary is the statement.
- Location: `lib/kestrel-analyze/src/body/move_tracking.rs:782-794` (freeze insert), `:276-306` (`analyze_block`), `:314` (`restore_frozen`). `analyze_stmt` (`:334`) has no endpoint.
- Confidence: high.
- Fix shape: at the end of each statement, drop freezes introduced in it that no live carrier binding holds (the same `carriers` test as `restore_frozen`, at statement granularity).

## H12: `two(x, { x })` with `mutating a`, which gives an OSSA "active mut borrow" ICE

- Root cause: args are lowered left to right. `begin_mut_borrow_addr %x` for the `mutating` arg opens first, and then the
  view closure's capture (`view_addr_of_local` → `emit_value_use` = `copy_value` of x's slot) reads
  the slot under that active mut borrow. No front-end rule rejects a `mutating` argument overlapping a
  place captured by a view-closure argument of the same call. `snapshot_overlapping_borrows` handles
  plain borrowed args only, and a view closure cannot be snapshotted.
- Location: `lib/kestrel-mir-lower/src/body/closure.rs:683-692` (`view_addr_of_local`); `body/mod.rs:2895` (`snapshot_overlapping_borrows`); no corresponding check in `lib/kestrel-analyze/src/body/access_mode.rs` or `move_tracking.rs`.
- Confidence: high on the mechanism. Choosing between reject and allow is a design decision.
- Fix shape: add an analyzer exclusivity diagnostic for "a view closure capturing a place overlapping a `mutating` arg of the same call". Alternatively, lower closure args before opening mut borrows. That would only move the aliasing to runtime, so the diagnostic is preferred.

## H13: `gen[T: Cloneable]` at an escaping closure → post-mono "does not implement clone"

- Root cause: the front end accepts `escaping (..)->T: Cloneable` (closures.md: escaping is Cloneable, and
  `concrete_copy` maps FuncThick Escaping → `Clone`). However, an explicit `x.clone()` stays a
  `Callee::Witness` and mono's witness table has entries only for nominals (stdlib `extend`s + clone
  shims). There is no structural witness for `FuncThick`, so mono verify reports the surviving witness call.
- Location: `lib/kestrel-mir/src/mono/witness.rs:398` (`resolve_witness_call` → `find_witness_with_method`); `lib/kestrel-mir/src/passes/clone_shim.rs:33` (shims nominals only); reported at `lib/kestrel-mir/src/mono/verify.rs:497-518`.
- Confidence: high.
- Fix shape: in mono, resolve `Cloneable.clone` on an `Escaping` FuncThick (and other structural Clone types) to a `copy_value` (handle retain), the same way expand handles `CopyBehavior::Clone`.
