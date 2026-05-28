# kestrel-mir-lower-3

HIR → OSSA MIR lowering. This file documents known rules and invariants.
It is not exhaustive — when you discover a new rule, add it here.

## Debugging

- `kestrel dump mir <file> --std lang/std -f Foo.bar` dumps one function's
  post-lowering MIR (substring match on function name).
- `VERBOSE_DEBUG_OUTPUT=1` enables `debug_trace!` in the binder/semantic tree
  crates but not in this crate. Use the `kestrel-debug` crate for tracing here.

## Failable / throwing inits

`init?` and `init throws` bodies write fields into `self` and return
`Optional[()]` or `Result[(), E]`. The **caller** must unwrap this:

1. Allocate `StackAlloc(T)` — the inner type, not `Optional[T]`.
2. Call with `emit_call_returning` → captures the `Optional[()]` return.
3. `emit_discriminant` on the return value → I32.
4. `emit_switch` on the discriminant: success → `emit_take` + wrap in
   `Optional[T].Some`, failure → `Optional[T].None`.

Detection: check for `InitEffect` component on the init entity. See
`emit_failable_init_call` in `body/call/mod.rs`.

## Live value threading through branches

When emitting branches (if/else, switch, failable init), all live @owned
values must be threaded through block parameters. The pattern:

1. `saved_tracker = self.tracker.clone()`
2. `self.tracker = LiveTracker::from_live(&self.all_live_tracked())`
3. `live_vals = self.tracker.values()`, `descs = self.tracker.descs()`
4. Create blocks with `new_block_with_params(&descs)`
5. Emit terminator with `live_vals.clone()` as args to each arm
6. In each arm: `switch_to`, `rebind_scope_values`, do work, `emit_jump`
7. Restore: `restore_scope`, `switch_to(merge)`, `rebind_scope_values`
8. `self.tracker = saved_tracker; self.tracker.rebind(...)`

Values created before the branch (like a StackAlloc pointer) get new
ValueIds in each arm via block params. Use `live_vals.iter().position()`
to find the rebound version.

## Irrefutable destructures (let/param patterns)

`let (a, b) = expr` desugars in the HIR to a `Match` with
`MatchSource::LetDestructure` and a single arm whose body is `()`.
The normal `lower_match` path uses scope snapshot/restore, which
**wipes** the `local_map` entries created by `emit_bindings` — values
created inside the match arm become unreachable after the merge.

Fix: `lower_match` detects `LetDestructure` / `ParamDestructure` with
one arm and calls `lower_irrefutable_destructure` instead. This emits
bindings directly in the current block — no branching, no snapshot,
no restore — so `local_map` entries survive for subsequent code.

## Trust `resolve_expr_type` — but it depends on inference recording the expr

`resolve_expr_type` returns `Error` whenever inference left no `expr_types`
entry for the expression. That is usually an **upstream bug in
`kestrel-type-infer/generate.rs`**, not something to paper over here: any
branch of `gen_expr` that does an early `return` must call
`ctx.expr_types.insert(id, tv)` itself, because the generic insert only runs on
the fall-through path. A historical instance: generic enum-case constructors
(`Wrapper[Wrapper[Resource]].Some(..)`) early-returned without inserting, so the
constructed value reached MIR as a bare `Named` with no type_args and then
failed drop-shim monomorphization (`type arg arity mismatch for __drop$Enum`).
Fixed at the source. When you see `resolve_expr_type` → `Error` for a
well-typed program, look upstream first rather than reconstructing the type
from HIR here.

## var_locals

`var_locals` are mutable locals stored at stack addresses via `uninit` +
`store_init`. `map_local()` returns the **address**, not the value. Code
that reads a var_local must load from the address (`emit_copy_addr`).
Closures capturing var_locals must snapshot the value, not capture the
raw address.
