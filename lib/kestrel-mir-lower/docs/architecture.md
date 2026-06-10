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
| `body/call/` | Call emission: arg binding, intrinsics, failable-init unwrapping |

## What Lives Elsewhere

- **Layout and mangling** are in `kestrel-mir`, not here — this crate emits
  abstract `MirTy`s; layout is computed by the MIR pass pipeline.
- **Closure captures** come from the `ClosureCaptures` query in
  `kestrel-type-infer`; this crate consumes the plan, never recomputes it.
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
