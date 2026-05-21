# kestrel-mir-lower-2 Design

## Overview

A greenfield HIR → MIR lowering crate that emits `kestrel-mir-2::MirModule`.
Replaces `kestrel-mir-lower` with a design optimized for small files, easy
maintenance, and low bug surface. The crate consumes the typed ECS world
(post–type-inference) and produces a generic `MirModule` ready for the
kestrel-mir-2 pass pipeline.

## Design principles

1. **One concern per file, ≤500 lines.** No 5500-line god files.
2. **Queries over component re-derivation.** Use hECS queries for cross-entity
   facts. Don't re-derive from raw `NodeKind`/`parent_of` walks what a
   name-res or inference query already computes.
3. **TyId at the boundary.** Type lowering returns `TyId`, not `MirTy`.
   No cloning types through the pipeline.
4. **Operand, not Value.** Expression lowering returns `Operand` (mode-free).
   The call site attaches `UseMode` / `ArgMode` based on `CopyBehavior`.
5. **Index-based places.** Field projections use `FieldIdx` resolved during
   lowering. No string field names in MIR.
6. **Classify-and-emit call dispatch.** Each call shape (direct, method,
   witness, init, construct, intrinsic) is a `try_*` function in its own
   file that classifies and emits in one step. No intermediate enum or trait.
7. **Fresh context for closures.** Closure bodies lower with a new `BodyCtx`.
   Capture collection is a free function; no save/restore of parent state.
8. **Table-driven intrinsics.** Intrinsic name → Op mapping is a static table,
   not 400 lines of match arms.

## hECS consumption model

MIR lowering is a **one-shot pass**, not a query. It reads from the ECS world
and produces a `MirModule`. It does not participate in the dependency graph for
incremental recompilation. The code is structured so that a future per-function
query could be introduced without architectural changes.

### Three kinds of ECS reads

| Kind | API | Tracks deps? | Use for |
|------|-----|-------------|---------|
| **Component** | `world.get::<C>(entity)` | No | Syntax-level facts: `Name`, `NodeKind`, `Callable`, `TypeParams`, `Body`, `Attributes`, `Static`, `Settable` |
| **Query** | `query.query(Q { ... })` | Yes (memoized) | Cross-entity resolution: type inference, name resolution, conformance, member lookup |
| **Traversal** | `world.parent_of(e)`, `world.children_of(e)` | No | Structural navigation for item discovery |

**Convention:** If a name-res or inference query already computes a fact, use
the query. Don't re-derive from raw components.

### Queries consumed per module

| Module | Queries |
|--------|---------|
| `function_sig.rs` | `ExtensionTargetEntity`, `ResolveTypePath` |
| `body/*.rs` | `LowerBody`, `InferBody`, `ExtensionTargetEntity`, `ExtensionsFor`, `ProtocolMembersByName`, `ResolveBuiltin` |
| `body/call/*` | `IsProtocolMethod` (new, thin), `InferBody` (for opaque returns) |
| `witness_lower.rs` | `ConformingProtocolInstantiations`, `ExtensionsFor`, `ProtocolMembers`, `TypeMembersByName`, `ProtocolAssociatedTypes` |
| `struct_lower.rs` | `NominalCopySemantics` |
| `protocol_lower.rs` | `ConformingProtocols` |
| `ty.rs` | `LowerTypeAnnotation`, `LowerCallableReturnType`, `LowerCallableTypes`, `InferBody` |
| `validate.rs` | `QueryContext::accumulate()` for ICE diagnostics |

### `IsProtocolMethod` query (new)

The existing crate has 8 call sites that walk `parent_of` → check
`NodeKind::Protocol` → check `NodeKind::Extension` → query
`ExtensionTargetEntity`. This becomes a single cached query:

```rust
/// Returns Some(protocol_entity) if `entity` lives on a protocol
/// (direct member or protocol extension default).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct IsProtocolMethod {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for IsProtocolMethod {
    type Output = Option<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Entity> {
        let parent = ctx.get::<EnclosingContainer>(self.entity)
            .map(|ec| ec.0)
            .or_else(|| ctx.parent_of(self.entity))?;
        match ctx.get::<NodeKind>(parent)? {
            NodeKind::Protocol => Some(parent),
            NodeKind::Extension => {
                let target = ctx.query(ExtensionTargetEntity {
                    extension: parent,
                    root: self.root,
                })?;
                match ctx.get::<NodeKind>(target)? {
                    NodeKind::Protocol => Some(target),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
```

Computed once per entity, cached, dependency-tracked. Every call site in the
lowerer calls `ctx.query.query(IsProtocolMethod { entity, root })` instead of
re-walking the parent chain. This is the single source of truth for "does this
entity need witness dispatch?"

This query can live in `kestrel-name-res` (it only reads `NodeKind`,
`EnclosingContainer`, and `ExtensionTargetEntity`), or in the lowerer crate
itself.

## Architecture

### Crate structure

```
kestrel-mir-lower-2/src/
├── lib.rs                  — lower_module() entry point
├── context.rs              — LowerCtx + shared ECS helpers
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
│   ├── mod.rs              — BodyCtx, lower_body(), block management, emit helpers
│   ├── expr.rs             — lower_expr dispatch, field/tuple/local access
│   ├── stmt.rs             — lower_stmt, let bindings, assignment + setter dispatch
│   ├── control.rs          — if, loop, break, continue, return, block
│   ├── literal.rs          — literals, array/dict literal via init
│   ├── closure.rs          — closure lowering (fresh BodyCtx)
│   ├── pattern.rs          — match, decision tree, bindings
│   └── call/
│       ├── mod.rs          — lower_call try_* chain + resolved-entity dispatch
│       ├── init.rs         — InitCall (regular + effectful)
│       ├── construct.rs    — struct + enum construction
│       ├── intrinsic.rs    — table-driven intrinsic → Op
│       └── args.rs         — arg lowering, type-arg resolution, modes, defaults
│
├── ty.rs                   — HirTy / ResolvedTy → TyId
├── name.rs                 — qualified name generation
└── validate.rs             — post-lowering MirTy::Error check
```

**19 files. ~3,800 lines.** Largest: `witness_lower.rs` (~400 lines).
Average: ~180 lines.

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
`self.module.ty_arena`.

Methods on `LowerCtx`:

```rust
impl LowerCtx<'_> {
    // --- Name registration ---
    pub fn register_name(&mut self, entity: Entity) -> String;

    // --- Type interning ---
    pub fn intern(&mut self, ty: MirTy) -> TyId;

    // --- MirModule lookups (read the module being built) ---
    pub fn resolve_field_idx(&self, struct_entity: Entity, field_name: &str) -> FieldIdx;
    pub fn resolve_variant_idx(&self, enum_entity: Entity, case_name: &str) -> VariantIdx;

    // --- hECS query wrappers (delegate to self.query) ---
    pub fn is_protocol_method(&self, entity: Entity) -> Option<Entity> {
        self.query.query(IsProtocolMethod { entity, root: self.root })
    }
    pub fn witness_method_key(&self, entity: Entity) -> WitnessMethodKey;
}
```

Protocol detection uses the `IsProtocolMethod` query — no raw parent-chain
walk. `resolve_field_idx` / `resolve_variant_idx` read from the
already-lowered `MirModule` (items must be lowered before bodies).

### Body-level context: `BodyCtx`

```rust
pub(crate) struct BodyCtx<'a, 'w> {
    pub ctx: &'a mut LowerCtx<'w>,
    pub hir: &'a HirBody,
    pub typed: Option<&'a TypedBody>,
    pub func_entity: Entity,
    pub func_idx: usize,
    pub in_protocol_extension: bool,
    body: MirBody,
    current_block: Option<BlockId>,
    local_map: HashMap<HirLocalId, LocalId>,
    loop_stack: Vec<LoopInfo>,
    temp_counter: u32,
    current_span: Option<Span>,
}
```

`in_protocol_extension` stays here — it's used during method call lowering
to replace protocol-named receivers with `SelfType`. Computed once in
`lower_function_body()`.

No `is_effectful_init` or `init_field_flags` — those live entirely inside the
init call emitter (`body/call/init.rs`), scoped to where they're needed.

### BodyCtx emit helpers

`BodyCtx` has its own emit methods that build statements and push them into
the detached `MirBody`. These mirror the kestrel-mir-2 `BlockBuilder` API
but add span tracking and current-block management:

```rust
impl BodyCtx<'_, '_> {
    // --- Block management ---
    pub fn new_block(&mut self) -> BlockId;
    pub fn switch_to(&mut self, block: BlockId);
    pub fn is_terminated(&self) -> bool;

    // --- Locals ---
    pub fn fresh_temp(&mut self, ty: TyId) -> LocalId;
    pub fn map_local(&self, hir_id: HirLocalId) -> LocalId;

    // --- Emit statements (auto-stamps current span) ---
    pub fn emit_assign(&mut self, dest: Place, rvalue: Rvalue);
    pub fn emit_use_copy(&mut self, dest: Place, src: Place);
    pub fn emit_use_move(&mut self, dest: Place, src: Place);
    pub fn emit_assign_const(&mut self, dest: Place, imm: Immediate);
    pub fn emit_assign_op1(&mut self, dest: Place, op: Op, arg: Operand);
    pub fn emit_assign_op2(&mut self, dest: Place, op: Op, lhs: Operand, rhs: Operand);
    pub fn emit_construct(&mut self, dest: Place, ty: TyId, fields: Vec<(FieldIdx, Operand, UseMode)>);
    pub fn emit_call(&mut self, dest: Option<Place>, callee: Callee, args: Vec<(Operand, ArgMode)>);
    pub fn emit_drop(&mut self, place: Place);

    // --- Emit terminators (auto-stamps current span) ---
    pub fn emit_ret(&mut self, operand: Operand);
    pub fn emit_ret_unit(&mut self);
    pub fn emit_jump(&mut self, target: BlockId);
    pub fn emit_branch(&mut self, cond: Operand, then_block: BlockId, else_block: BlockId);
    pub fn emit_switch(&mut self, disc: Place, cases: Vec<(SwitchCase, BlockId)>);
    pub fn emit_panic(&mut self, msg: &str);

    // --- Mode decisions (single source of truth) ---
    pub fn use_mode_for(&self, ty: TyId) -> UseMode;
    pub fn arg_mode_for(&self, ty: TyId, convention: ParamConvention) -> ArgMode;

    // --- Finish ---
    pub fn finish(self) -> MirBody;
}
```

These are NOT wrappers around `BlockBuilder`. They operate directly on
`self.body.block_mut(self.current_block)`. The kestrel-mir-2 builder stays
for tests and downstream consumers; the lowerer doesn't use it because
body construction is detached from the module (closures and init thunks
create new functions while a parent body is in-flight).

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
decides copy vs move via `use_mode_for()` / `arg_mode_for()`.

### Call dispatch: `try_*` chain

```rust
// body/call/mod.rs

/// Lower a call expression. Tries specialized handlers in priority order,
/// falls through to resolved-entity dispatch.
pub fn lower_call(
    bctx: &mut BodyCtx,
    expr_id: HirExprId,
    callee_expr: HirExprId,
    args: &[HirCallArg],
) -> Operand {
    // Priority 1: panic intrinsic → Panic terminator
    if let Some(op) = try_panic(bctx, callee_expr) { return op; }

    // Priority 2: lang intrinsic → MIR Op
    if let Some(op) = try_intrinsic(bctx, expr_id, callee_expr, args) { return op; }

    // Priority 3: enum case construction → Rvalue::EnumVariant
    if let Some(op) = try_enum_construct(bctx, expr_id, callee_expr, args) { return op; }

    // Priority 4: struct memberwise construction → Rvalue::Construct
    if let Some(op) = try_struct_construct(bctx, expr_id, callee_expr, args) { return op; }

    // Everything else: resolve entity, type args, build callee, emit call
    emit_resolved_call(bctx, expr_id, callee_expr, args)
}
```

`emit_resolved_call` handles the remaining shapes in one function:

```rust
fn emit_resolved_call(bctx: &mut BodyCtx, ...) -> Operand {
    let (entity, type_args) = resolve_callee_and_type_args(bctx, ...);
    let result_ty = bctx.resolve_expr_type(expr_id);

    // Init call? → allocate self, special emit
    if let Some(parent) = is_init_function(bctx.ctx, entity) {
        return emit_init_call(bctx, entity, parent, type_args, args, result_ty);
    }

    // Build callee — one branch, not eight
    let callee = if let Some(protocol) = bctx.ctx.is_protocol_method(entity) {
        let key = bctx.ctx.witness_method_key(entity);
        Callee::Witness { protocol, method: key, self_type, method_type_args: type_args }
    } else if has_receiver {
        Callee::direct_with_args(entity, type_args, Some(receiver_ty))
    } else {
        Callee::direct_with_args(entity, type_args, None)
    };

    let call_args = lower_and_mode_args(bctx, args, entity);
    bctx.emit_call(Some(Place::local(dest)), callee, call_args);
    Operand::Place(Place::local(dest))
}
```

The protocol-vs-direct decision happens exactly once. `is_protocol_method`
is a cached hECS query. Type-arg resolution happens in
`resolve_callee_and_type_args` — one function, called once per call.

Each `try_*` function lives in its own file. `emit_resolved_call` and
`emit_init_call` handle the common cases in `call/mod.rs`. Indirect calls
(thin/thick function pointers) are a branch in `emit_resolved_call` for
unresolved callees.

### Type-arg resolution: one function, one place

```rust
// body/call/args.rs
pub fn resolve_callee_and_type_args(
    bctx: &BodyCtx,
    expr_id: HirExprId,
    callee_expr: HirExprId,
    args: &[HirCallArg],
) -> (Entity, Vec<TyId>) {
    // 1. Check typed.resolutions[expr_id] for inference-resolved entity
    // 2. Fall back to typed.resolutions[callee_expr]
    // 3. Fall back to HirExpr::Def entity
    let entity = resolve_entity(bctx, expr_id, callee_expr, args);

    // Type args — single cascade, called once:
    // 1. typed.type_args[expr_id]
    // 2. typed.type_args[callee_expr] (if not init and not chained call)
    // 3. explicit AST type args on callee
    // 4. parent struct type args (for static methods on generic types)
    let type_args = resolve_type_args(bctx, expr_id, callee_expr, entity);

    (entity, type_args)
}
```

### Setter dispatch

Assignment lowering in `stmt.rs` classifies the target before emitting:

```rust
// body/stmt.rs
fn lower_assign(&mut self, target: HirExprId, value: HirExprId) {
    // Try setter dispatch first (computed properties, protocol properties,
    // subscript setters). Returns Some(unit_operand) if handled.
    if let Some(op) = self.try_setter_assign(target, value) {
        return;
    }
    // Fall through to stored-place assignment
    let rhs = self.lower_expr(value);
    let lhs_place = self.lower_expr_to_place(target);
    let mode = self.use_mode_for(rhs_ty);
    self.emit_assign(lhs_place, Rvalue::Use(rhs, mode));
}
```

Setter calls emit through `BodyCtx::emit_call()` — the same primitive the
call/ module uses. The witness-vs-direct decision uses
`ctx.is_protocol_method()` — the same query. No separate code path.

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

~130 lines total instead of ~400. Adding an intrinsic = one table row.
A `#[test]` cross-references the table against `lang` module entities.

### Closure lowering

```rust
// body/closure.rs

/// Collect captured locals. Free function — takes read-only references,
/// no BodyCtx needed. Runs before the closure BodyCtx is created.
fn find_captures(
    hir: &HirBody,
    local_map: &HashMap<HirLocalId, LocalId>,
    closure_params: &HashSet<HirLocalId>,
    body: &HirBlock,
) -> Vec<HirLocalId> { ... }

pub fn lower_closure(parent: &mut BodyCtx, expr_id: HirExprId, ...) -> Operand {
    let closure_params: HashSet<_> = params.iter().map(|p| p.local).collect();

    // Step 1: collect captures (reads parent fields, no mutation)
    let captures = find_captures(parent.hir, &parent.local_map, &closure_params, body);

    // Step 2: create env struct + closure function def
    // (uses parent.ctx, which borrows through parent — fine because
    // find_captures is done and returned owned data)
    let env_entity = create_env_struct(parent.ctx, &captures, ...);
    let closure_func_idx = create_closure_func(parent.ctx, ...);

    // Step 3: lower closure body in a fresh BodyCtx
    // Split borrow: &mut parent.ctx (for module access) +
    // &parent.hir + parent.typed (shared refs, different fields)
    let mut closure_ctx = BodyCtx::new_for_closure(
        parent.ctx,     // &mut LowerCtx — reborrow from parent
        parent.hir,     // &HirBody — shared ref, different field
        parent.typed,   // Option<&TypedBody> — shared ref
    );
    // ... set up params, load captures from env ...
    closure_ctx.lower_body();
    let closure_body = closure_ctx.finish();

    // Step 4: attach body, emit ApplyPartial in parent scope
    parent.ctx.module.functions[closure_func_idx].body = Some(closure_body);
    // ... emit capture + ApplyPartial ...
}
```

No save/restore. `find_captures` is a free function that reads `parent`
fields and returns owned data. Rust's split borrows let
`BodyCtx::new_for_closure` take `&mut parent.ctx` + `&parent.hir` +
`parent.typed` simultaneously because they're different struct fields.

### Type lowering returns TyId

```rust
// ty.rs
pub fn lower_type(ctx: &mut LowerCtx, ty: &HirTy) -> TyId {
    match ty {
        HirTy::SelfType(..) => ctx.intern(MirTy::SelfType),
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
`substitute_mir_type` functions. One implementation in kestrel-mir-2,
consumed by all three current call sites.

## Pipeline impact

| Stage | Change |
|-------|--------|
| kestrel-name-res (or lowerer) | Add `IsProtocolMethod` query |
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
  for any surviving `MirTy::Error` via `QueryContext::accumulate()`

## What stays the same

- **Orchestration order:** items → witnesses → static inits → validate.
  Items must complete before bodies (bodies need struct field indices
  from the lowered module). This constraint is documented in `lib.rs`
  but not type-enforced — one entry point, not worth the ceremony.
- **Witness generation logic:** per-instantiation witnesses, label+param-type
  matching, setter dispatch, blanket conformances — preserved from the
  existing crate with the same correctness
- **Integration tests:** stdlib smoke tests, tightened with specific count
  assertions
- **Name generation:** `qualified_name()` walks parent chain, same algorithm

## What changes vs kestrel-mir-lower

| Current problem | New design |
|----------------|------------|
| 5500-line body_lower.rs | 12 files under body/, largest ~300 lines |
| 8-site protocol dispatch fork | `IsProtocolMethod` hECS query, called once per entity |
| 15 raw `NodeKind` reads for entity classification | Queries where possible; remaining reads documented |
| 5-layer type-arg cascade | One `resolve_callee_and_type_args()` in args.rs |
| `Value` enum with 4 place variants | `Operand` (mode-free), mode at use site |
| HirExpr clone per expression | Borrow HirExpr from arena by index |
| 60 manual `Statement::new()` | `BodyCtx::emit_*()` helpers with auto span |
| 3 duplicate substitute functions | Single `kestrel_mir_2::substitute()` |
| Closure save/restore 6 fields | Fresh `BodyCtx::new_for_closure()` |
| 400-line intrinsic match | ~130-line static table |
| Field access by string name | `FieldIdx` resolved during lowering |
| `emit_call` vs `emit_call_maybe_init` | Init is a call shape in `call/init.rs` |
| ECS queries undocumented | Query dependencies listed per module |

## LOC estimate

| File | Lines |
|------|------:|
| `lib.rs` | 50 |
| `context.rs` | 130 |
| `items/mod.rs` | 80 |
| `items/struct_lower.rs` | 70 |
| `items/enum_lower.rs` | 80 |
| `items/protocol_lower.rs` | 80 |
| `items/witness_lower.rs` | 400 |
| `items/function_sig.rs` | 250 |
| `items/static_lower.rs` | 150 |
| `body/mod.rs` | 150 |
| `body/expr.rs` | 350 |
| `body/stmt.rs` | 250 |
| `body/control.rs` | 150 |
| `body/literal.rs` | 250 |
| `body/closure.rs` | 200 |
| `body/pattern.rs` | 250 |
| `body/call/mod.rs` | 150 |
| `body/call/init.rs` | 200 |
| `body/call/construct.rs` | 100 |
| `body/call/intrinsic.rs` | 130 |
| `body/call/args.rs` | 120 |
| `ty.rs` | 200 |
| `name.rs` | 50 |
| `validate.rs` | 120 |
| tests | 400 |
| **Total** | **~3,810** |

Current crate: ~8,500 lines. **55% reduction.**
