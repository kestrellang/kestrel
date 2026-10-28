# Indirection — compiler architecture (as built)

> How the shipped feature works, end to end. The *why* is in `README.md`. All
> anchors verified against the tree on 2026-06-16 — re-grep before editing, they
> drift. Function names are stable; line numbers are not.

## The shape in one paragraph

Inference's `solve_member` grows **one lazy arm**: when a member misses on a
concrete nominal wrapper that conforms to `Indirection`, it requeues the member
on the wrapper's `Target` and records a **side-table entry** naming the
`pointeeRef`/`pointeeMutRef` accessors. MIR lowering reads that side table and
**replays** the peel — it calls the accessor(s) to get a `&Target` view and then
projects the member off it. No new MIR instruction, no codegen change, no
mono/witness change: a peeled access bottoms out in the same ref-returning-call
+ struct-extract machinery `arr.at(i).field` already uses.

Everything new lives in five edits:

| Layer | File | What |
|---|---|---|
| 1 | `lang/std/core/indirection.ks` (new) + `memory/*.ks` + `kestrel-hir/src/builtin.rs` | the two protocols, the conformers, `Builtin::Indirection` |
| 2 | `kestrel-type-infer/src/solver.rs` | `try_indirection_peel` + the `NotFound`-arm call site |
| 2 | `kestrel-type-infer/src/generate.rs` | the R7 guard set (`protocol_dispatch_members`) |
| 3 | `kestrel-type-infer/src/result.rs` + `ctx.rs` | `IndirectionPeel` side table |
| 4 | `kestrel-mir-lower/src/body/{expr.rs,call/mod.rs}` | `lower_indirection_chain` + read/write/method replay |
| 5 | `kestrel-analyze/src/body/assignment.rs` | D2 (read-only write → E208) |

## Layer 1 — stdlib + lang item

`Indirection` / `MutableIndirection` are ordinary nominal protocols
(`core/indirection.ks`). `Indirection` is registered as a lang item so the
solver can resolve its entity: a `Builtin::Indirection` variant in
`kestrel-hir/src/builtin.rs` (added to the enum, `name()`, `from_attribute_name()`,
and `kind()` ⇒ `BuiltinKind::protocol()` — non-marker, explicit conformance, the
`Exitable` template), plus `@builtin(.Indirection)` on the protocol. The solver
reaches it via `ResolveBuiltin { builtin: Builtin::Indirection, root }` — the
same registry it reads `Copyable` from. `MutableIndirection`, `pointeeRef`, and
`pointeeMutRef` need **no** builtin handle: they're found by name on the
concrete wrapper (the peel is always concrete — see Layer 2).

`CowBox.inner` had to widen `private` → `fileprivate` so the same-file
`extend CowBox: MutableIndirection` can run the COW barrier on it.

## Layer 2 — inference: the lazy peel arm

### Placement

`solve_member` resolves a member by calling `ctx.resolver.resolve_member(...)`.
Its `Err(MemberError::NotFound)` arm falls back to static-member lookup; when
*that* also misses, the peel fires (just before `member_not_found_error`):

```rust
if !ctx.protocol_dispatch_members.contains(&expr)
    && let Some(pointee_tv) = try_indirection_peel(ctx, &recv_kind, receiver, expr, &span)
{
    return SolveResult::Deferred(Constraint::Member { receiver: pointee_tv, ..verbatim });
}
```

This is the **lazy** twin of the eager `TyKind::Ref` arm higher in the same
function: identical `Deferred(Member { receiver: <pointee>, .. })` shape, placed
at the opposite end of lookup. The eager arm fires *before* any lookup (a ref
has no members of its own); the lazy arm fires *after* the wrapper's own
instance + static lookup both miss — that placement difference **is** the
eager-vs-lazy / wrapper-wins distinction, and is the one subtlety in the feature.

### `try_indirection_peel`

1. Bail unless `recv_kind` is a nominal (`Struct`/`Enum`/`Protocol`).
2. Resolve `Builtin::Indirection`; bail unless
   `type_satisfies(reify_tv(receiver), Indirection)` — bound-aware, so a
   conditional `extend CowBox[T]: MutableIndirection where T: Cloneable` is
   evaluated here.
3. Resolve the concrete `pointeeRef` (required) and `pointeeMutRef` (`Option` —
   `None` ⇒ read-only) member entities on the wrapper, for MIR.
4. Compute the pointee type **by reusing the assoc-type machinery**: a fresh
   `pointee_tv` + an `Associated(receiver, "Target", pointee_tv)` constraint,
   which `solve_associated` resolves. (No re-implementation of assoc projection.)
5. Append a `PendingPeel { read_method, mut_method, target_tv: pointee_tv }` to
   `ctx.indirection_peels[expr]` (append → the outer→inner chain for nesting).
6. Return `pointee_tv` for the caller to requeue the member on.

Mutability (read vs write) is **not** decided here — like the eager arm, the
peel is uniform; read/write routing happens in MIR (Layer 4) and mutability is
policed in analyze (Layer 5).

### The R7 guard (operators never peel)

Operators / for-in / try desugar to `HirExpr::ProtocolCall`, whose
`generate.rs` arm emits *both* a `conforms(recv, Protocol)` gate and a
`member(recv, method)` constraint — so an operator's method lookup *would* reach
the peel. To keep operators receiver-only-and-explicit (R7), the `ProtocolCall`
arm records the expr in a generate-time set:

```rust
ctx.protocol_dispatch_members.insert(id);   // generate.rs, ProtocolCall arm
```

The peel skips any member in that set. This mirrors the existing
`poison_protocol_call_recv_on_failure` pattern (a generate-time → solve-time
signal), and is why `rc1 == rc2` requires `extend RcBox: Equatable` and cleanly
rejects without it instead of silently peeling `isEqual` to the pointee.

## Layer 3 — the side table (single source of truth)

`TypedBody` (`result.rs`) gains:

```rust
pub indirection_peels: HashMap<HirExprId, Vec<IndirectionPeel>>,
pub struct IndirectionPeel { read_method: Entity, mut_method: Option<Entity>, target: ResolvedTy }
```

Accumulated during solving as `PendingPeel` (carrying `target_tv`); `build_result`
resolves each `target_tv` to a concrete `ResolvedTy`. The `target` lets MIR build
the `&Target` accessor-return type without re-deriving the peel. This is the
`ClosureCaptures` / `resolutions` pattern: inference computes a per-`HirExprId`
fact, MIR replays it, never re-derives it.

**Why MIR needs the table at all:** after the requeue, `resolutions[expr]` points
at a member of the *pointee* (`Account.balance`), but the receiver *expression*
is still typed as the *wrapper* (`RcBox`). Unlike the eager `&T` arm — where the
receiver's static type literally *is* `&T`, so MIR sees the ref — an Indirection
receiver is an ordinary nominal, indistinguishable at the MIR seam from a normal
field access. The side table is the explicit signal.

## Layer 4 — MIR: replaying the peel

`lower_indirection_chain(base, peels, mutating)` (`body/expr.rs`) emits the
accessor call(s) and returns the final pointee **view**:

- Level 0 receiver is the wrapper expr (borrowed `Borrow`/`MutBorrow`); each
  later level receives the previous level's view (the nested chain).
- The call passes the **pointee** type (`target`) as `result_ty`, NOT `&Target`.
  The callee's `CallableRefReturn` (`pointeeRef -> &Target`) drives
  `emit_call_inner` to register the result `@guaranteed` *as the pointee view*
  (holding the address) — exactly like `arr.at(i)`. **Passing `&Target` would
  register an `@owned` pointer whose `StructExtract` reads the pointer's bytes as
  the pointee (segfault).** This was the one non-obvious bug during bring-up.

Three call sites consume the chain:

- **Field read** (`lower_field_access`, top): chain with `mutating=false`, then
  `emit_struct_extract(view, field_idx, result_ty)`.
- **Field write** (`lower_assign` → `lower_indirection_field_store`): RHS first,
  chain with `mutating=true`, then `PtrTo(view)` → `FieldAddr` →
  `StoreAssign` (drops the old value), mirroring the `cell.mutatingValue = v`
  store path.
- **Method call** (`call/mod.rs`): chain selected by the receiver convention
  (`MutBorrow` ⇒ mutating), used as the receiver arg; `receiver_ty` is retargeted
  to the pointee so the callee's type args resolve against the pointee.

No codegen, mono, or witness changes — all calls are `Callee::Direct` on the
concrete conformer, and the views feed the existing eager-ref lowering.

## Layer 5 — diagnostics

- **D2** (`assignment.rs`): a write whose target `indirection_peels` chain ends
  with `mut_method == None` (read-only `Indirection`, no `pointeeMutRef`) reuses
  **E208 `assign_through_shared_ref`** — semantically identical (writing through
  a `&T` `pointeeRef()` place). Checked at the top of the `Field` target arm,
  before the settable check (the pointee field *is* settable, so the plain check
  would miss it).
- **D1**: a member on neither wrapper nor pointee surfaces as the ordinary
  member-not-found, reported on the pointee type ("no member 'x' on type
  'Account'"). Naming *both* wrapper and pointee is an unshipped wording polish.

## Known follow-ups

- D1 wording (name the wrapper too).
- `&T` as an intrinsic `Indirection` (unify the eager + lazy arms).
- A pre-existing, unrelated debug-only ICE (`kestrel-mir/src/ty_query.rs`) trips
  on *any* `RcBox`/`Pointer[CopyableStruct]` in a **debug** build — release /
  triage are unaffected. See the `debug_build_ice_pointer_copyable_struct`
  memory; out of scope for this feature.
