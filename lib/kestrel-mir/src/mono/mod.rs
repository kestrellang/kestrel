pub mod collect;
pub mod expand;
pub mod mangle;
pub mod types;
pub mod verify;
pub mod witness;

pub use collect::CollectionResult;
pub use types::{
    InstantiationKey, MonoEnum, MonoEnumCase, MonoField, MonoFunction, MonoModule, MonoParam,
    MonoStruct, MonoTypeKey,
};
pub use verify::{MonoVerifyError, MonoVerifyResult};
pub use witness::MonoError;

use std::collections::HashMap;

use indexmap::IndexMap;
use kestrel_hecs::Entity;

use crate::body::OssaBody;
use crate::callee::Callee;
use crate::immediate::ImmediateKind;
use crate::inst::InstKind;
use crate::item::enum_def::EnumDef;
use crate::item::function::{FunctionDef, FunctionKind};
use crate::item::protocol::ProtocolDef;
use crate::item::struct_def::StructDef;
use crate::item::witness::WitnessDef;
use crate::item::{Layout, TargetConfig};
use crate::layout::StructLayout;
use crate::substitute::{SubstMap, substitute};
use crate::ty::{MirTy, TyArena};
use crate::value::Ownership;
use crate::{CopyBehavior, MirModule, MonoFuncId, TyId};

/// Check if a function needs self_type in its InstantiationKey.
///
/// Monomorphize a generic MirModule into a concrete MonoModule.
pub fn monomorphize(
    module: MirModule,
    target: &TargetConfig,
) -> Result<MonoModule, Vec<MonoError>> {
    // Destructure to split borrows: &mut ty_arena alongside &functions etc.
    let MirModule {
        name: _,
        functions,
        structs,
        enums,
        protocols,
        witnesses,
        statics,
        mut ty_arena,
        entity_names,
    } = module;

    // Phase 1: Instantiation discovery
    let CollectionResult {
        instantiations,
        witness_cache,
    } = collect::collect_all(
        &functions,
        &structs,
        &enums,
        &protocols,
        &witnesses,
        &mut ty_arena,
        &entity_names,
    )?;

    // Phase 2: Body monomorphization
    let mut mono_bodies: Vec<MonoBodyResult> = Vec::with_capacity(instantiations.len());

    let _ = witness_cache;
    for key in &instantiations {
        let result = monomorphize_body(
            &mut ty_arena,
            &functions,
            &protocols,
            &witnesses,
            &entity_names,
            key,
        );
        mono_bodies.push(result);
    }

    // Phase 3a: Resolve Witness -> Direct.
    // resolved_witnesses is keyed by (block_idx, inst_idx), so it must be
    // consumed before any pass that could shift instruction indices.
    for body_result in &mut mono_bodies {
        resolve_witnesses_to_direct(body_result);
    }

    // Phase 3b: ID assignment + callee rewriting
    let func_id_map: HashMap<InstantiationKey, MonoFuncId> = instantiations
        .iter()
        .enumerate()
        .map(|(i, key)| (key.clone(), MonoFuncId::new(i)))
        .collect();

    for body_result in mono_bodies.iter_mut() {
        rewrite_callees(body_result, &func_id_map, &functions);
    }

    // Phase 4: Type and layout resolution
    let (mono_structs, mono_enums) = resolve_types_and_layouts(
        &mut ty_arena,
        &structs,
        &enums,
        &witnesses,
        &mono_bodies,
        target,
    );

    // Phase 5: Assembly
    let mut mono_module = MonoModule::new(ty_arena);

    for (i, key) in instantiations.iter().enumerate() {
        let body_result = &mono_bodies[i];
        let func_name = entity_names
            .get(&key.func_entity)
            .map(|s| s.as_str())
            .unwrap_or("<unknown>");

        // Determine receiver convention for mangling
        let receiver = functions.get(&key.func_entity).and_then(|f| match &f.kind {
            FunctionKind::Method { receiver, .. } => Some(*receiver),
            _ => None,
        });

        // Safety net: resolve any residual projections in key type_args/self_type.
        // Phase 1 should produce fully-resolved keys, but deep_resolve catches
        // edge cases where substitute() couldn't resolve nested projections.
        let resolved_type_args: Vec<TyId> = key
            .type_args
            .iter()
            .map(|&ta| {
                collect::substitute_and_resolve(
                    &mut mono_module.ty_arena,
                    &witnesses,
                    ta,
                    &SubstMap::new(),
                )
            })
            .collect();
        let resolved_self = key.self_type.map(|st| {
            collect::substitute_and_resolve(
                &mut mono_module.ty_arena,
                &witnesses,
                st,
                &SubstMap::new(),
            )
        });

        let mangled_name = mangle::mangle_function(
            &mono_module.ty_arena,
            &entity_names,
            func_name,
            &resolved_type_args,
            resolved_self,
            &body_result.params,
            body_result.ret,
            receiver,
        );

        mono_module.add_function(MonoFunction {
            name: mangled_name,
            source: key.func_entity,
            type_args: resolved_type_args,
            self_type: resolved_self,
            params: body_result.params.clone(),
            ret: body_result.ret,
            body: body_result.body.clone(),
            extern_info: body_result.extern_info.clone(),
        });
    }

    mono_module.entity_names = entity_names;
    mono_module.structs = mono_structs;
    mono_module.enums = mono_enums;

    // Copy statics (statics aren't monomorphized — use StaticDef directly)
    for s in statics.values() {
        mono_module.statics.insert(s.entity, s.clone());
    }

    Ok(mono_module)
}

// -- Phase 2: Body monomorphization --

struct MonoBodyResult {
    body: Option<OssaBody>,
    params: Vec<MonoParam>,
    ret: TyId,
    extern_info: Option<crate::item::function::ExternInfo>,
    /// Resolved witness callees: (block_idx, inst_idx) -> target key
    resolved_witnesses: HashMap<(usize, usize), InstantiationKey>,
}

fn monomorphize_body(
    arena: &mut TyArena,
    functions: &IndexMap<Entity, FunctionDef>,
    protocols: &IndexMap<Entity, ProtocolDef>,
    witnesses: &[WitnessDef],
    entity_names: &IndexMap<Entity, String>,
    key: &InstantiationKey,
) -> MonoBodyResult {
    let func = functions
        .get(&key.func_entity)
        .expect("instantiation key must reference a valid function");

    let subst = collect::build_subst(
        func,
        &key.type_args,
        key.self_type,
        arena,
        protocols,
        witnesses,
    );

    // Substitute param and return types
    let params: Vec<MonoParam> = func
        .params
        .iter()
        .map(|p| {
            MonoParam::with_label(
                &p.name,
                substitute(arena, p.ty, &subst),
                p.convention,
                p.external_label.clone(),
            )
        })
        .collect();
    let ret = substitute(arena, func.ret, &subst);

    let extern_info = func.extern_info.clone();

    let Some(body) = &func.body else {
        return MonoBodyResult {
            body: None,
            params,
            ret,
            extern_info,
            resolved_witnesses: HashMap::new(),
        };
    };

    // Clone and substitute the body
    let mut mono_body = body.clone();
    let mut resolved_witnesses = HashMap::new();

    // Substitute value types
    for value in &mut mono_body.values {
        value.ty = substitute(arena, value.ty, &subst);
    }

    // Substitute block param types
    for block in &mut mono_body.blocks {
        for param in &mut block.params {
            param.ty = substitute(arena, param.ty, &subst);
        }
    }

    // Walk instructions and substitute types
    for (bi, block) in mono_body.blocks.iter_mut().enumerate() {
        for (ii, inst) in block.insts.iter_mut().enumerate() {
            substitute_inst(
                arena,
                witnesses,
                protocols,
                functions,
                entity_names,
                &mut inst.kind,
                &subst,
                key.self_type,
                bi,
                ii,
                &mut resolved_witnesses,
            );
        }
        // No terminator substitution needed — MIR terminators carry ValueId only
    }

    // Re-derive ownership after type substitution. Guaranteed values keep their
    // ownership (they represent borrows); everything else becomes Owned.
    for value in &mut mono_body.values {
        if value.ownership != Ownership::Guaranteed {
            value.ownership = Ownership::Owned;
        }
    }
    for block in &mut mono_body.blocks {
        for param in &mut block.params {
            if param.ownership != Ownership::Guaranteed {
                param.ownership = Ownership::Owned;
            }
        }
    }

    // Resolve any AssociatedProjections that survived substitution. This
    // handles cases where substitute() replaces a TypeParam base to produce
    // a concrete-base projection that isn't in the SubstMap's assoc_types.
    let resolve = |arena: &mut TyArena, ty: TyId| -> TyId {
        collect::substitute_and_resolve(arena, witnesses, ty, &subst)
    };
    let params: Vec<MonoParam> = params
        .into_iter()
        .map(|mut p| {
            p.ty = resolve(arena, p.ty);
            p
        })
        .collect();
    let ret = resolve(arena, ret);
    for value in &mut mono_body.values {
        value.ty = resolve(arena, value.ty);
    }
    for block in &mut mono_body.blocks {
        for param in &mut block.params {
            param.ty = resolve(arena, param.ty);
        }
    }

    MonoBodyResult {
        body: Some(mono_body),
        params,
        ret,
        extern_info,
        resolved_witnesses,
    }
}

/// Substitute types in a single instruction (the OSSA analogue of the
/// older place-based `substitute_rvalue` / `substitute_operand` /
/// `substitute_terminator` helpers).
fn substitute_inst(
    arena: &mut TyArena,
    witnesses: &[WitnessDef],
    protocols: &IndexMap<Entity, ProtocolDef>,
    functions: &IndexMap<Entity, FunctionDef>,
    entity_names: &IndexMap<Entity, String>,
    kind: &mut InstKind,
    subst: &SubstMap,
    parent_self: Option<TyId>,
    block_idx: usize,
    inst_idx: usize,
    resolved_witnesses: &mut HashMap<(usize, usize), InstantiationKey>,
) {
    // Embedded types must be `substitute_and_resolve`d, not just `substitute`d:
    // substitution replaces a projection's TypeParam base with a concrete type
    // but leaves the projection itself in place (it isn't keyed in the SubstMap's
    // assoc_types). Only `deep_resolve` runs the witness lookup that turns
    // `Array[Int64].TargetIterator.Item` into `Int64`. The value/block-param
    // pass at the call site re-resolves those, but instruction-embedded types
    // (enum/struct/array construction, addr ops) are only seen here — without
    // this, a surviving `AssociatedProjection` in e.g. an `Optional` enum payload
    // fails post-mono verify ("AssociatedProjection in Enum type").
    let resolve = |arena: &mut TyArena, ty: TyId| {
        collect::substitute_and_resolve(arena, witnesses, ty, subst)
    };
    match kind {
        // Memory access instructions with embedded type
        InstKind::CopyAddr { ty, .. }
        | InstKind::Take { ty, .. }
        | InstKind::BeginBorrowAddr { ty, .. }
        | InstKind::BeginMutBorrowAddr { ty, .. }
        | InstKind::DestroyAddr { ty, .. }
        | InstKind::FieldAddr { ty, .. }
        | InstKind::Uninit { ty, .. } => {
            *ty = resolve(arena, *ty);
        },

        // Aggregate construction
        InstKind::Struct { ty, .. } => {
            *ty = resolve(arena, *ty);
        },
        InstKind::Enum { enum_ty, .. } => {
            *enum_ty = resolve(arena, *enum_ty);
        },
        InstKind::Array { element_ty, .. } => {
            *element_ty = resolve(arena, *element_ty);
        },

        // Ops with embedded type
        InstKind::Op1 { op, .. } => {
            substitute_op_type(arena, witnesses, op, subst);
        },
        InstKind::Op2 { op, .. } => {
            substitute_op_type(arena, witnesses, op, subst);
        },
        InstKind::Op3 { op, .. } => {
            substitute_op_type(arena, witnesses, op, subst);
        },

        // Constants
        InstKind::Literal { value, .. } => {
            substitute_immediate(arena, witnesses, &mut value.kind, subst);
        },

        // Calls and partial applications both reference a callable through a
        // `Callee` — substitute its type args / self_type identically so the
        // instantiation key matches what `rewrite_callee` later looks up.
        InstKind::Call { callee, .. } | InstKind::ApplyPartial { callee, .. } => {
            substitute_callee_and_resolve(
                arena,
                witnesses,
                protocols,
                functions,
                entity_names,
                callee,
                subst,
                parent_self,
                block_idx,
                inst_idx,
                resolved_witnesses,
            );
        },

        // All other InstKinds carry only ValueId — no substitution needed
        _ => {},
    }
}

fn substitute_immediate(
    arena: &mut TyArena,
    witnesses: &[WitnessDef],
    kind: &mut ImmediateKind,
    subst: &SubstMap,
) {
    let resolve = |arena: &mut TyArena, ty: TyId| {
        collect::substitute_and_resolve(arena, witnesses, ty, subst)
    };
    match kind {
        ImmediateKind::SizeOf(ty) | ImmediateKind::AlignOf(ty) | ImmediateKind::NullPtr(ty) => {
            *ty = resolve(arena, *ty);
        },
        ImmediateKind::FunctionRef {
            type_args,
            self_type,
            ..
        } => {
            for ta in type_args.iter_mut() {
                *ta = resolve(arena, *ta);
            }
            if let Some(st) = self_type {
                *st = resolve(arena, *st);
            }
        },
        _ => {},
    }
}

fn substitute_op_type(
    arena: &mut TyArena,
    witnesses: &[WitnessDef],
    op: &mut crate::op::Op,
    subst: &SubstMap,
) {
    use crate::op::Op;
    let resolve = |arena: &mut TyArena, ty: TyId| {
        collect::substitute_and_resolve(arena, witnesses, ty, subst)
    };
    match op {
        Op::PtrFromAddress(ty)
        | Op::PtrRead(ty)
        | Op::PtrWrite(ty)
        | Op::PtrNull(ty)
        | Op::PtrTo(ty)
        | Op::PtrCast(ty)
        | Op::PtrBitcast(ty)
        | Op::SizeOf(ty)
        | Op::AlignOf(ty)
        | Op::StackAlloc(ty) => {
            *ty = resolve(arena, *ty);
        },
        _ => {},
    }
}

fn substitute_callee_and_resolve(
    arena: &mut TyArena,
    witnesses: &[WitnessDef],
    protocols: &IndexMap<Entity, ProtocolDef>,
    functions: &IndexMap<Entity, FunctionDef>,
    entity_names: &IndexMap<Entity, String>,
    callee: &mut Callee,
    subst: &SubstMap,
    parent_self: Option<TyId>,
    block_idx: usize,
    inst_idx: usize,
    resolved_witnesses: &mut HashMap<(usize, usize), InstantiationKey>,
) {
    match callee {
        Callee::Direct {
            func,
            type_args,
            self_type,
        } => {
            for ta in type_args.iter_mut() {
                *ta = collect::substitute_and_resolve(arena, witnesses, *ta, subst);
            }
            if let Some(st) = self_type {
                *st = collect::substitute_and_resolve(arena, witnesses, *st, subst);
            }
            // Nested callees (closures/thunks) inherit parent's self_type
            // so rewrite_callee can look them up with the correct key.
            if self_type.is_none() && parent_self.is_some() {
                if let Some(f) = functions.get(func) {
                    if matches!(
                        f.kind,
                        FunctionKind::Closure { .. }
                            | FunctionKind::ClosureCall { .. }
                            | FunctionKind::Thunk { .. }
                    ) {
                        *self_type = parent_self;
                    }
                }
            }
        },
        Callee::Witness {
            protocol,
            method,
            self_type,
            method_type_args,
        } => {
            *self_type = collect::substitute_and_resolve(arena, witnesses, *self_type, subst);
            for ta in method_type_args.iter_mut() {
                *ta = collect::substitute_and_resolve(arena, witnesses, *ta, subst);
            }
            // Resolve witness to concrete function
            let witness_result = witness::resolve_witness_call(
                arena,
                witnesses,
                protocols,
                functions,
                entity_names,
                *protocol,
                method,
                *self_type,
                method_type_args,
            );
            if let Ok(resolved) = witness_result {
                resolved_witnesses.insert(
                    (block_idx, inst_idx),
                    InstantiationKey::new(
                        resolved.func_entity,
                        resolved.type_args,
                        resolved.self_type,
                    ),
                );
            }
        },
        _ => {},
    }
}

// -- Phase 3a: Witness resolution --

/// Resolve Callee::Witness -> Callee::Direct using the resolved_witnesses
/// map (keyed by pre-expansion block/inst indices). Must run before any
/// pass that could shift instruction indices.
fn resolve_witnesses_to_direct(body_result: &mut MonoBodyResult) {
    let Some(body) = &mut body_result.body else {
        return;
    };
    for (bi, block) in body.blocks.iter_mut().enumerate() {
        for (ii, inst) in block.insts.iter_mut().enumerate() {
            if let InstKind::Call {
                callee: callee @ Callee::Witness { .. },
                ..
            } = &mut inst.kind
            {
                if let Some(target_key) = body_result.resolved_witnesses.get(&(bi, ii)) {
                    *callee = Callee::Direct {
                        func: target_key.func_entity,
                        type_args: target_key.type_args.clone(),
                        self_type: target_key.self_type,
                    };
                }
            }
        }
    }
}

// -- Phase 3b: Callee rewriting --

fn rewrite_callees(
    body_result: &mut MonoBodyResult,
    func_id_map: &HashMap<InstantiationKey, MonoFuncId>,
    functions: &IndexMap<Entity, FunctionDef>,
) {
    let Some(body) = &mut body_result.body else {
        return;
    };

    for (bi, block) in body.blocks.iter_mut().enumerate() {
        for (ii, inst) in block.insts.iter_mut().enumerate() {
            match &mut inst.kind {
                InstKind::Call { callee, .. } | InstKind::ApplyPartial { callee, .. } => {
                    rewrite_callee(
                        callee,
                        bi,
                        ii,
                        &body_result.resolved_witnesses,
                        func_id_map,
                        functions,
                    );
                },
                InstKind::Literal { value, .. } => {
                    if let ImmediateKind::FunctionRef {
                        func,
                        type_args,
                        self_type,
                    } = &value.kind
                    {
                        let key = InstantiationKey::new(*func, type_args.clone(), *self_type);
                        if let Some(&mono_id) = func_id_map.get(&key) {
                            value.kind = ImmediateKind::MonoFunctionRef(mono_id);
                        }
                    }
                },
                _ => {},
            }
        }
    }
}

fn rewrite_callee(
    callee: &mut Callee,
    block_idx: usize,
    inst_idx: usize,
    resolved_witnesses: &HashMap<(usize, usize), InstantiationKey>,
    func_id_map: &HashMap<InstantiationKey, MonoFuncId>,
    functions: &IndexMap<Entity, FunctionDef>,
) {
    match callee {
        Callee::Direct {
            func,
            type_args,
            self_type,
        } => {
            // Mirror collection's arity normalization (collect::scan_callee) so
            // the lookup key matches the key the instance was enqueued under.
            let mut targs = type_args.clone();
            if let Some(f) = functions.get(&*func) {
                collect::normalize_direct_arity(&mut targs, f.type_params.len());
            }
            let key = InstantiationKey::new(*func, targs, *self_type);
            if let Some(&mono_id) = func_id_map.get(&key) {
                *callee = Callee::Resolved(mono_id);
            }
        },
        Callee::Witness { .. } => {
            if let Some(target_key) = resolved_witnesses.get(&(block_idx, inst_idx))
                && let Some(&mono_id) = func_id_map.get(target_key)
            {
                *callee = Callee::Resolved(mono_id);
            }
        },
        _ => {},
    }
}

// -- Phase 4: Type and layout resolution --

fn resolve_types_and_layouts(
    arena: &mut TyArena,
    structs: &IndexMap<Entity, StructDef>,
    enums: &IndexMap<Entity, EnumDef>,
    witnesses: &[WitnessDef],
    mono_bodies: &[MonoBodyResult],
    target: &TargetConfig,
) -> (
    IndexMap<MonoTypeKey, MonoStruct>,
    IndexMap<MonoTypeKey, MonoEnum>,
) {
    // Collect all concrete Named types from monomorphized bodies
    let mut concrete_types: IndexMap<(Entity, Vec<TyId>), ConcreteTypeKind> = IndexMap::new();

    for body_result in mono_bodies {
        if let Some(body) = &body_result.body {
            collect_named_types(arena, body, &mut concrete_types, structs, enums);
        }
    }

    // Compute layouts for concrete types (fixed-point loop)
    let mut mono_structs: IndexMap<MonoTypeKey, MonoStruct> = IndexMap::new();
    let mut mono_enums: IndexMap<MonoTypeKey, MonoEnum> = IndexMap::new();
    let mut layout_cache: HashMap<(Entity, Vec<TyId>), (u64, u64)> = HashMap::new();

    // Fixed-point: loop until no progress (handles dependency chains)
    loop {
        let mut progress = false;

        for ((entity, type_args), kind) in &concrete_types {
            let cache_key = (*entity, type_args.clone());
            if layout_cache.contains_key(&cache_key) {
                continue;
            }

            match kind {
                ConcreteTypeKind::Struct(struct_entity) => {
                    let sdef = &structs[struct_entity];
                    let subst =
                        build_type_subst(sdef.type_params.iter().map(|tp| tp.entity), type_args);

                    let mut layout = StructLayout::new();
                    let mut all_resolved = true;
                    let mut fields = Vec::new();

                    for field in &sdef.fields {
                        let concrete_ty =
                            collect::substitute_and_resolve(arena, witnesses, field.ty, &subst);
                        fields.push(MonoField::new(&field.name, concrete_ty));

                        if let Some((size, align)) =
                            mono_size_and_align(arena, concrete_ty, target, &layout_cache)
                        {
                            layout.append_field(StructLayout::scalar(size, align));
                        } else {
                            all_resolved = false;
                            break;
                        }
                    }

                    if all_resolved {
                        layout.pad_to_align();
                        layout_cache.insert(cache_key, (layout.size, layout.align));
                        let mut ms = MonoStruct::new(*entity, type_args.clone());
                        ms.fields = fields;
                        ms.type_info = sdef.type_info.clone();
                        ms.type_info.layout = Some(Layout::Struct(layout));
                        mono_structs.insert((*entity, type_args.clone()), ms);
                        progress = true;
                    }
                },
                ConcreteTypeKind::Enum(enum_entity) => {
                    let edef = &enums[enum_entity];
                    let subst =
                        build_type_subst(edef.type_params.iter().map(|tp| tp.entity), type_args);

                    let mut all_resolved = true;
                    let mut cases = Vec::new();
                    let mut variant_layouts = Vec::new();

                    for case in &edef.cases {
                        let mut case_layout = StructLayout::new();
                        let mut mono_fields = Vec::new();
                        for field in &case.payload_fields {
                            let concrete_ty =
                                collect::substitute_and_resolve(arena, witnesses, field.ty, &subst);
                            mono_fields.push(MonoField::new(&field.name, concrete_ty));
                            if let Some((size, align)) =
                                mono_size_and_align(arena, concrete_ty, target, &layout_cache)
                            {
                                case_layout.append_field(StructLayout::scalar(size, align));
                            } else {
                                all_resolved = false;
                                break;
                            }
                        }
                        if !all_resolved {
                            break;
                        }
                        case_layout.pad_to_align();
                        variant_layouts.push(case_layout);
                        let mut mc = MonoEnumCase::new(&case.name, case.discriminant);
                        mc.payload_fields = mono_fields;
                        cases.push(mc);
                    }

                    if all_resolved {
                        let enum_layout = build_enum_layout(&variant_layouts, edef.cases.len());
                        layout_cache.insert(cache_key, (enum_layout.size, enum_layout.align));
                        let mut me = MonoEnum::new(
                            *entity,
                            type_args.clone(),
                            enum_layout.discriminant_width,
                        );
                        me.cases = cases;
                        me.type_info = edef.type_info.clone();
                        me.type_info.layout = Some(Layout::Enum(enum_layout.clone()));
                        mono_enums.insert((*entity, type_args.clone()), me);
                        progress = true;
                    }
                },
            }
        }

        if !progress {
            break;
        }
    }

    refine_mono_copy_behavior(arena, structs, enums, &mut mono_structs, &mut mono_enums);

    (mono_structs, mono_enums)
}

/// Re-derive per-instantiation `copy` behavior for conditional containers.
///
/// `MonoStruct`/`MonoEnum.type_info.copy` is cloned from the *generic* def,
/// which for a conditional container (`: not Copyable` + `extend …: Copyable
/// where T: Copyable`) is `None`. For a concrete instantiation the type is
/// Copyable iff every gating arg (the positions in
/// `StructDef::conditionally_copyable`) is itself Copyable — so `Result[Int64,
/// Error]` becomes `Bitwise` while `Result[Array[Int64], E]` stays `None`.
/// Mirrors `ty_query::instantiated_copy_behavior` but over already-mono types
/// (args are concrete, so no `where_clause`).
///
/// Inert for types with no gating positions. A fixed point handles nesting
/// (`Result[Optional[Int], E]`): each round refines types whose gating children
/// are already refined, until no `copy` field changes.
fn refine_mono_copy_behavior(
    arena: &TyArena,
    structs: &IndexMap<Entity, StructDef>,
    enums: &IndexMap<Entity, EnumDef>,
    mono_structs: &mut IndexMap<MonoTypeKey, MonoStruct>,
    mono_enums: &mut IndexMap<MonoTypeKey, MonoEnum>,
) {
    loop {
        // (key, is_struct, new copy) — collected read-only, applied after.
        let mut updates: Vec<(MonoTypeKey, bool, CopyBehavior)> = Vec::new();
        for (key, ms) in mono_structs.iter() {
            let gating = &structs[&ms.source].conditionally_copyable;
            if gating.is_empty() {
                continue;
            }
            let want = conditional_copy(
                arena,
                ms.source,
                gating,
                &ms.type_args,
                mono_structs,
                mono_enums,
            );
            if want != ms.type_info.copy {
                updates.push((key.clone(), true, want));
            }
        }
        for (key, me) in mono_enums.iter() {
            let gating = &enums[&me.source].conditionally_copyable;
            if gating.is_empty() {
                continue;
            }
            let want = conditional_copy(
                arena,
                me.source,
                gating,
                &me.type_args,
                mono_structs,
                mono_enums,
            );
            if want != me.type_info.copy {
                updates.push((key.clone(), false, want));
            }
        }
        if updates.is_empty() {
            break;
        }
        for (key, is_struct, cb) in updates {
            if is_struct {
                if let Some(ms) = mono_structs.get_mut(&key) {
                    ms.type_info.copy = cb;
                }
            } else if let Some(me) = mono_enums.get_mut(&key) {
                me.type_info.copy = cb;
            }
        }
    }
}

/// Copyability of a conditional container, mirroring `ty_query`: any gating arg
/// `None` → `None`; all `Bitwise` → `Bitwise`; all Copyable with ≥1 `Clone` →
/// `Clone(entity)` (copyable element-wise via the container's clone shim).
fn conditional_copy(
    arena: &TyArena,
    entity: Entity,
    gating: &[usize],
    type_args: &[TyId],
    mono_structs: &IndexMap<MonoTypeKey, MonoStruct>,
    mono_enums: &IndexMap<MonoTypeKey, MonoEnum>,
) -> CopyBehavior {
    let mut saw_clone = false;
    for &pos in gating {
        let Some(&arg) = type_args.get(pos) else {
            return CopyBehavior::None;
        };
        match concrete_copy(arena, arg, mono_structs, mono_enums) {
            CopyBehavior::Bitwise => {},
            CopyBehavior::Clone(_) => saw_clone = true,
            CopyBehavior::None => return CopyBehavior::None,
        }
    }
    if saw_clone {
        CopyBehavior::Clone(entity)
    } else {
        CopyBehavior::Bitwise
    }
}

/// Copy behavior of an already-monomorphized concrete type. Named types are
/// looked up in the mono maps (their `copy` is correct, or being refined this
/// round); primitives/pointers/functions are bit-copyable; a tuple is Bitwise
/// iff every element is.
fn concrete_copy(
    arena: &TyArena,
    ty: TyId,
    mono_structs: &IndexMap<MonoTypeKey, MonoStruct>,
    mono_enums: &IndexMap<MonoTypeKey, MonoEnum>,
) -> CopyBehavior {
    match arena.get(ty) {
        MirTy::Named { entity, type_args } => {
            let key = (*entity, type_args.clone());
            mono_structs
                .get(&key)
                .map(|s| s.type_info.copy.clone())
                .or_else(|| mono_enums.get(&key).map(|e| e.type_info.copy.clone()))
                .unwrap_or(CopyBehavior::Bitwise)
        },
        MirTy::Tuple(elems) => {
            for &e in elems {
                if !matches!(
                    concrete_copy(arena, e, mono_structs, mono_enums),
                    CopyBehavior::Bitwise
                ) {
                    return CopyBehavior::None;
                }
            }
            CopyBehavior::Bitwise
        },
        _ => CopyBehavior::Bitwise,
    }
}

enum ConcreteTypeKind {
    Struct(Entity),
    Enum(Entity),
}

/// Walk an OssaBody and collect all concrete Named types.
fn collect_named_types(
    arena: &TyArena,
    body: &OssaBody,
    out: &mut IndexMap<(Entity, Vec<TyId>), ConcreteTypeKind>,
    structs: &IndexMap<Entity, StructDef>,
    enums: &IndexMap<Entity, EnumDef>,
) {
    // Walk value types
    for value in &body.values {
        collect_named_type_from_ty(arena, value.ty, out, structs, enums);
    }
    // Walk block param types
    for block in &body.blocks {
        for param in &block.params {
            collect_named_type_from_ty(arena, param.ty, out, structs, enums);
        }
    }
    // Walk instruction types (Struct.ty, Enum.enum_ty, Array.element_ty, Literal immediates)
    for block in &body.blocks {
        for inst in &block.insts {
            match &inst.kind {
                InstKind::Struct { ty, .. } => {
                    collect_named_type_from_ty(arena, *ty, out, structs, enums);
                },
                InstKind::Enum { enum_ty, .. } => {
                    collect_named_type_from_ty(arena, *enum_ty, out, structs, enums);
                },
                InstKind::Array { element_ty, .. } => {
                    collect_named_type_from_ty(arena, *element_ty, out, structs, enums);
                },
                InstKind::Literal { value, .. } => match &value.kind {
                    ImmediateKind::SizeOf(ty)
                    | ImmediateKind::AlignOf(ty)
                    | ImmediateKind::NullPtr(ty) => {
                        collect_named_type_from_ty(arena, *ty, out, structs, enums);
                    },
                    _ => {},
                },
                // CopyAddr/Take/BeginBorrowAddr/BeginMutBorrowAddr/DestroyAddr/FieldAddr/Uninit
                // carry ty but those are address types (Pointer), not Named
                _ => {},
            }
        }
    }
}

fn collect_named_type_from_ty(
    arena: &TyArena,
    ty: TyId,
    out: &mut IndexMap<(Entity, Vec<TyId>), ConcreteTypeKind>,
    structs: &IndexMap<Entity, StructDef>,
    enums: &IndexMap<Entity, EnumDef>,
) {
    match arena.get(ty) {
        MirTy::Named { entity, type_args } => {
            let entity = *entity;
            let type_args = type_args.clone();
            let key = (entity, type_args.clone());
            if !out.contains_key(&key) {
                if structs.contains_key(&entity) {
                    out.insert(key, ConcreteTypeKind::Struct(entity));
                } else if enums.contains_key(&entity) {
                    out.insert(key, ConcreteTypeKind::Enum(entity));
                }
            }
            // Recurse into type args
            for &arg in &type_args {
                collect_named_type_from_ty(arena, arg, out, structs, enums);
            }
        },
        MirTy::Pointer(inner) => {
            collect_named_type_from_ty(arena, *inner, out, structs, enums);
        },
        MirTy::Tuple(elems) => {
            for &elem in elems {
                collect_named_type_from_ty(arena, elem, out, structs, enums);
            }
        },
        _ => {},
    }
}

fn build_type_subst(
    type_param_entities: impl Iterator<Item = Entity>,
    type_args: &[TyId],
) -> SubstMap {
    let mut subst = SubstMap::new();
    for (entity, &arg) in type_param_entities.zip(type_args.iter()) {
        subst.type_params.insert(entity, arg);
    }
    subst
}

// Reuse layout functions from passes/layout.rs
use crate::passes::layout::{build_enum_layout, primitive_size_and_align};

/// Compute size and alignment for a concrete type, looking up mono layouts.
fn mono_size_and_align(
    arena: &TyArena,
    ty: TyId,
    target: &TargetConfig,
    layout_cache: &HashMap<(Entity, Vec<TyId>), (u64, u64)>,
) -> Option<(u64, u64)> {
    if let Some(sa) = primitive_size_and_align(arena.get(ty), target) {
        return Some(sa);
    }
    match arena.get(ty) {
        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            if elems.is_empty() {
                return Some((0, 1));
            }
            let mut layout = StructLayout::new();
            for elem in &elems {
                let (size, align) = mono_size_and_align(arena, *elem, target, layout_cache)?;
                layout.append_field(StructLayout::scalar(size, align));
            }
            layout.pad_to_align();
            Some((layout.size, layout.align))
        },

        MirTy::Named { entity, type_args } => {
            let key = (*entity, type_args.clone());
            layout_cache.get(&key).copied()
        },

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{BasicBlock, BlockParam};
    use crate::body::OssaBody;
    use crate::callee::Callee;
    use crate::inst::{CallArg, InstKind, Instruction};
    use crate::item::TypeParamDef;
    use crate::item::function::{FunctionDef, FunctionKind, ParamDef};
    use crate::terminator::{Terminator, TerminatorKind};
    use crate::ty::ParamConvention;
    use crate::value::ValueDef;
    use crate::{BlockId, ValueId};

    fn entity(id: u32) -> Entity {
        Entity::from_raw(id)
    }

    /// Build a single-block OssaBody with the given instructions, returning `ret_val`.
    fn make_body(insts: Vec<Instruction>, ret_val: ValueId, values: Vec<ValueDef>) -> OssaBody {
        let mut block = BasicBlock::new();
        block.insts = insts;
        block.terminator = Terminator::new(TerminatorKind::Return(ret_val));
        OssaBody {
            values,
            blocks: vec![block],
            entry: BlockId::new(0),
            param_count: 0,
        }
    }

    #[test]
    fn monomorphize_concrete_function() {
        let mut module = MirModule::new("test");
        let unit = module.ty_arena.unit();

        let ret_val = ValueId::new(0);
        let func = FunctionDef {
            entity: entity(1),
            name: "main".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], ret_val, vec![ValueDef::owned(unit)])),
            extern_info: None,
        };
        module.add_function(func);
        module.register_name(entity(1), "main");

        let target = TargetConfig::host_64();
        let mono = monomorphize(module, &target).unwrap();

        assert_eq!(mono.functions.len(), 1);
        assert!(mono.functions[0].body.is_some());
        assert!(mono.functions[0].name.starts_with("_K0"));
    }

    #[test]
    fn monomorphize_generic_function_via_call() {
        let mut module = MirModule::new("test");
        let unit = module.ty_arena.unit();
        let i64_ty = module.ty_arena.i64();
        let tp_ty = module.ty_arena.intern(MirTy::TypeParam(entity(3)));

        // generic_fn[T](x: T) -> T
        let x_val = ValueId::new(0);
        let generic_fn = FunctionDef {
            entity: entity(2),
            name: "identity".into(),
            kind: FunctionKind::Free,
            type_params: vec![TypeParamDef::new(entity(3), "T")],
            params: vec![ParamDef::new("x", x_val, tp_ty, ParamConvention::Consuming)],
            ret: tp_ty,
            where_clause: None,
            body: Some({
                let mut body = OssaBody::new();
                // value 0: the parameter x
                body.alloc_value(ValueDef::owned(tp_ty));
                let entry = body.alloc_block();
                body.entry = entry;
                body.param_count = 1;
                // Entry block has x as a block param
                body.block_mut(entry).params.push(BlockParam {
                    value: x_val,
                    ty: tp_ty,
                    ownership: Ownership::Owned,
                });
                body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(x_val));
                body
            }),
            extern_info: None,
        };

        // main() calls identity[Int64]
        let ret_val = ValueId::new(0);
        let arg_val = ValueId::new(1);
        let result_val = ValueId::new(2);
        let call_inst = Instruction::new(InstKind::Call {
            result: Some(result_val),
            callee: Callee::Direct {
                func: entity(2),
                type_args: vec![i64_ty],
                self_type: None,
            },
            args: vec![CallArg {
                value: arg_val,
                convention: ParamConvention::Consuming,
            }],
        });

        let main_fn = FunctionDef {
            entity: entity(1),
            name: "main".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(
                vec![call_inst],
                ret_val,
                vec![
                    ValueDef::owned(unit),
                    ValueDef::owned(i64_ty),
                    ValueDef::owned(i64_ty),
                ],
            )),
            extern_info: None,
        };

        module.add_function(main_fn);
        module.add_function(generic_fn);
        module.register_name(entity(1), "main");
        module.register_name(entity(2), "identity");

        let target = TargetConfig::host_64();
        let mono = monomorphize(module, &target).unwrap();

        // main + identity[Int64]
        assert_eq!(mono.functions.len(), 2);

        // The identity function should have concrete params
        let identity = mono
            .functions
            .iter()
            .find(|f| f.source == entity(2))
            .unwrap();
        assert_eq!(identity.params.len(), 1);
        assert_eq!(identity.params[0].ty, i64_ty);
        assert_eq!(identity.ret, i64_ty);

        // The call in main should be Resolved
        let main = mono
            .functions
            .iter()
            .find(|f| f.source == entity(1))
            .unwrap();
        let body = main.body.as_ref().unwrap();
        let call = &body.blocks[0].insts[0];
        match &call.kind {
            InstKind::Call { callee, .. } => {
                assert!(matches!(callee, Callee::Resolved(_)));
            },
            _ => panic!("expected call"),
        }
    }

    #[test]
    fn monomorphize_extern_function() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();
        let unit = module.ty_arena.unit();
        let ptr = module.ty_arena.pointer(unit);

        let func = FunctionDef {
            entity: entity(1),
            name: "malloc".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![ParamDef::new(
                "size",
                ValueId::new(0),
                i64_ty,
                ParamConvention::Consuming,
            )],
            ret: ptr,
            where_clause: None,
            body: None,
            extern_info: Some(crate::item::function::ExternInfo {
                calling_convention: crate::item::function::CallingConvention::C,
                symbol_name: "malloc".into(),
            }),
        };
        module.add_function(func);
        module.register_name(entity(1), "malloc");

        let target = TargetConfig::host_64();
        let mono = monomorphize(module, &target).unwrap();

        assert_eq!(mono.functions.len(), 1);
        assert!(mono.functions[0].body.is_none());
        assert!(mono.functions[0].extern_info.is_some());
    }
}
