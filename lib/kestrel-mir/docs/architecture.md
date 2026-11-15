# kestrel-mir Architecture

The OSSA MIR: its data structures, the verifier, the pass pipeline and
monomorphization. `kestrel-mir-lower` builds it; the codegen backends consume
the monomorphized result.

## Pipeline Position

```
Source Text → Tokens → CST → AST Build → Name Res → HIR Lower → Type Infer → MIR → Mono → Codegen
                                                                            ^^^^^^^^^^^^
                                                                         this crate (passes, mono)
```

`kestrel-mir-lower` produces a generic `MirModule`. The pre-mono passes run
here (`passes::run_pipeline_until`). `mono::monomorphize` turns it into a
concrete `MonoModule`, and `Compiler::monomorphize_mir_until` in
`kestrel-compiler` runs the post-mono passes (copy elimination, copy
lifetimes, `expand`, the audit and `verify_mono`).

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `MirModule` | `lib.rs` | Pre-mono module: functions, type infos, protocols, witnesses, vtable templates |
| `MonoModule` | `mono/types.rs` | Post-mono module: concrete functions and layouts, interned vtables |
| `FunctionDef` / `OssaBody` | `item/function.rs`, `body.rs` | A function and its body of `BasicBlock`s |
| `BasicBlock` / `BlockParam` | `block.rs` | Blocks; values cross joins as block params |
| `Instruction` / `InstKind` | `inst.rs` | The instruction set (copy, destroy, borrow, load/store, aggregates, calls, closures, …) |
| `Terminator` / `TerminatorKind` | `terminator.rs` | Branch, switch, return, unreachable |
| `ValueDef` / `Ownership` | `value.rs` | Every SSA value and its kind: `@owned`, `@guaranteed`, `@none` |
| `MirTy` / `TyArena` / `TyId` | `ty.rs`, `id.rs` | Interned MIR types |
| `TypeInfo`, `CopyBehavior`, `DropBehavior`, `Layout` | `item/mod.rs` | Per-nominal facts, set by the pass that decides each |
| `passes::Stage` | `passes/mod.rs` | The pipeline order and the dump points |

## Module Map

| Path | Responsibility |
|------|---------------|
| `item/` | Declarations: structs, enums, functions, protocols, statics, witnesses, vtable templates |
| `inst.rs`, `op.rs`, `immediate.rs`, `callee.rs`, `terminator.rs`, `block.rs`, `body.rs`, `value.rs` | The IR itself |
| `builder.rs` | Building OSSA bodies programmatically |
| `ty.rs`, `ty_query.rs` | Types; copy behavior (through `kestrel-copy-fold`), `needs_drop` |
| `substitute.rs` | Type-parameter substitution |
| `layout.rs` | `@layout` / `@align` inputs to layout |
| `verify.rs` | The OSSA verifier (linear ownership, borrow scopes, block args, address init) |
| `passes/` | Pre-mono passes (`drop_fix`, `thunk`, `drop_shim`, `clone_shim`, `layout`, `copy_check`) and the post-mono copy passes (`copy_propagation`, `copy_lifetime/`) |
| `mono/` | `collect` (reachable instantiations), `witness`, `entail` (the `kestrel-entail` layer), `vtable`, `fat` (fat-pointer legalization), `expand`, `audit`, `verify`, `mangle` |
| `display.rs` | The text form `kestrel dump mir` prints |

## Topic Documents

| Doc | Covers |
|-----|--------|
| [ir.md](ir.md) | The instruction set and value model |
| [ownership.md](ownership.md) | Ownership kinds, the linear rule, borrow scopes, partial moves |
| [lowering.md](lowering.md) | How HIR lowers to OSSA |
| [passes.md](passes.md) | Each pass and the verifiers |
| [codegen.md](codegen.md) | How backends consume MIR |
| [design-rationale.md](design-rationale.md) | Why it is built this way |

The `mir` skill is the short working guide, and `../AGENTS.md` lists the rules
that must not be broken.

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-copy-fold` | The single Copyable / Cloneable decision |
| `kestrel-structural` | `Static` / `Sized` rules |
| `kestrel-entail` | Where-clause entailment at mono |
| `kestrel-target` | Pointer width for layout |
| `kestrel-hecs`, `kestrel-span`, `kestrel-debug` | Entities, spans, `ktrace!` |
| `indexmap`, `smallvec`, `rustc-hash` | Containers |
