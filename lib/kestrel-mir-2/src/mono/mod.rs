pub mod collect;
pub mod mangle;
pub mod types;
pub mod verify;
pub mod witness;

pub use types::{
    InstantiationKey, MonoEnum, MonoEnumCase, MonoField, MonoFunction, MonoModule, MonoParam,
    MonoStatic, MonoStruct,
};
pub use witness::MonoError;

use std::collections::HashMap;

use indexmap::IndexMap;
use kestrel_hecs::Entity;

use crate::body::MirBody;
use crate::immediate::ImmediateKind;
use crate::item::function::{FunctionDef, FunctionKind};
use crate::item::protocol::ProtocolDef;
use crate::item::struct_def::StructDef;
use crate::item::enum_def::EnumDef;
use crate::item::witness::WitnessDef;
use crate::item::{Layout, TargetConfig};
use crate::layout::{EnumLayout, StructLayout};
use crate::operand::Operand;
use crate::statement::{Callee, Rvalue, StatementKind};
use crate::substitute::{SubstMap, substitute};
use crate::terminator::TerminatorKind;
use crate::ty::{MirTy, TyArena};
use crate::{FunctionIdx, MirModule, MonoFuncId, TyId};

use self::collect::{CollectionResult, WitnessCache};

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

    // Build entity→index map once (shared by Phase 2 and Phase 3)
    let entity_to_func: HashMap<Entity, FunctionIdx> = functions
        .iter()
        .enumerate()
        .map(|(i, f)| (f.entity, FunctionIdx::new(i)))
        .collect();

    // Phase 2: Body monomorphization
    let mut mono_bodies: Vec<MonoBodyResult> = Vec::with_capacity(instantiations.len());

    for key in &instantiations {
        let result = monomorphize_body(
            &mut ty_arena,
            &functions,
            &protocols,
            &witnesses,
            &entity_names,
            &entity_to_func,
            key,
            &witness_cache,
        );
        mono_bodies.push(result);
    }

    // Phase 3a: Drop expansion — rewrite Drop/DropIf/SetDropFlag to Call/Branch/Assign.
    // This also discovers drop shim instantiations iteratively.
    let mut instantiations = instantiations;
    expand_drops(
        &mut ty_arena,
        &functions,
        &protocols,
        &witnesses,
        &entity_names,
        &entity_to_func,
        &mut instantiations,
        &mut mono_bodies,
    );

    // Phase 3b: ID assignment + callee rewriting
    let func_id_map: HashMap<InstantiationKey, MonoFuncId> = instantiations
        .iter()
        .enumerate()
        .map(|(i, key)| (key.clone(), MonoFuncId::new(i)))
        .collect();

    for body_result in &mut mono_bodies {
        rewrite_callees(body_result, &func_id_map);
    }

    // Phase 4: Type and layout resolution
    let (mono_structs, mono_enums) = resolve_types_and_layouts(
        &mut ty_arena,
        &structs,
        &enums,
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

        let mangled_name = mangle::mangle_function(
            &mono_module.ty_arena,
            &entity_names,
            func_name,
            &key.type_args,
            key.self_type,
            &body_result.params,
            body_result.ret,
            receiver,
        );

        mono_module.add_function(MonoFunction {
            name: mangled_name,
            source: key.func_entity,
            type_args: key.type_args.clone(),
            self_type: key.self_type,
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
    body: Option<MirBody>,
    params: Vec<MonoParam>,
    ret: TyId,
    extern_info: Option<crate::item::function::ExternInfo>,
    /// Resolved witness callees: (block_idx, stmt_idx) -> target key
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
    _witness_cache: &WitnessCache,
) -> MonoBodyResult {
    let func_idx = entity_to_func
        .get(&key.func_entity)
        .expect("instantiation key must reference a valid function");
    let func = &functions[func_idx.index()];

    // Build SubstMap
    let mut subst = SubstMap::new();
    for (tp, &arg) in func.type_params.iter().zip(key.type_args.iter()) {
        subst.type_params.insert(tp.entity, arg);
    }
    // Protocol default methods have Self as TypeParam(protocol_entity).
    if let Some(st) = key.self_type {
        if let Some(first_param) = func.params.first() {
            if let MirTy::TypeParam(entity) = arena.get(first_param.ty) {
                if !subst.type_params.contains_key(entity) {
                    subst.type_params.insert(*entity, st);
                }
            }
        }
    }
    // Pre-resolve associated types via witness cache
    if let Some(where_clause) = &func.where_clause {
        for constraint in &where_clause.constraints {
            if let crate::item::function::WhereConstraint::Implements {
                type_param,
                protocol,
                protocol_type_args,
            } = constraint
            {
                let concrete_type = subst
                    .type_params
                    .get(type_param)
                    .copied();

                if let Some(concrete_ty) = concrete_type {
                    // Enrich subst with witness proto_type_args → concrete mappings.
                    // For `I: SeqIndex[T]` where T→concrete_T, the witness for
                    // `Int64: SeqIndex[T_ext]` has proto_type_args = [TypeParam(T_ext)].
                    // We build: T_ext → concrete_T by chaining:
                    //   where_clause_arg[i] → subst → concrete
                    //   witness.proto_type_args[i] → TypeParam(T_ext)
                    for witness in witnesses.iter() {
                        if witness.protocol != *protocol {
                            continue;
                        }
                        let mut bindings = HashMap::new();
                        if !witness::match_pattern(arena, witness.implementing_type, concrete_ty, &mut bindings) {
                            continue;
                        }
                        // Chain: where clause type arg entities → subst → concrete,
                        // then map witness proto_type_args TypeParams to those concrete values.
                        for (pi, &wc_arg_entity) in protocol_type_args.iter().enumerate() {
                            if let Some(&proto_expr) = witness.proto_type_args.get(pi) {
                                if let MirTy::TypeParam(ext_entity) = arena.get(proto_expr) {
                                    if !subst.type_params.contains_key(ext_entity) {
                                        if let Some(&cv) = subst.type_params.get(&wc_arg_entity) {
                                            subst.type_params.insert(*ext_entity, cv);
                                        }
                                    }
                                }
                            }
                        }
                        // Also add pattern match bindings
                        for (entity, ty) in &bindings {
                            subst.type_params.entry(*entity).or_insert(*ty);
                        }
                        break;
                    }

                    // Find associated types for this protocol
                    for proto_def in protocols {
                        if proto_def.entity == *protocol {
                            for assoc in &proto_def.associated_types {
                                if let Some(bound_ty) = witness::resolve_associated_type(
                                    arena,
                                    witnesses,
                                    *protocol,
                                    concrete_ty,
                                    assoc.entity,
                                ) {
                                    // Substitute the binding through our SubstMap
                                    let resolved = substitute(arena, bound_ty, &subst);
                                    subst.assoc_types.insert(
                                        (concrete_ty, *protocol, assoc.entity),
                                        resolved,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Substitute param types
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

    // Substitute local types
    for local in &mut mono_body.locals {
        local.ty = substitute(arena, local.ty, &subst);
    }

    // Walk blocks: substitute types in statements and resolve witnesses
    for (bi, block) in mono_body.blocks.iter_mut().enumerate() {
        for (si, stmt) in block.stmts.iter_mut().enumerate() {
            match &mut stmt.kind {
                StatementKind::Assign { rvalue, .. } => {
                    substitute_rvalue(arena, rvalue, &subst);
                }
                StatementKind::Call { callee, args, .. } => {
                    substitute_callee_and_resolve(
                        arena,
                        witnesses,
                        protocols,
                        functions,
                        entity_names,
                        callee,
                        &subst,
                        bi,
                        si,
                        &mut resolved_witnesses,
                    );
                    // Substitute types in args (function ref immediates)
                    for (op, _) in args.iter_mut() {
                        substitute_operand(arena, op, &subst);
                    }
                }
                _ => {}
            }
        }
        // Substitute terminator operands
        substitute_terminator(arena, &mut block.terminator, &subst);
    }

    MonoBodyResult {
        body: Some(mono_body),
        params,
        ret,
        extern_info,
        resolved_witnesses,
    }
}

fn substitute_rvalue(arena: &mut TyArena, rvalue: &mut Rvalue, subst: &SubstMap) {
    match rvalue {
        Rvalue::Construct { ty, fields, .. } => {
            *ty = substitute(arena, *ty, subst);
            for (_, op, _) in fields.iter_mut() {
                substitute_operand(arena, op, subst);
            }
        }
        Rvalue::EnumVariant { enum_ty, payload, .. } => {
            *enum_ty = substitute(arena, *enum_ty, subst);
            for (op, _) in payload.iter_mut() {
                substitute_operand(arena, op, subst);
            }
        }
        Rvalue::ArrayLiteral { element_ty, values, .. } => {
            *element_ty = substitute(arena, *element_ty, subst);
            for (op, _) in values.iter_mut() {
                substitute_operand(arena, op, subst);
            }
        }
        Rvalue::Use(op, _) => substitute_operand(arena, op, subst),
        Rvalue::Tuple(elems) => {
            for (op, _) in elems.iter_mut() {
                substitute_operand(arena, op, subst);
            }
        }
        Rvalue::ApplyPartial { captures, .. } => {
            for (op, _) in captures.iter_mut() {
                substitute_operand(arena, op, subst);
            }
        }
        Rvalue::Op1 { arg, op } => {
            substitute_op_type(arena, op, subst);
            substitute_operand(arena, arg, subst);
        }
        Rvalue::Op2 { lhs, rhs, op } => {
            substitute_op_type(arena, op, subst);
            substitute_operand(arena, lhs, subst);
            substitute_operand(arena, rhs, subst);
        }
        Rvalue::Op3 { a, b, c, op } => {
            substitute_op_type(arena, op, subst);
            substitute_operand(arena, a, subst);
            substitute_operand(arena, b, subst);
            substitute_operand(arena, c, subst);
        }
        Rvalue::Ref(_) | Rvalue::RefMut(_) => {}
    }
}

fn substitute_operand(arena: &mut TyArena, operand: &mut Operand, subst: &SubstMap) {
    if let Operand::Const(imm) = operand {
        substitute_immediate(arena, &mut imm.kind, subst);
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
        | Op::PtrCast(ty)
        | Op::PtrBitcast(ty)
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
    block_idx: usize,
    stmt_idx: usize,
    resolved_witnesses: &mut HashMap<(usize, usize), InstantiationKey>,
) {
    match callee {
        Callee::Direct {
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
        Callee::Witness {
            protocol,
            method,
            self_type,
            method_type_args,
        } => {
            *self_type = substitute(arena, *self_type, subst);
            for ta in method_type_args.iter_mut() {
                *ta = substitute(arena, *ta, subst);
            }
            // Resolve witness to concrete function
            if let Ok(resolved) = witness::resolve_witness_call(
                arena,
                witnesses,
                protocols,
                functions,
                entity_names,
                *protocol,
                method,
                *self_type,
                method_type_args,
            ) {
                resolved_witnesses.insert(
                    (block_idx, stmt_idx),
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

fn substitute_terminator(
    arena: &mut TyArena,
    terminator: &mut crate::terminator::Terminator,
    subst: &SubstMap,
) {
    match &mut terminator.kind {
        TerminatorKind::Return(op) => substitute_operand(arena, op, subst),
        TerminatorKind::Branch { condition, .. } => substitute_operand(arena, condition, subst),
        _ => {}
    }
}

// -- Phase 3a: Drop expansion --

fn expand_drops(
    arena: &mut TyArena,
    functions: &[FunctionDef],
    protocols: &[ProtocolDef],
    witnesses: &[WitnessDef],
    entity_names: &IndexMap<Entity, String>,
    entity_to_func: &HashMap<Entity, FunctionIdx>,
    instantiations: &mut indexmap::IndexSet<InstantiationKey>,
    mono_bodies: &mut Vec<MonoBodyResult>,
) {
    // Iterative: expand drops, discover new shim instantiations, monomorphize them, repeat
    loop {
        let mut new_shim_keys: Vec<InstantiationKey> = Vec::new();

        for body_result in mono_bodies.iter_mut() {
            let Some(body) = &mut body_result.body else {
                continue;
            };
            expand_drops_in_body(body, arena, functions, &mut new_shim_keys);
        }

        // Deduplicate and find truly new shim instantiations
        let mut added_any = false;
        for key in new_shim_keys {
            if instantiations.insert(key.clone()) {
                // Monomorphize the new shim body
                let dummy_cache = WitnessCache { resolved: HashMap::new() };
                let result = monomorphize_body(
                    arena,
                    functions,
                    protocols,
                    witnesses,
                    entity_names,
                    entity_to_func,
                    &key,
                    &dummy_cache,
                );
                mono_bodies.push(result);
                added_any = true;
            }
        }

        if !added_any {
            break;
        }
    }
}

fn expand_drops_in_body(
    body: &mut MirBody,
    arena: &TyArena,
    functions: &[FunctionDef],
    new_shim_keys: &mut Vec<InstantiationKey>,
) {
    // Collect expansions first to avoid mutating while iterating
    struct DropExpansion {
        block_idx: usize,
        stmt_idx: usize,
    }
    struct DropIfExpansion {
        block_idx: usize,
        stmt_idx: usize,
    }

    let mut drops = Vec::new();
    let mut drop_ifs = Vec::new();
    let mut set_flags = Vec::new();

    for (bi, block) in body.blocks.iter().enumerate() {
        for (si, stmt) in block.stmts.iter().enumerate() {
            match &stmt.kind {
                StatementKind::Drop { .. } => drops.push(DropExpansion {
                    block_idx: bi,
                    stmt_idx: si,
                }),
                StatementKind::DropIf { .. } => drop_ifs.push(DropIfExpansion {
                    block_idx: bi,
                    stmt_idx: si,
                }),
                StatementKind::SetDropFlag { .. } => set_flags.push((bi, si)),
                _ => {}
            }
        }
    }

    // Expand SetDropFlag → Assign (simple, no new blocks)
    for &(bi, si) in &set_flags {
        let stmt = &body.blocks[bi].stmts[si];
        if let StatementKind::SetDropFlag { flag, value } = &stmt.kind {
            let flag = *flag;
            let value = *value;
            body.blocks[bi].stmts[si] = crate::statement::Statement {
                kind: StatementKind::Assign {
                    dest: crate::place::Place::local(flag),
                    rvalue: Rvalue::Use(
                        Operand::Const(crate::immediate::Immediate::bool(value)),
                        crate::operand::UseMode::Copy,
                    ),
                },
                span: body.blocks[bi].stmts[si].span.clone(),
            };
        }
    }

    // Expand Drop → Call (in-place replacement, no new blocks)
    for exp in &drops {
        let stmt = &body.blocks[exp.block_idx].stmts[exp.stmt_idx];
        if let StatementKind::Drop { place } = &stmt.kind {
            let place = place.clone();
            let local_ty = body.locals[place.root_local().unwrap_or(crate::LocalId::new(0)).index()].ty;
            if let Some((shim_entity, type_args)) = find_drop_shim_for_type(arena, local_ty, functions) {
                let callee = Callee::Direct {
                    func: shim_entity,
                    type_args: type_args.clone(),
                    self_type: None,
                };
                let span = body.blocks[exp.block_idx].stmts[exp.stmt_idx].span.clone();
                body.blocks[exp.block_idx].stmts[exp.stmt_idx] = crate::statement::Statement {
                    kind: StatementKind::Call {
                        dest: None,
                        callee,
                        args: vec![(Operand::Place(place), crate::operand::ArgMode::Move)],
                    },
                    span,
                };
                new_shim_keys.push(InstantiationKey::new(shim_entity, type_args, None));
            }
        }
    }

    // Expand DropIf → Branch + Call + Jump (adds new blocks)
    // Process in reverse order so block indices stay valid for earlier expansions
    for exp in drop_ifs.iter().rev() {
        let stmt = &body.blocks[exp.block_idx].stmts[exp.stmt_idx];
        if let StatementKind::DropIf { place, flag } = &stmt.kind {
            let place = place.clone();
            let flag = *flag;
            let local_ty = body.locals[place.root_local().unwrap_or(crate::LocalId::new(0)).index()].ty;

            if let Some((shim_entity, type_args)) = find_drop_shim_for_type(arena, local_ty, functions) {
                let span = body.blocks[exp.block_idx].stmts[exp.stmt_idx].span.clone();

                // Create new block indices (allocated after all existing blocks)
                let continue_block = crate::BlockId::new(body.blocks.len());
                let drop_block = crate::BlockId::new(body.blocks.len() + 1);
                let skip_block = crate::BlockId::new(body.blocks.len() + 2);

                // Split: replace the DropIf with a Branch terminator.
                // Move everything after the DropIf to the continue block.
                let remaining_stmts = body.blocks[exp.block_idx]
                    .stmts
                    .split_off(exp.stmt_idx + 1);
                let old_terminator = std::mem::replace(
                    &mut body.blocks[exp.block_idx].terminator,
                    crate::terminator::Terminator {
                        kind: TerminatorKind::Branch {
                            condition: Operand::Place(crate::place::Place::local(flag)),
                            then_block: drop_block,
                            else_block: skip_block,
                        },
                        span: span.clone(),
                    },
                );
                // Remove the DropIf statement itself
                body.blocks[exp.block_idx].stmts.pop();

                // Continue block: remaining statements + original terminator
                body.blocks.push(crate::body::BasicBlock {
                    stmts: remaining_stmts,
                    terminator: old_terminator,
                });

                // Drop block: call shim, jump to continue
                body.blocks.push(crate::body::BasicBlock {
                    stmts: vec![crate::statement::Statement {
                        kind: StatementKind::Call {
                            dest: None,
                            callee: Callee::Direct {
                                func: shim_entity,
                                type_args: type_args.clone(),
                                self_type: None,
                            },
                            args: vec![(Operand::Place(place), crate::operand::ArgMode::Move)],
                        },
                        span,
                    }],
                    terminator: crate::terminator::Terminator {
                        kind: TerminatorKind::Jump(continue_block),
                        span: None,
                    },
                });

                // Skip block: jump to continue
                body.blocks.push(crate::body::BasicBlock {
                    stmts: vec![],
                    terminator: crate::terminator::Terminator {
                        kind: TerminatorKind::Jump(continue_block),
                        span: None,
                    },
                });

                new_shim_keys.push(InstantiationKey::new(shim_entity, type_args, None));
            }
        }
    }
}

/// Find the drop shim entity for a concrete type.
/// Returns (shim_entity, type_args for instantiation).
fn find_drop_shim_for_type(
    arena: &TyArena,
    ty: TyId,
    functions: &[FunctionDef],
) -> Option<(Entity, Vec<TyId>)> {
    match arena.get(ty) {
        MirTy::Named { entity, type_args } => {
            let entity = *entity;
            let type_args = type_args.clone();
            // Find the drop shim for this type entity
            let shim = functions.iter().find(|f| {
                matches!(f.kind, FunctionKind::DropShim { nominal } if nominal == entity)
            })?;
            Some((shim.entity, type_args))
        }
        _ => None,
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
        for (si, stmt) in block.stmts.iter_mut().enumerate() {
            match &mut stmt.kind {
                StatementKind::Call { callee, args, .. } => {
                    rewrite_callee(callee, bi, si, &body_result.resolved_witnesses, func_id_map);
                    for (op, _) in args.iter_mut() {
                        rewrite_operand(op, func_id_map);
                    }
                }
                StatementKind::Assign { rvalue, .. } => {
                    rewrite_rvalue(rvalue, func_id_map);
                }
                _ => {}
            }
        }
        rewrite_terminator_operands(&mut block.terminator, func_id_map);
    }
}

fn rewrite_callee(
    callee: &mut Callee,
    block_idx: usize,
    stmt_idx: usize,
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
            if let Some(target_key) = resolved_witnesses.get(&(block_idx, stmt_idx))
                && let Some(&mono_id) = func_id_map.get(target_key)
            {
                *callee = Callee::Resolved(mono_id);
            }
        }
        _ => {}
    }
}

fn rewrite_operand(
    operand: &mut Operand,
    func_id_map: &HashMap<InstantiationKey, MonoFuncId>,
) {
    if let Operand::Const(imm) = operand
        && let ImmediateKind::FunctionRef {
            func,
            type_args,
            self_type,
        } = &imm.kind
    {
        let key = InstantiationKey::new(*func, type_args.clone(), *self_type);
        if let Some(&mono_id) = func_id_map.get(&key) {
            imm.kind = ImmediateKind::MonoFunctionRef(mono_id);
        }
    }
}

fn rewrite_rvalue(
    rvalue: &mut Rvalue,
    func_id_map: &HashMap<InstantiationKey, MonoFuncId>,
) {
    match rvalue {
        Rvalue::Use(op, _) => rewrite_operand(op, func_id_map),
        Rvalue::Construct { fields, .. } => {
            for (_, op, _) in fields.iter_mut() {
                rewrite_operand(op, func_id_map);
            }
        }
        Rvalue::Tuple(elems) => {
            for (op, _) in elems.iter_mut() {
                rewrite_operand(op, func_id_map);
            }
        }
        Rvalue::EnumVariant { payload, .. } => {
            for (op, _) in payload.iter_mut() {
                rewrite_operand(op, func_id_map);
            }
        }
        Rvalue::ArrayLiteral { values, .. } => {
            for (op, _) in values.iter_mut() {
                rewrite_operand(op, func_id_map);
            }
        }
        Rvalue::ApplyPartial { captures, .. } => {
            for (op, _) in captures.iter_mut() {
                rewrite_operand(op, func_id_map);
            }
        }
        Rvalue::Op1 { arg, .. } => rewrite_operand(arg, func_id_map),
        Rvalue::Op2 { lhs, rhs, .. } => {
            rewrite_operand(lhs, func_id_map);
            rewrite_operand(rhs, func_id_map);
        }
        Rvalue::Op3 { a, b, c, .. } => {
            rewrite_operand(a, func_id_map);
            rewrite_operand(b, func_id_map);
            rewrite_operand(c, func_id_map);
        }
        Rvalue::Ref(_) | Rvalue::RefMut(_) => {}
    }
}

fn rewrite_terminator_operands(
    terminator: &mut crate::terminator::Terminator,
    func_id_map: &HashMap<InstantiationKey, MonoFuncId>,
) {
    match &mut terminator.kind {
        TerminatorKind::Return(op) => rewrite_operand(op, func_id_map),
        TerminatorKind::Branch { condition, .. } => rewrite_operand(condition, func_id_map),
        _ => {}
    }
}

// -- Phase 4: Type and layout resolution --

fn resolve_types_and_layouts(
    arena: &mut TyArena,
    structs: &[StructDef],
    enums: &[EnumDef],
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
                        let concrete_ty = substitute(arena, field.ty, &subst);
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
                            let concrete_ty = substitute(arena, field.ty, &subst);
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

fn collect_named_types(
    arena: &TyArena,
    body: &MirBody,
    out: &mut IndexMap<(Entity, Vec<TyId>), ConcreteTypeKind>,
    structs: &[StructDef],
    enums: &[EnumDef],
) {
    // Walk local types
    for local in &body.locals {
        collect_named_type_from_ty(arena, local.ty, out, structs, enums);
    }
    // Walk statement types (Construct.ty, EnumVariant.enum_ty, ArrayLiteral.element_ty, etc.)
    for block in &body.blocks {
        for stmt in &block.stmts {
            match &stmt.kind {
                StatementKind::Assign { rvalue, .. } => {
                    collect_named_types_from_rvalue(arena, rvalue, out, structs, enums);
                }
                StatementKind::Call { args, .. } => {
                    for (op, _) in args {
                        collect_named_types_from_operand(arena, op, out, structs, enums);
                    }
                }
                _ => {}
            }
        }
    }
}

fn collect_named_types_from_rvalue(
    arena: &TyArena,
    rvalue: &Rvalue,
    out: &mut IndexMap<(Entity, Vec<TyId>), ConcreteTypeKind>,
    structs: &[StructDef],
    enums: &[EnumDef],
) {
    match rvalue {
        Rvalue::Construct { ty, .. } => collect_named_type_from_ty(arena, *ty, out, structs, enums),
        Rvalue::EnumVariant { enum_ty, .. } => collect_named_type_from_ty(arena, *enum_ty, out, structs, enums),
        Rvalue::ArrayLiteral { element_ty, .. } => collect_named_type_from_ty(arena, *element_ty, out, structs, enums),
        _ => {
            // Walk operands for function refs that might contain Named types
            for op in rvalue.operands() {
                collect_named_types_from_operand(arena, op, out, structs, enums);
            }
        }
    }
}

fn collect_named_types_from_operand(
    arena: &TyArena,
    op: &Operand,
    out: &mut IndexMap<(Entity, Vec<TyId>), ConcreteTypeKind>,
    structs: &[StructDef],
    enums: &[EnumDef],
) {
    if let Operand::Const(imm) = op {
        match &imm.kind {
            ImmediateKind::SizeOf(ty) | ImmediateKind::AlignOf(ty) | ImmediateKind::NullPtr(ty) => {
                collect_named_type_from_ty(arena, *ty, out, structs, enums);
            }
            _ => {}
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
    use crate::body::{BasicBlock, LocalDef, MirBody};
    use crate::item::function::{FunctionDef, FunctionKind, ParamDef};
    use crate::item::TypeParamDef;
    use crate::operand::ArgMode;
    use crate::place::Place;
    use crate::statement::Statement;
    use crate::terminator::{Terminator, TerminatorKind};
    use crate::ty::ParamConvention;
    use crate::{BlockId, LocalId};

    fn entity(id: u32) -> Entity {
        Entity::from_raw(id)
    }

    fn make_body(stmts: Vec<Statement>, ret_local: LocalId, locals: Vec<LocalDef>) -> MirBody {
        let block = BasicBlock {
            stmts,
            terminator: Terminator {
                kind: TerminatorKind::Return(Operand::Place(Place::local(ret_local))),
                span: None,
            },
        };
        MirBody {
            locals,
            blocks: vec![block],
            param_count: 0,
            entry: BlockId::new(0),
            local_scopes: HashMap::new(),
            failure_return_blocks: std::collections::HashSet::new(),
        }
    }

    #[test]
    fn monomorphize_concrete_function() {
        let mut module = MirModule::new("test");
        let unit = module.ty_arena.unit();

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
                LocalId::new(0),
                vec![LocalDef {
                    name: "_ret".into(),
                    ty: unit,
                }],
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
        let i64 = module.ty_arena.i64();
        let tp_ty = module.ty_arena.intern(MirTy::TypeParam(entity(3)));

        // generic_fn[T] -> ()
        let generic_fn = FunctionDef {
            entity: entity(2),
            name: "identity".into(),
            kind: FunctionKind::Free,
            type_params: vec![TypeParamDef::new(entity(3), "T")],
            params: vec![ParamDef {
                name: "x".into(),
                local: LocalId::new(0),
                ty: tp_ty,
                convention: ParamConvention::Consuming,
                external_label: None,
            }],
            ret: tp_ty,
            where_clause: None,
            body: Some(make_body(
                vec![],
                LocalId::new(0),
                vec![LocalDef {
                    name: "x".into(),
                    ty: tp_ty,
                }],
            )),
            extern_info: None,
        };

        // main() calls identity[Int64]
        let call_stmt = Statement {
            kind: StatementKind::Call {
                dest: Some(Place::local(LocalId::new(1))),
                callee: Callee::Direct {
                    func: entity(2),
                    type_args: vec![i64],
                    self_type: None,
                },
                args: vec![(Operand::Place(Place::local(LocalId::new(1))), ArgMode::Copy)],
            },
            span: None,
        };

        let main_fn = FunctionDef {
            entity: entity(1),
            name: "main".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(
                vec![call_stmt],
                LocalId::new(0),
                vec![
                    LocalDef { name: "_ret".into(), ty: unit },
                    LocalDef { name: "x".into(), ty: i64 },
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
        assert_eq!(identity.params[0].ty, i64);
        assert_eq!(identity.ret, i64);

        // The call in main should be Resolved
        let main = mono.functions.iter().find(|f| f.source == entity(1)).unwrap();
        let body = main.body.as_ref().unwrap();
        let call = &body.blocks[0].stmts[0];
        match &call.kind {
            StatementKind::Call { callee, .. } => {
                assert!(matches!(callee, Callee::Resolved(_)));
            }
            _ => panic!("expected call"),
        }
    }

    #[test]
    fn monomorphize_extern_function() {
        let mut module = MirModule::new("test");
        let i64 = module.ty_arena.i64();
        let unit = module.ty_arena.unit();
        let ptr = module.ty_arena.pointer(unit);

        let func = FunctionDef {
            entity: entity(1),
            name: "malloc".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![ParamDef {
                name: "size".into(),
                local: LocalId::new(0),
                ty: i64,
                convention: ParamConvention::Consuming,
                external_label: None,
            }],
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
