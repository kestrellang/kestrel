pub mod collect;
pub mod expand;
pub mod mangle;
pub mod types;
pub mod verify;
pub mod witness;

pub use collect::CollectionResult;
pub use types::{
    InstantiationKey, MonoEnum, MonoEnumCase, MonoField, MonoFunction, MonoModule, MonoParam,
    MonoStatic, MonoStruct,
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
use crate::item::function::{FunctionDef, FunctionKind};
use crate::item::protocol::ProtocolDef;
use crate::item::struct_def::StructDef;
use crate::item::enum_def::EnumDef;
use crate::item::witness::WitnessDef;
use crate::item::{Layout, TargetConfig};
use crate::layout::{EnumLayout, StructLayout};
use crate::substitute::{SubstMap, substitute};
use crate::ty::{MirTy, TyArena};
use crate::value::Ownership;
use crate::{FunctionIdx, MirModule, MonoFuncId, TyId};


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

    // Build entity->index map once (shared by Phase 2 and Phase 3)
    let entity_to_func: HashMap<Entity, FunctionIdx> = functions
        .iter()
        .enumerate()
        .map(|(i, f)| (f.entity, FunctionIdx::new(i)))
        .collect();

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
            &entity_to_func,
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
        rewrite_callees(body_result, &func_id_map);
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
    let mut mono_module = MonoModule::new(ty_arena, entity_names.clone());

    for (i, key) in instantiations.iter().enumerate() {
        let body_result = &mono_bodies[i];
        let func_name = entity_names
            .get(&key.func_entity)
            .map(|s| s.as_str())
            .unwrap_or("<unknown>");

        // Determine receiver convention for mangling
        let func_idx = functions
            .iter()
            .position(|f| f.entity == key.func_entity);
        let receiver = func_idx.and_then(|fi| match &functions[fi].kind {
            FunctionKind::Method { receiver, .. } => Some(*receiver),
            _ => None,
        });

        // Safety net: resolve any residual projections in key type_args/self_type.
        // Phase 1 should produce fully-resolved keys, but deep_resolve catches
        // edge cases where substitute() couldn't resolve nested projections.
        let resolved_type_args: Vec<TyId> = key.type_args
            .iter()
            .map(|&ta| collect::substitute_and_resolve(&mut mono_module.ty_arena, &witnesses, ta, &SubstMap::new()))
            .collect();
        let resolved_self = key.self_type
            .map(|st| collect::substitute_and_resolve(&mut mono_module.ty_arena, &witnesses, st, &SubstMap::new()));

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

    mono_module.structs = mono_structs;
    mono_module.enums = mono_enums;

    // Copy statics
    for s in &statics {
        mono_module.statics.push(MonoStatic::from_static_def(s));
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
    functions: &[FunctionDef],
    protocols: &[ProtocolDef],
    witnesses: &[WitnessDef],
    entity_names: &IndexMap<Entity, String>,
    entity_to_func: &HashMap<Entity, FunctionIdx>,
    key: &InstantiationKey,
) -> MonoBodyResult {
    let func_idx = entity_to_func
        .get(&key.func_entity)
        .expect("instantiation key must reference a valid function");
    let func = &functions[func_idx.index()];

    let subst = collect::build_subst(func, &key.type_args, key.self_type, arena, protocols, witnesses);

    // Substitute param and return types
    let params: Vec<MonoParam> = func
        .params
        .iter()
        .map(|p| MonoParam::with_label(&p.name, substitute(arena, p.ty, &subst), p.convention, p.external_label.clone()))
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
        // No terminator substitution needed — MIR-3 terminators carry ValueId only
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
    let params: Vec<MonoParam> = params.into_iter().map(|mut p| { p.ty = resolve(arena, p.ty); p }).collect();
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

/// Substitute types in a single instruction. Replaces the MIR-2 helpers
/// `substitute_rvalue`, `substitute_operand`, and `substitute_terminator`.
fn substitute_inst(
    arena: &mut TyArena,
    witnesses: &[WitnessDef],
    protocols: &[ProtocolDef],
    functions: &[FunctionDef],
    entity_names: &IndexMap<Entity, String>,
    kind: &mut InstKind,
    subst: &SubstMap,
    parent_self: Option<TyId>,
    block_idx: usize,
    inst_idx: usize,
    resolved_witnesses: &mut HashMap<(usize, usize), InstantiationKey>,
) {
    match kind {
        // Memory access instructions with embedded type
        InstKind::CopyAddr { ty, .. }
        | InstKind::Take { ty, .. }
        | InstKind::BeginBorrowAddr { ty, .. }
        | InstKind::BeginMutBorrowAddr { ty, .. }
        | InstKind::DestroyAddr { ty, .. }
        | InstKind::FieldAddr { ty, .. }
        | InstKind::Uninit { ty, .. } => {
            *ty = substitute(arena, *ty, subst);
        }

        // Aggregate construction
        InstKind::Struct { ty, .. } => {
            *ty = substitute(arena, *ty, subst);
        }
        InstKind::Enum { enum_ty, .. } => {
            *enum_ty = substitute(arena, *enum_ty, subst);
        }
        InstKind::Array { element_ty, .. } => {
            *element_ty = substitute(arena, *element_ty, subst);
        }

        // Ops with embedded type
        InstKind::Op1 { op, .. } => {
            substitute_op_type(arena, op, subst);
        }
        InstKind::Op2 { op, .. } => {
            substitute_op_type(arena, op, subst);
        }
        InstKind::Op3 { op, .. } => {
            substitute_op_type(arena, op, subst);
        }

        // Constants
        InstKind::Literal { value, .. } => {
            substitute_immediate(arena, &mut value.kind, subst);
        }

        // Calls
        InstKind::Call { callee, .. } => {
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
        }

        // All other InstKinds carry only ValueId — no substitution needed
        _ => {}
    }
}

fn substitute_immediate(arena: &mut TyArena, kind: &mut ImmediateKind, subst: &SubstMap) {
    match kind {
        ImmediateKind::SizeOf(ty) | ImmediateKind::AlignOf(ty) | ImmediateKind::NullPtr(ty) => {
            *ty = substitute(arena, *ty, subst);
        }
        ImmediateKind::FunctionRef {
            type_args,
            self_type,
            ..
        } => {
            for ta in type_args.iter_mut() {
                *ta = substitute(arena, *ta, subst);
            }
            if let Some(st) = self_type {
                *st = substitute(arena, *st, subst);
            }
        }
        _ => {}
    }
}

fn substitute_op_type(arena: &mut TyArena, op: &mut crate::op::Op, subst: &SubstMap) {
    use crate::op::Op;
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
            *ty = substitute(arena, *ty, subst);
        }
        _ => {}
    }
}

fn substitute_callee_and_resolve(
    arena: &mut TyArena,
    witnesses: &[WitnessDef],
    protocols: &[ProtocolDef],
    functions: &[FunctionDef],
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
                if let Some(f) = functions.iter().find(|f| f.entity == *func) {
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
        }
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
        }
        _ => {}
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
            if let InstKind::Call { callee: callee @ Callee::Witness { .. }, .. } = &mut inst.kind {
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
) {
    let Some(body) = &mut body_result.body else {
        return;
    };

    for (bi, block) in body.blocks.iter_mut().enumerate() {
        for (ii, inst) in block.insts.iter_mut().enumerate() {
            match &mut inst.kind {
                InstKind::Call { callee, .. } => {
                    rewrite_callee(callee, bi, ii, &body_result.resolved_witnesses, func_id_map);
                }
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
                }
                _ => {}
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
) {
    match callee {
        Callee::Direct {
            func,
            type_args,
            self_type,
        } => {
            let key = InstantiationKey::new(
                *func,
                type_args.clone(),
                *self_type,
            );
            if let Some(&mono_id) = func_id_map.get(&key) {
                *callee = Callee::Resolved(mono_id);
            }
        }
        Callee::Witness { .. } => {
            if let Some(target_key) = resolved_witnesses.get(&(block_idx, inst_idx))
                && let Some(&mono_id) = func_id_map.get(target_key)
            {
                *callee = Callee::Resolved(mono_id);
            }
        }
        _ => {}
    }
}

// -- Phase 4: Type and layout resolution --

fn resolve_types_and_layouts(
    arena: &mut TyArena,
    structs: &[StructDef],
    enums: &[EnumDef],
    witnesses: &[WitnessDef],
    mono_bodies: &[MonoBodyResult],
    target: &TargetConfig,
) -> (Vec<MonoStruct>, Vec<MonoEnum>) {
    // Collect all concrete Named types from monomorphized bodies
    let mut concrete_types: IndexMap<(Entity, Vec<TyId>), ConcreteTypeKind> = IndexMap::new();

    for body_result in mono_bodies {
        if let Some(body) = &body_result.body {
            collect_named_types(arena, body, &mut concrete_types, structs, enums);
        }
    }

    // Compute layouts for concrete types (fixed-point loop)
    let mut mono_structs = Vec::new();
    let mut mono_enums = Vec::new();
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
                ConcreteTypeKind::Struct(struct_idx) => {
                    let sdef = &structs[*struct_idx];
                    let subst = build_type_subst(sdef.type_params.iter().map(|tp| tp.entity), type_args);

                    let mut layout = StructLayout::new();
                    let mut all_resolved = true;
                    let mut fields = Vec::new();

                    for field in &sdef.fields {
                        let concrete_ty = collect::substitute_and_resolve(arena, witnesses, field.ty, &subst);
                        fields.push(MonoField::new(&field.name, concrete_ty));

                        if let Some((size, align)) = mono_size_and_align(arena, concrete_ty, target, &layout_cache) {
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
                        mono_structs.push(ms);
                        progress = true;
                    }
                }
                ConcreteTypeKind::Enum(enum_idx) => {
                    let edef = &enums[*enum_idx];
                    let subst = build_type_subst(edef.type_params.iter().map(|tp| tp.entity), type_args);

                    let mut all_resolved = true;
                    let mut cases = Vec::new();
                    let mut variant_layouts = Vec::new();

                    for case in &edef.cases {
                        let mut case_layout = StructLayout::new();
                        let mut mono_fields = Vec::new();
                        for field in &case.payload_fields {
                            let concrete_ty = collect::substitute_and_resolve(arena, witnesses, field.ty, &subst);
                            mono_fields.push(MonoField::new(&field.name, concrete_ty));
                            if let Some((size, align)) = mono_size_and_align(arena, concrete_ty, target, &layout_cache) {
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
                        let mut me = MonoEnum::new(*entity, type_args.clone(), enum_layout.discriminant_width);
                        me.cases = cases;
                        me.type_info = edef.type_info.clone();
                        me.type_info.layout = Some(Layout::Enum(enum_layout.clone()));
                        me.payload_offset = enum_layout.payload_offset as u32;
                        mono_enums.push(me);
                        progress = true;
                    }
                }
            }
        }

        if !progress {
            break;
        }
    }

    (mono_structs, mono_enums)
}

enum ConcreteTypeKind {
    Struct(usize),
    Enum(usize),
}

/// Walk an OssaBody and collect all concrete Named types.
fn collect_named_types(
    arena: &TyArena,
    body: &OssaBody,
    out: &mut IndexMap<(Entity, Vec<TyId>), ConcreteTypeKind>,
    structs: &[StructDef],
    enums: &[EnumDef],
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
                }
                InstKind::Enum { enum_ty, .. } => {
                    collect_named_type_from_ty(arena, *enum_ty, out, structs, enums);
                }
                InstKind::Array { element_ty, .. } => {
                    collect_named_type_from_ty(arena, *element_ty, out, structs, enums);
                }
                InstKind::Literal { value, .. } => {
                    match &value.kind {
                        ImmediateKind::SizeOf(ty) | ImmediateKind::AlignOf(ty) | ImmediateKind::NullPtr(ty) => {
                            collect_named_type_from_ty(arena, *ty, out, structs, enums);
                        }
                        _ => {}
                    }
                }
                // CopyAddr/Take/BeginBorrowAddr/BeginMutBorrowAddr/DestroyAddr/FieldAddr/Uninit
                // carry ty but those are address types (Pointer), not Named
                _ => {}
            }
        }
    }
}

fn collect_named_type_from_ty(
    arena: &TyArena,
    ty: TyId,
    out: &mut IndexMap<(Entity, Vec<TyId>), ConcreteTypeKind>,
    structs: &[StructDef],
    enums: &[EnumDef],
) {
    match arena.get(ty) {
        MirTy::Named { entity, type_args } => {
            let entity = *entity;
            let type_args = type_args.clone();
            let key = (entity, type_args.clone());
            if !out.contains_key(&key) {
                if let Some(idx) = structs.iter().position(|s| s.entity == entity) {
                    out.insert(key, ConcreteTypeKind::Struct(idx));
                } else if let Some(idx) = enums.iter().position(|e| e.entity == entity) {
                    out.insert(key, ConcreteTypeKind::Enum(idx));
                }
            }
            // Recurse into type args
            for &arg in &type_args {
                collect_named_type_from_ty(arena, arg, out, structs, enums);
            }
        }
        MirTy::Pointer(inner) => {
            collect_named_type_from_ty(arena, *inner, out, structs, enums);
        }
        MirTy::Tuple(elems) => {
            for &elem in elems {
                collect_named_type_from_ty(arena, elem, out, structs, enums);
            }
        }
        _ => {}
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

fn discriminant_width(num_variants: usize) -> crate::op::IntBits {
    use crate::op::IntBits;
    if num_variants <= 256 {
        IntBits::I8
    } else if num_variants <= 65536 {
        IntBits::I16
    } else {
        IntBits::I32
    }
}

fn build_enum_layout(variant_layouts: &[StructLayout], num_variants: usize) -> EnumLayout {
    let disc_width = discriminant_width(num_variants);
    let disc_size = disc_width.byte_width();
    let disc_align = disc_size;

    let mut max_payload_size: u64 = 0;
    let mut max_payload_align: u64 = 1;
    for vl in variant_layouts {
        max_payload_size = max_payload_size.max(vl.size);
        max_payload_align = max_payload_align.max(vl.align);
    }

    let overall_align = disc_align.max(max_payload_align);
    let payload_offset = if max_payload_align == 0 || disc_size.is_multiple_of(max_payload_align) {
        disc_size
    } else {
        disc_size + (overall_align - disc_size % overall_align) % overall_align
    };
    let total_size = payload_offset + max_payload_size;
    let padding = if overall_align == 0 {
        0
    } else {
        (overall_align - total_size % overall_align) % overall_align
    };

    EnumLayout {
        size: total_size + padding,
        align: overall_align,
        discriminant_width: disc_width,
        payload_offset,
        variant_layouts: variant_layouts.to_vec(),
    }
}

/// Compute size and alignment for a concrete type, looking up mono layouts.
fn mono_size_and_align(
    arena: &TyArena,
    ty: TyId,
    target: &TargetConfig,
    layout_cache: &HashMap<(Entity, Vec<TyId>), (u64, u64)>,
) -> Option<(u64, u64)> {
    match arena.get(ty) {
        MirTy::Bool => Some((1, 1)),
        MirTy::I8 => Some((1, 1)),
        MirTy::I16 => Some((2, 2)),
        MirTy::I32 => Some((4, 4)),
        MirTy::I64 => Some((8, 8)),
        MirTy::F16 => Some((2, 2)),
        MirTy::F32 => Some((4, 4)),
        MirTy::F64 => Some((8, 8)),
        MirTy::Never => Some((0, 1)),
        MirTy::Str => Some((target.pointer_width * 2, target.pointer_width)),
        MirTy::Pointer(_) => Some((target.pointer_width, target.pointer_width)),
        MirTy::FuncThin { .. } => Some((target.pointer_width, target.pointer_width)),
        MirTy::FuncThick { .. } => Some((target.pointer_width * 2, target.pointer_width)),
        MirTy::Error => Some((0, 1)),

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
        }

        MirTy::Named { entity, type_args } => {
            let key = (*entity, type_args.clone());
            layout_cache.get(&key).copied()
        }

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
    use crate::item::function::{FunctionDef, FunctionKind, ParamDef};
    use crate::item::TypeParamDef;
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
            body: Some(make_body(
                vec![],
                ret_val,
                vec![ValueDef::owned(unit)],
            )),
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
            args: vec![CallArg { value: arg_val, convention: ParamConvention::Consuming }],
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
        let identity = mono.functions.iter().find(|f| f.source == entity(2)).unwrap();
        assert_eq!(identity.params.len(), 1);
        assert_eq!(identity.params[0].ty, i64_ty);
        assert_eq!(identity.ret, i64_ty);

        // The call in main should be Resolved
        let main = mono.functions.iter().find(|f| f.source == entity(1)).unwrap();
        let body = main.body.as_ref().unwrap();
        let call = &body.blocks[0].insts[0];
        match &call.kind {
            InstKind::Call { callee, .. } => {
                assert!(matches!(callee, Callee::Resolved(_)));
            }
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
            params: vec![ParamDef::new("size", ValueId::new(0), i64_ty, ParamConvention::Consuming)],
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
