# kestrel-mir-lower-2 Implementation Plan

## Test strategy

- **Reuse existing integration tests** from `kestrel-mir-lower/src/lib.rs` (stdlib smoke,
  struct/enum/function body lowering, calls, witnesses, string literals, passes).
  Tighten assertions: specific counts instead of "non-empty."
- **New unit tests** per module: one test file per `items/` and `body/` module verifying
  the lowered MIR shape for a minimal input.
- **Intrinsic cross-reference test**: verify every entry in the intrinsic table corresponds
  to a `lang` module entity with the `Intrinsic` marker.
- All tests run via `/triage` — never `cargo test` directly.

## Phases

### Phase 0 — Scaffold crate + `IsProtocolMethod` query

Create the crate skeleton and the one upstream change needed before any lowering code.

- [ ] Create `lib/kestrel-mir-lower-2/Cargo.toml` with dependencies:
      `kestrel-hecs`, `kestrel-span`, `kestrel-mir-2`, `kestrel-hir`,
      `kestrel-hir-lower`, `kestrel-type-infer`, `kestrel-ast-builder`,
      `kestrel-name-res`, `kestrel-semantics`, `kestrel-pattern-matching`,
      `kestrel-reporting`, `kestrel-compiler` (dev), `kestrel-compiler-driver` (dev),
      `indexmap`, `smallvec`
- [ ] Create `src/lib.rs` with a stub `pub fn lower_module(world: &World, root: Entity) -> MirModule`
      that returns an empty module
- [ ] Create `src/context.rs` with `LowerCtx` struct and basic methods
      (`register_name`, `intern`, `resolve_field_idx`, `resolve_variant_idx`)
- [ ] Implement `IsProtocolMethod` query (in `kestrel-name-res` or in the lowerer crate).
      Add `is_protocol_method()` and `witness_method_key()` on `LowerCtx` delegating to it.
- [ ] Create `src/name.rs` — port `qualified_name()` as-is from existing crate
- [ ] Verify: `cargo check -p kestrel-mir-lower-2`

### Phase 1 — Type lowering (`ty.rs`)

Types are needed by every subsequent phase.

- [ ] Create `src/ty.rs` with `lower_type(ctx, &HirTy) -> TyId`
- [ ] Implement `lower_resolved_ty(ctx, &ResolvedTy) -> TyId`
- [ ] Implement `try_lang_primitive()` — recognize `lang.i64`, `lang.str`, etc.
- [ ] Implement `lower_named_type_from_entity()` — shared by both HirTy and ResolvedTy paths
- [ ] Handle `AssocProjection`, `Opaque` (resolve via `InferBody`), `SelfType`
- [ ] Port opaque-type cycle guard (thread-local `HashSet<Entity>`)
- [ ] Test: lower stdlib types, verify no `MirTy::Error` in struct fields

### Phase 2 — Item lowering (`items/`)

Structs, enums, protocols must exist before bodies can reference them.

- [ ] `items/mod.rs` — entity tree walk by `NodeKind`, dispatch to sub-lowerers
- [ ] `items/struct_lower.rs` — `StructDef` with fields, `CopyBehavior` via
      `NominalCopySemantics` query, `DropBehavior` with user deinit detection
- [ ] `items/enum_lower.rs` — `EnumDef` with cases, payload structs per case
- [ ] `items/protocol_lower.rs` — `ProtocolDef` with methods, associated types,
      parent protocols via `ConformingProtocols` query
- [ ] `items/function_sig.rs` — `FunctionDef` signatures:
  - [ ] `determine_function_kind()` (Method/Static/Init/Deinit/Closure/Free)
  - [ ] `collect_inherited_type_params()` (parent struct/enum/extension params)
  - [ ] `populate_where_clause()` (walk entity + parents for constraints)
  - [ ] Parameter lowering with `ParamConvention` (borrow → pointer type in local)
  - [ ] `@extern` detection → `ExternInfo`
  - [ ] Intrinsic detection → register without body
- [ ] `items/witness_lower.rs` — port witness generation logic:
  - [ ] Per-instantiation witnesses from `ConformingProtocolInstantiations`
  - [ ] Method binding via `TypeMembersByName` + label/param-type matching
  - [ ] Setter dispatch (`<name>.set` keys)
  - [ ] Protocol extension default fallback
  - [ ] Associated type bindings (including blanket conformances)
  - [ ] Use `kestrel_mir_2::substitute()` instead of local `substitute_type_params`
- [ ] `items/static_lower.rs` — `StaticDef` + `@fileconstant` extraction
- [ ] Test: `lower_module` on stdlib produces structs/enums/protocols/witnesses/functions
      with correct counts and no panics

### Phase 3 — Body context + emit helpers (`body/mod.rs`)

The infrastructure all body lowering depends on.

- [ ] `BodyCtx` struct with all fields from design doc
- [ ] `BodyCtx::new()` — create from `LowerCtx` + HIR + typed body
- [ ] `BodyCtx::new_for_closure()` — create for closure body lowering
- [ ] Block management: `new_block()`, `switch_to()`, `is_terminated()`
- [ ] Local management: `fresh_temp()`, `map_local()`, `resolve_local_type()`,
      `resolve_expr_type()`
- [ ] Emit helpers: `emit_assign()`, `emit_use_copy()`, `emit_use_move()`,
      `emit_assign_const()`, `emit_assign_op1()`, `emit_assign_op2()`,
      `emit_construct()`, `emit_call()`, `emit_drop()`
- [ ] Terminator helpers: `emit_ret()`, `emit_ret_unit()`, `emit_jump()`,
      `emit_branch()`, `emit_switch()`, `emit_panic()`
- [ ] Mode helpers: `use_mode_for(ty) -> UseMode`, `arg_mode_for(ty, convention) -> ArgMode`
- [ ] `lower_body()` — create locals, entry block, lower statements + tail expr
- [ ] `finish()` — return detached `MirBody`
- [ ] Wire `function_sig.rs` to call `lower_function_body()` for entities with `Body`
- [ ] Test: lower a function with params, verify body has correct local/block count

### Phase 4 — Expression and statement lowering

Core expression dispatch. Each file is an `impl BodyCtx` block.

**`body/expr.rs`:**
- [ ] `lower_expr()` — dispatch + promotion application
- [ ] `HirExpr::Literal` → delegate to `literal.rs`
- [ ] `HirExpr::Local` → `Operand::Place(Place::local(mapped_id))`
- [ ] `HirExpr::Tuple` → `emit_assign` with `Rvalue::Tuple`
- [ ] `HirExpr::Field` — stored field → `Place::field(FieldIdx)` via `resolve_field_idx`;
      computed property → getter call; protocol property → witness call;
      static property → `Place::Global`
- [ ] `HirExpr::TupleIndex` → `Place::tuple_index()`
- [ ] `HirExpr::Def` → `Operand::Const(Immediate::function_ref(...))`
- [ ] `HirExpr::OverloadSet` → resolve via `typed.resolutions`, same as Def
- [ ] `HirExpr::ImplicitMember` → resolve + lower
- [ ] `HirExpr::Call` → delegate to `call/mod.rs`
- [ ] `HirExpr::MethodCall` → delegate to `call/mod.rs`
- [ ] `HirExpr::ProtocolCall` → delegate to `call/mod.rs`
- [ ] `HirExpr::Sugar` → lower inner expression (desugaring already done by HIR)
- [ ] `HirExpr::Error` → `Operand::Const(Immediate::error())`
- [ ] Delegate to `control.rs`: If, Loop, Match, Break, Continue, Return, Block
- [ ] Delegate to `closure.rs`: Closure
- [ ] Delegate to `literal.rs`: Array, Dict

**`body/stmt.rs`:**
- [ ] `lower_stmt()` dispatch
- [ ] `HirStmt::Let` — create local, lower init value, emit assign
- [ ] `HirStmt::Expr` — lower expression, discard result
- [ ] `HirStmt::Deinit` — emit field deinit
- [ ] `HirExpr::Assign` — setter classification (`try_setter_assign`), then
      stored-place assignment fallback. Setter calls emit through `emit_call()`.

**Test:** lower `abs(x)` function with if/else, verify block count and terminator types

### Phase 5 — Control flow (`body/control.rs`)

- [ ] `lower_if()` — condition → branch → then/else blocks → join
- [ ] `lower_loop()` — header block, exit block, push/pop loop stack
- [ ] `lower_break()` — find loop by label, jump to exit
- [ ] `lower_continue()` — find loop by label, jump to header
- [ ] `lower_return()` — emit ret terminator, handle failure-return blocks
- [ ] `lower_hir_block()` — lower statements + tail in a block scope
- [ ] `collect_block_locals()` — for scope-live tracking

**Test:** lower function with nested loops + break/continue, verify CFG shape

### Phase 6 — Call dispatch (`body/call/`)

**`body/call/mod.rs`:**
- [ ] `lower_call()` — try_* chain as specified in design
- [ ] `emit_resolved_call()` — entity → callee → emit, single protocol-vs-direct branch
- [ ] `try_panic()` — detect `lang.panic`/`lang.panic_unwind`, emit Panic terminator
- [ ] Indirect call handling (thin/thick function pointers) as branch in `emit_resolved_call`

**`body/call/args.rs`:**
- [ ] `resolve_callee_and_type_args()` — single type-arg resolution function
- [ ] `resolve_entity()` — find callee entity from inference or HIR
- [ ] `resolve_type_args()` — the unified cascade (inference → explicit → parent struct)
- [ ] `lower_and_mode_args()` — lower call args + assign ArgMode per param convention
- [ ] `expand_default_args()` — fill missing args from default-value entities

**`body/call/intrinsic.rs`:**
- [ ] Static `INTRINSIC_TABLE` with all ~100 entries
- [ ] `try_intrinsic()` — table lookup + emit Op1/Op2/Op3
- [ ] Special cases: `panic` (if not caught by `try_panic`), float constants, pointer ops

**`body/call/init.rs`:**
- [ ] `emit_init_call()` — detect regular vs effectful
- [ ] Regular init: allocate temp, prepend &mut self, emit call, return temp
- [ ] Effectful init: allocate temp, call, switch on discriminant, wrap in Optional/Result
- [ ] Init field flags (SetDeinitFlag) scoped entirely within this file

**`body/call/construct.rs`:**
- [ ] `try_enum_construct()` — detect `NodeKind::EnumCase`, emit `Rvalue::EnumVariant`
- [ ] `try_struct_construct()` — detect memberwise struct init, emit `Rvalue::Construct`

**Test:** lower stdlib, verify call count > 100, witness method calls present, intrinsic ops present

### Phase 7 — Literals (`body/literal.rs`)

- [ ] Primitive literals: Int → `Immediate::i64()`, Float → `Immediate::f64()`, etc.
- [ ] String literals: decode escapes, emit `str.ptr` + `str.len` + init call
- [ ] Bool/Null literals
- [ ] `HirExpr::Array` → find `init(from:)` on target type, emit init call with elements
- [ ] `HirExpr::Dict` → find dict init, emit key/value pairs via `insert` calls
- [ ] Literal promotion: `lower_expr_with_hint()` for type-directed literal lowering

**Test:** lower function with array/dict literals, verify Construct + Call statements

### Phase 8 — Closures (`body/closure.rs`)

- [ ] `find_captures()` — free function, walks HirBlock for Local references
- [ ] `create_env_struct()` — synthesize StructDef for captured locals
- [ ] `create_closure_func()` — synthesize FunctionDef with env param
- [ ] `lower_closure()` — fresh BodyCtx, load captures from env, lower body,
      attach, emit ApplyPartial in parent
- [ ] Register `ClosureInfo` in module

**Test:** lower function with closure capturing a local, verify env struct + closure function exist

### Phase 9 — Pattern matching (`body/pattern.rs`)

- [ ] `lower_match()` — lower scrutinee to place, compile decision tree via
      `kestrel_pattern_matching::compile_decision_tree()`
- [ ] `emit_decision_tree()` — recursive: Switch nodes → branch/switch terminators,
      Success nodes → bind + lower arm body
- [ ] Boolean branch optimization (2-case true/false → Branch terminator)
- [ ] String match: chain of `Matchable.matches` calls (not SwitchTerminator)
- [ ] `emit_bindings()` — bind pattern variables from scrutinee place via access path
- [ ] `constructor_to_switch_case()` — map pattern constructors to `SwitchCase` variants

**Test:** lower function with match on enum + int range, verify switch terminator shape

### Phase 10 — Static init synthesis (`items/static_lower.rs`)

- [ ] `synthesize_static_inits()` — create per-static init thunks + master
      `__kestrel_init_statics` function
- [ ] `inject_init_call_into_main()` — prepend call to main's entry block
- [ ] Init thunks reuse `lower_function_body()` on the static entity

**Test:** lower module with global variable, verify init thunk + main injection

### Phase 11 — Validation (`validate.rs`)

- [ ] Walk all structs, enums, functions, statics for `MirTy::Error` (via TyId → arena lookup)
- [ ] Emit ICE diagnostics via `QueryContext::accumulate()`
- [ ] Return error count for `module.lowering_error_count`

### Phase 12 — Wire into compiler

- [ ] Add `kestrel-mir-lower-2` to workspace `Cargo.toml`
- [ ] Wire `kestrel-compiler` pipeline to call `kestrel_mir_lower_2::lower_module()`
      instead of `kestrel_mir_lower::lower_module()`
- [ ] Verify: full triage run green
- [ ] Verify: `cargo fmt` + `cargo clippy -p kestrel-mir-lower-2` clean

### Phase 13 — Cleanup

- [ ] Remove `kestrel-mir-lower` from workspace (after confirming nothing else depends on it)
- [ ] Update `docs/contributing/` if any pipeline docs reference the old crate
- [ ] Update any `AGENTS.md` files that reference `kestrel-mir-lower`

## Verification

- [ ] Targeted triage run green after each phase
- [ ] Full triage run green before Phase 12 commit
- [ ] `cargo fmt` + `cargo clippy -p kestrel-mir-lower-2` clean
- [ ] stdlib lowering: 0 MirTy::Error in struct fields (same bar as current crate)
- [ ] All existing `kestrel-mir-lower` integration tests ported and passing
