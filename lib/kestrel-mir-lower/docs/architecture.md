# kestrel-mir-lower Architecture

Lowers typed HIR to OSSA (ownership-SSA) MIR. Consumes ECS declarations, `HirBody`, and inference results; produces a `MirModule` ready for the MIR pass pipeline (drop elaboration, monomorphization) in `kestrel-mir`.

## Pipeline Position

```
Source → Lex → Parse → AST Build → Name Res → HIR Lower → Type Infer → MIR Lower → MIR Passes/Mono → Codegen
                                                                          ^^^
                                                                       this crate
```

## Entry Point and Phases

`lower_module(world, root)` in `lib.rs` is the only entry point. Unlike the
upstream query-driven crates, lowering is mostly plain functions threading a
mutable `LowerCtx` through four phases:

1. **Items** — structs, enums, protocols, function signatures, statics (`items/`)
2. **Witnesses** — one `WitnessDef` per conformance source (`items/witness_lower.rs`)
3. **Static inits** — per-static init thunks + master init injected into main (`items/static_lower.rs`)
4. **Validate** — ICE backstop: no `MirTy::Error` may escape into the module (`validate.rs`)

Function bodies are lowered by `lower_function_body` (`body/mod.rs`), called
from signature lowering in phase 1 and for synthesized init thunks in phase 3.

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `LowerCtx` | `context.rs` | Module-wide state: world + query handle, `MirModule` under construction, type interner, synthetic-entity allocator |
| `OssaBodyCtx` | `body/mod.rs` | Per-body state: blocks, local map, scope stack, `LiveTracker` |
| `LiveTracker` | `body/mod.rs` | Tracks live @owned values so branches can thread them through block params (see AGENTS.md) |

## Queries

The crate's only HECS query. Everything else is non-memoized functions.

| Query | Input | Output | Purpose |
|-------|-------|--------|---------|
| `IsProtocolMethod` (`context.rs`) | `{ entity, root }` | `Option<Entity>` (the protocol) | Is this entity a protocol member — directly, or as a protocol-extension default? Replaces 8 scattered parent-chain walks with one memoized lookup; used to decide witness-dispatch lowering |

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | `lower_module` entry point, phase orchestration |
| `context.rs` | `LowerCtx`, `IsProtocolMethod`, witness-key helpers |
| `ty.rs` | `HirTy`/`ResolvedTy` → interned `TyId` |
| `name.rs` | Qualified-name generation from the entity parent chain |
| `validate.rs` | Post-lowering `MirTy::Error` ICE backstop |
| `items/` | Declaration lowering: struct/enum/protocol/function-sig/static/witness |
| `body/` | Body lowering: expr, stmt, control flow, patterns (via `kestrel-pattern-matching` decision trees), closures, literals |
| `body/closure.rs` | Env struct, synthetic call function, `ApplyPartial` — both capture tiers (see below) |
| `body/closure_box.rs` | The owning tiers' heap box: `@builtin` binding resolution, boxing/unboxing, per-environment retain/release shims |
| `body/call/` | Call emission: arg binding, intrinsics, failable-init unwrapping |

## Closure Lowering Tiers

The closure's `FnKind` selects one of two lowering strategies. Captures
themselves always come from `ClosureCaptures` — the tier only decides how
each captured place is *represented*.

**View tier (`normal` / `mutating`).** Every captured place is captured by
ADDRESS: the env field is `Pointer[T]` holding a pointer into the enclosing
frame, and the body binds each capture the way a `MutBorrow` param is bound.
Reads therefore load through the pointer AT EACH USE (never in a per-call
prologue), so a write between calls is observed, and a `mutating` body's
writes store back into the original. The env owns nothing: nothing is
copied, cloned, moved or dropped, and the value stays frame-bound (E494).

**Owning tier (`consuming` / `escaping`).** The env struct is by value
(bit-copy a Copyable capture, `clone()` a Cloneable one, move a
non-Copyable one) and lives on the heap, so the value may leave the frame.
`closure_box.rs` resolves the box entity through `ResolveBuiltin`
(`Builtin::SharedBox` for `escaping`, `Builtin::UniqueBox` for
`consuming`), then finds every member it needs **by requirement name**
(`init`, `sharedMutRef`, `takeValue`, `destroy`) — never by a hard-coded
`RcBox` reference — and peels the handle type through its single-field
wrappers to the raw machine pointer that becomes word 1 of the closure
value. Swapping the lang binding retargets every boxing site.

The two owning kinds differ in how the body reaches the environment:

| Kind | Box | Body prologue | Capture binding |
|------|-----|---------------|-----------------|
| `escaping` | shared | `sharedMutRef()` borrows the payload in place | field ADDRESS inside the shared storage — state persists across calls and aliases observe it |
| `consuming` | unique | `takeValue()` moves the whole env into the frame | per-capture `@owned` locals, so the one-shot body may move a capture out while the rest drop normally |

**Prologue invariant: leave NO tracked ref behind.** `sharedMutRef()` is a
ret-borrow call, so its result is a scope-tracked single-use reference. The
prologue must convert it to a raw pointer (`RefToPtr`) and end the ref
immediately — a still-tracked ref is live at the body's first terminator, so
any `if`/`match` reading a capture reports a false E497 ("a reference cannot
stay live across a control-flow merge"). The pointer it yields is not a
reference and crosses merges freely.

`synthesize_env_shims` emits the per-environment retain/release pair packed
into words 2/3. Both take the raw handle word and are ordinary MIR calling
the box API — retain conjures a handle, `CopyValue`s it and forgets both;
release conjures a handle and destroys it. Only the *dispatch* is
compiler-emitted, post-mono in `kestrel-mir`'s `expand`. Capture-free
`escaping` values share one no-op pair and a null handle, so they never
allocate. A module with no box binding (a `stdlib: false` fixture) falls
back to the historical stack-env snapshot path.

## What Lives Elsewhere

- **Layout and mangling** are in `kestrel-mir`, not here — this crate emits
  abstract `MirTy`s; layout is computed by the MIR pass pipeline.
- **Closure captures** come from the `ClosureCaptures` query in
  `kestrel-type-infer`; this crate consumes the plan, never recomputes it.
- **Closure kinds** are decided in `kestrel-type-infer` and read back off the
  resolved type (`ty::to_mir_fn_kind`). `TypedBody.kind_coercions` records
  accepted CROSS-kind cells for replay here; today no replay is needed,
  because the three accepted cells need no representation change —
  `normal → mutating` keeps the 2-word view layout, `escaping → consuming`
  is one retained shared handle released by the call itself, and
  `escaping → normal` passes the wider value to a callee that only ever
  reads words 0–1.
- Ownership rules and lowering invariants (live-value threading,
  `value_forwarding` resolution, var_locals) are catalogued in this crate's
  `AGENTS.md`.

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-mir` | `MirModule`, `MirTy`, instructions, terminators — the output IR |
| `kestrel-hecs` | World, `QueryContext`, `QueryFn` |
| `kestrel-hir` / `kestrel-hir-lower` | `HirBody` input + type-annotation lowering queries |
| `kestrel-type-infer` | `InferBody` results, `ResolvedTy`, `ClosureCaptures` |
| `kestrel-ast` / `kestrel-ast-builder` | Decl components (`Callable`, `NodeKind`, …), arg binding |
| `kestrel-name-res` | `ExtensionTargetEntity` and friends |
| `kestrel-pattern-matching` | Match decision trees |
| `kestrel-semantics` / `kestrel-reporting` / `kestrel-span` | Copy semantics, diagnostics, spans |
