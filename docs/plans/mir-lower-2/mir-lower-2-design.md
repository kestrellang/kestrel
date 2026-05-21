# kestrel-mir-lower-2 Design

## Overview

A greenfield HIR → MIR lowering crate that emits `kestrel-mir-2::MirModule`.
Replaces `kestrel-mir-lower` with a design optimized for small files, easy
maintenance, and low bug surface. The crate consumes the typed ECS world
(post–type-inference) and produces a generic `MirModule` ready for the
kestrel-mir-2 pass pipeline.

## Design principles

1. **One concern per file, ≤500 lines.** No 5500-line god files.
2. **Builder API everywhere.** Use `ModuleBuilder` / `FunctionBuilder` /
   `BlockBuilder` for all MIR construction. Zero manual `Statement::new()`
   sites.
3. **TyId at the boundary.** Type lowering returns `TyId`, not `MirTy`.
   No cloning types through the pipeline.
4. **Operand, not Value.** Expression lowering returns `Operand` (mode-free).
   The call site attaches `UseMode` / `ArgMode` based on `CopyBehavior`.
5. **Index-based places.** Field projections use `FieldIdx` resolved during
   lowering. No string field names in MIR.
6. **Trait-based call dispatch.** Each call shape (direct, method, witness,
   init, struct-construct, intrinsic, etc.) is a self-contained type
   implementing a `CallEmitter` trait. No 8-site protocol fork.
7. **Fresh context for closures.** Closure bodies lower with a new `BodyCtx`,
   not a save/restore swap of 6+ fields.
8. **Table-driven intrinsics.** Intrinsic name → Op mapping is a static table,
   not 400 lines of match arms.

## Architecture

### Crate structure

```
kestrel-mir-lower-2/src/
├── lib.rs                  — lower_module() entry point
├── context.rs              — LowerCtx (module-level shared state)
│
├── items/
│   ├── mod.rs              — entity tree walk, NodeKind dispatch
│   ├── struct_lower.rs     — StructDef construction
│   ├── enum_lower.rs       — EnumDef + payload structs
│   ├── protocol_lower.rs   — ProtocolDef construction
│   ├── witness_lower.rs    — WitnessDef generation
│   ├── function_sig.rs     — FunctionDef signatures, kind, where clauses
│   └── static_lower.rs     — StaticDef + init thunk synthesis
│
├── body/
│   ├── mod.rs              — BodyCtx, lower_body(), block management
│   ├── expr.rs             — lower_expr dispatch, field/tuple/local access
│   ├── stmt.rs             — lower_stmt, let bindings, assignment + setter
│   ├── control.rs          — if, loop, break, continue, return, block
│   ├── literal.rs          — literals, array/dict literal via init
│   ├── closure.rs          — closure lowering (fresh BodyCtx)
│   ├── pattern.rs          — match, decision tree, bindings
│   └── call/
│       ├── mod.rs          — lower_call entry, CallEmitter trait
│       ├── classify.rs     — CallShape classification from HIR
│       ├── direct.rs       — DirectCall emitter
│       ├── method.rs       — MethodCall emitter
│       ├── witness.rs      — WitnessCall emitter
│       ├── init.rs         — InitCall emitter (regular + effectful)
│       ├── construct.rs    — StructConstruct / EnumConstruct emitters
│       ├── indirect.rs     — IndirectCall (thin/thick fn pointers)
│       ├── intrinsic.rs    — IntrinsicCall emitter (table-driven)
│       └── args.rs         — shared arg lowering, mode assignment, defaults
│
├── ty.rs                   — HirTy / ResolvedTy → TyId
├── name.rs                 — qualified name generation
└── validate.rs             — post-lowering MirTy::Error check
```

**File count:** 24 files. Largest file: `witness_lower.rs` (~400 lines).
Average file: ~150 lines.

### Module-level context: `LowerCtx`

```rust
pub struct LowerCtx<'w> {
    pub world: &'w World,
    pub query: QueryContext<'w>,
    pub root: Entity,
    pub module: MirModule,
    pub closure_counter: u32,
}
```

Owns the `MirModule` being built. All type interning goes through
`self.module.ty_arena`. Synthetic entities use the builder's
`fresh_entity()` for tests, or a dedicated counter with a reserved range
for production (documented, with a debug_assert against collision).

Methods on `LowerCtx` that the current crate scatters across `BodyLowerCtx`:

- `register_name(entity) -> String`
- `intern_type(ty: MirTy) -> TyId`
- `is_protocol_method(entity) -> Option<Entity>` — the single source of
  truth for "does this entity need witness dispatch?"
- `witness_method_key(entity) -> WitnessMethodKey`
- `resolve_field_idx(struct_entity, field_name) -> FieldIdx`
- `resolve_variant_idx(enum_entity, case_name) -> VariantIdx`

Moving protocol detection and field index resolution to `LowerCtx` means
every call site in the body lowerer uses the same logic. No duplication.

### Body-level context: `BodyCtx`

```rust
pub(crate) struct BodyCtx<'a, 'w> {
    pub ctx: &'a mut LowerCtx<'w>,
    pub hir: &'a HirBody,
    pub typed: Option<&'a TypedBody>,
    pub func_entity: Entity,
    pub func_idx: usize,
    body: MirBody,
    current_block: Option<BlockId>,
    local_map: HashMap<HirLocalId, LocalId>,
    loop_stack: Vec<LoopInfo>,
    temp_counter: u32,
    current_span: Option<Span>,
}
```

No `is_effectful_init`, `init_field_flags`, or `in_protocol_extension` —
these move to the init call emitter and function signature lowerer
respectively, scoped to where they're needed.

`BodyCtx` provides the block-management primitives:

- `new_block() -> BlockId`
- `switch_to(&mut self, block: BlockId)`
- `is_terminated() -> bool`
- `block(&mut self) -> &mut BlockBuilder` — access the current block's builder
- `fresh_temp(ty: TyId) -> LocalId`
- `map_local(hir_id) -> LocalId`
- `span(&self) -> Option<Span>` — current HIR span

Expression lowering methods live in `body/expr.rs` as `impl BodyCtx` blocks.
Same struct, split across files.

### Expression lowering returns `Operand`

```rust
// body/expr.rs
impl BodyCtx<'_, '_> {
    pub fn lower_expr(&mut self, expr_id: HirExprId) -> Operand {
        let operand = self.lower_expr_inner(expr_id);
        self.apply_promotion(expr_id, operand)
    }
}
```

`Operand` is mode-free (`Place(Place)` or `Const(Immediate)`). The caller
decides copy vs move:

```rust
// Single function, used everywhere:
fn use_mode_for(&self, ty: TyId) -> UseMode {
    match self.ctx.module.ty_arena.get(ty) {
        // primitives, bitwise-copyable → Copy
        // CopyBehavior::None → Move
    }
}

fn arg_mode_for(&self, ty: TyId, convention: ParamConvention) -> ArgMode {
    match convention {
        ParamConvention::Borrow => ArgMode::Ref,
        ParamConvention::MutBorrow => ArgMode::RefMut,
        ParamConvention::Consuming => {
            if /* copy behavior != None */ { ArgMode::Copy } else { ArgMode::Move }
        }
    }
}
```

Two functions. One source of truth. Every call site uses them.

### Trait-based call dispatch

```rust
// body/call/mod.rs

/// A classified call ready to emit.
pub(crate) trait CallEmitter {
    /// Emit the call into the current block, returning the result operand.
    fn emit(self, bctx: &mut BodyCtx) -> Operand;
}

/// Classification result — one variant per call shape.
pub(crate) enum ClassifiedCall {
    Panic,
    Intrinsic(IntrinsicCall),
    EnumConstruct(EnumConstructCall),
    StructConstruct(StructConstructCall),
    Init(InitCall),
    Direct(DirectCall),
    Method(MethodCall),
    Witness(WitnessCall),
    Indirect(IndirectCall),
}
```

The entry point tries classifiers in priority order:

```rust
pub fn lower_call(bctx: &mut BodyCtx, expr_id: HirExprId, ...) -> Operand {
    let classified = classify_call(bctx, expr_id, callee_expr, args);
    classified.emit(bctx)
}
```

Each classifier is a struct in its own file with a `classify()` constructor
that returns `Option<Self>`, and an `emit()` method. For example:

```rust
// body/call/intrinsic.rs
pub struct IntrinsicCall {
    op: Op,
    arity: u8,
    args: SmallVec<[HirExprId; 3]>,
    result_ty: TyId,
}

impl IntrinsicCall {
    pub fn classify(bctx: &BodyCtx, entity: Entity, args: &[HirCallArg]) -> Option<Self> {
        let name = bctx.ctx.world.get::<Intrinsic>(entity)?;
        let entry = INTRINSIC_TABLE.iter().find(|e| e.name == name)?;
        Some(Self { op: entry.op, arity: entry.arity, ... })
    }
}

impl CallEmitter for IntrinsicCall {
    fn emit(self, bctx: &mut BodyCtx) -> Operand { ... }
}
```

**Why trait-based, not enum-based?** Each call shape carries different data
(an `InitCall` carries the parent struct entity and effectfulness flag; a
`WitnessCall` carries the protocol entity and method key). A trait lets each
shape own exactly the data it needs without a mega-enum. The `ClassifiedCall`
enum wraps them for the dispatch site, but the `emit()` implementations are
decoupled.

### Classification resolves type args once

The type-arg resolution cascade (currently 5 fallback layers) consolidates
into `classify.rs`. By the time a `ClassifiedCall` is constructed, its
`type_args: Vec<TyId>` field is fully resolved. The emit phase never
touches type-arg resolution.

```rust
// body/call/classify.rs
fn resolve_call_type_args(bctx: &BodyCtx, expr_id: HirExprId, callee_expr: HirExprId) -> Vec<TyId> {
    // 1. inference on call expression
    // 2. inference on callee expression
    // 3. explicit AST type args
    // 4. parent struct type args (for static methods)
    // All in one place. Called once per call.
}
```

### Init calls

The init emitter (`body/call/init.rs`) handles:

- **Regular init:** allocate temp, prepend `&mut self`, emit call, return temp
- **Effectful init:** allocate temp, call → switch on result discriminant →
  wrap in `Optional`/`Result` variant

The effectful-init field flags (`init_field_flags`, `SetDeinitFlag`) live
entirely inside the init emitter. They don't leak into `BodyCtx` or other
call shapes.

### Intrinsic table

```rust
// body/call/intrinsic.rs
struct IntrinsicEntry {
    name: &'static str,
    op: Op,
    arity: u8,
}

static TABLE: &[IntrinsicEntry] = &[
    IntrinsicEntry { name: "i64_add", op: Op::Add(IntBits::I64, Signedness::Signed), arity: 2 },
    // ~100 entries, one per line
];
```

~120 lines total instead of ~400. Adding an intrinsic = one table row.
A `#[test]` cross-references the table against `lang` module entities.

### Closure lowering

```rust
// body/closure.rs
pub fn lower_closure(parent: &mut BodyCtx, expr_id: HirExprId, ...) -> Operand {
    // Collect captures from parent scope
    let captures = find_captures(parent, ...);
    
    // Create env struct and closure function def (via LowerCtx)
    let (env_struct, closure_func) = setup_closure(parent.ctx, ...);
    
    // Lower closure body in a fresh BodyCtx
    let mut closure_ctx = BodyCtx::new_for_closure(parent.ctx, parent.hir, parent.typed);
    // ... set up params, capture loads ...
    closure_ctx.lower_body();
    let closure_body = closure_ctx.finish();
    
    // Attach body to function, emit ApplyPartial in parent
    ...
}
```

No save/restore. `parent` is borrowed immutably for capture collection,
then `closure_ctx` borrows `parent.ctx` mutably for the actual lowering.
The parent's `BodyCtx` fields are never touched.

### Field index resolution

```rust
// context.rs
impl LowerCtx<'_> {
    pub fn resolve_field_idx(&self, struct_entity: Entity, field_name: &str) -> FieldIdx {
        // Look up the struct in module.structs, find field by name, return index.
        // Panics if not found — field names are validated upstream by name-res.
        ...
    }
}
```

Called in `expr.rs` when lowering `HirExpr::Field` for stored fields.
The resulting `Place` uses `FieldIdx`, not a string. Downstream passes
never compare strings.

### Type lowering returns TyId

```rust
// ty.rs
pub fn lower_type(ctx: &mut LowerCtx, ty: &HirTy) -> TyId {
    match ty {
        HirTy::SelfType(..) => ctx.intern_type(MirTy::SelfType),
        HirTy::Struct { entity, args, .. } => {
            let args: Vec<TyId> = args.iter().map(|a| lower_type(ctx, a)).collect();
            try_lang_primitive(ctx, *entity, &args)
                .unwrap_or_else(|| ctx.module.ty_arena.named(*entity, args))
        },
        HirTy::Tuple(elems, _) => {
            let elems: Vec<TyId> = elems.iter().map(|e| lower_type(ctx, e)).collect();
            ctx.module.ty_arena.tuple(elems)
        },
        // ... etc
    }
}

pub fn lower_resolved_ty(ctx: &mut LowerCtx, ty: &ResolvedTy) -> TyId {
    // Same pattern, from inference results
}
```

No `MirTy::clone()` anywhere. Arena deduplicates automatically.

### Shared substitution

Type substitution uses `kestrel_mir_2::substitute()` directly. No
duplicated `substitute_type_params` / `replace_self_type` /
`substitute_mir_type` functions. One implementation, three current
call sites consolidated.

## Pipeline impact

| Stage | Change |
|-------|--------|
| kestrel-mir-2 | No changes — crate already exists |
| kestrel-mir-lower-2 (new) | New crate, replaces kestrel-mir-lower |
| kestrel-compiler | Wire `lower_module` to call new crate |
| kestrel-codegen-cranelift | Later — consumes MonoModule from kestrel-mir-2 |
| kestrel-mir-lower (old) | Deleted after switchover |

## Error handling

- `HirExpr::Error` / `HirPat::Error` → emit `Operand::Const(Immediate::error())`
- `ResolvedTy::Error` → intern `MirTy::Error`, propagate as TyId
- Missing type inference results → `TyId` for `MirTy::Error`
- Post-lowering `validate.rs` walks the module and emits ICE diagnostics
  for any surviving `MirTy::Error`, same as today

## What stays the same

- **Orchestration order:** items → witnesses → static inits → validate
- **Witness generation logic:** per-instantiation witnesses, label+param-type
  matching, setter dispatch, blanket conformances — preserved from the
  existing crate with the same correctness
- **Integration tests:** stdlib smoke tests, tightened with specific count
  assertions
- **Name generation:** `qualified_name()` walks parent chain, same algorithm

## What changes vs kestrel-mir-lower

| Current problem | New design |
|----------------|------------|
| 5500-line body_lower.rs | 10 files under body/, largest ~300 lines |
| 8-site protocol dispatch fork | Single `is_protocol_method()` on LowerCtx, used by classify.rs |
| 5-layer type-arg cascade | One `resolve_call_type_args()` in classify.rs |
| `Value` enum with 4 place variants | `Operand` (mode-free), mode at use site |
| HirExpr clone per expression | Borrow HirExpr from arena via index |
| 60 manual Statement::new() | Builder API: `block.assign_op2(...)`, etc. |
| 3 duplicate substitute functions | Single `kestrel_mir_2::substitute()` |
| Closure save/restore 6 fields | Fresh `BodyCtx::new_for_closure()` |
| 400-line intrinsic match | ~120-line static table |
| Field access by string name | `FieldIdx` resolved during lowering |
| `emit_call` vs `emit_call_maybe_init` | Init is a call shape, not a wrapper |
