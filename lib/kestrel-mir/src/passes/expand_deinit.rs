//! Expand `Deinit` and `DeinitIf` statements into actual destructor calls.
//!
//! Runs after the deinit pass (which inserts Deinit/DeinitIf nodes) and before
//! codegen. Rewrites each node into a Call to the type's deinit function, or
//! removes it if the type has no destructor.
//!
//! `DeinitIf` expands into a conditional branch: if the flag is false (value is
//! live), call deinit; otherwise skip.

use std::collections::{HashMap, HashSet};

use crate::MirModule;
use crate::body::{BasicBlock, MirBody};
use crate::id::{BlockId, LocalId};
use crate::item::FunctionKind;
use crate::place::Place;
use crate::statement::{CallArg, Callee, Statement, StatementKind};
use crate::terminator::{SwitchCase, Terminator, TerminatorKind};
use crate::ty::MirTy;
use crate::value::Value;
use kestrel_hecs::Entity;

/// Expand `Deinit` and `DeinitIf` statements into concrete destructor calls.
///
/// Phase 1: Expand all `Deinit` and `DeinitIf` on locals across all functions
/// (scope-exit cleanup inserted by the deinit pass before Return terminators).
///
/// Phase 2: Expand `DeinitIf` on self-fields in failure-return blocks
/// (partial-drop on init failure).
pub fn run_expand_deinit_pass(module: &mut MirModule) {
    let deinit_funcs: HashMap<Entity, Entity> = module
        .functions
        .iter()
        .filter_map(|f| match &f.kind {
            FunctionKind::Deinit { parent } => Some((*parent, f.entity)),
            _ => None,
        })
        .collect();

    // Phase 1: expand Deinit/DeinitIf statements into calls
    for func_idx in 0..module.functions.len() {
        let Some(body) = &module.functions[func_idx].body else {
            continue;
        };

        let expansions = collect_expansions(body, &deinit_funcs, module);
        if expansions.is_empty() {
            continue;
        }

        let body = module.functions[func_idx].body.as_mut().unwrap();
        apply_expansions(body, expansions);
    }

    // Phase 2: recursive field deinit — for each deinit function, append
    // deinit calls for fields that themselves have deinit functions.
    inject_field_deinits(module, &deinit_funcs);
}

/// What kind of expansion a statement needs.
enum Expansion {
    /// Replace Deinit with a Call (or remove if no deinit function).
    ReplaceDeinit {
        block: usize,
        stmt: usize,
        deinit_entity: Option<Entity>,
        place: Place,
        place_ty: MirTy,
    },
    /// Expand DeinitIf into a branch + conditional call.
    ExpandDeinitIf {
        block: usize,
        stmt: usize,
        deinit_entity: Option<Entity>,
        place: Place,
        place_ty: MirTy,
        flag: LocalId,
    },
    /// Expand Deinit/DeinitIf for an enum into a switch + per-variant deinit.
    ExpandEnumDrop {
        block: usize,
        stmt: usize,
        place: Place,
        place_ty: MirTy,
        /// (variant_name, field_path, deinit_func_entity, field_ty) for each droppable field.
        /// field_path is a list of field names to chain (e.g., ["0", "a"] for variant.0.a).
        variant_drops: Vec<(String, Vec<String>, Entity, MirTy)>,
        /// If Some, this is a DeinitIf with a flag
        flag: Option<LocalId>,
    },
    /// Expand Deinit/DeinitIf for a struct without explicit deinit but with
    /// droppable sub-fields into individual field-level deinit calls.
    ExpandStructFieldDrop {
        block: usize,
        stmt: usize,
        place: Place,
        /// (field_path, deinit_func_entity, field_ty) for each droppable sub-field.
        field_drops: Vec<(Vec<String>, Entity, MirTy)>,
        /// If Some, this is a DeinitIf with a flag
        flag: Option<LocalId>,
    },
}

fn collect_expansions(
    body: &MirBody,
    deinit_funcs: &HashMap<Entity, Entity>,
    module: &MirModule,
) -> Vec<Expansion> {
    let mut expansions = Vec::new();
    let failure_blocks: HashSet<usize> = body
        .failure_return_blocks
        .iter()
        .map(|b| b.index())
        .collect();

    for (block_idx, block) in body.blocks.iter().enumerate() {
        for (stmt_idx, stmt) in block.stmts.iter().enumerate() {
            match &stmt.kind {
                // Unconditional deinit on a local (inserted by deinit pass before returns)
                StatementKind::Deinit { place } => {
                    let place_ty = resolve_place_type(place, body, module);
                    let deinit_entity = place_ty
                        .as_ref()
                        .and_then(|ty| struct_entity(ty))
                        .and_then(|e| deinit_funcs.get(&e).copied());
                    if deinit_entity.is_none() {
                        // Check if this is an enum needing drop glue
                        if let Some(drops) = enum_variant_drops(place_ty.as_ref(), deinit_funcs, module) {
                            if !drops.is_empty() {
                                expansions.push(Expansion::ExpandEnumDrop {
                                    block: block_idx,
                                    stmt: stmt_idx,
                                    place: place.clone(),
                                    place_ty: place_ty.unwrap_or(MirTy::Error),
                                    variant_drops: drops,
                                    flag: None,
                                });
                                continue;
                            }
                        }
                        // Check for struct without deinit but with droppable sub-fields
                        let mut field_drops = Vec::new();
                        struct_field_drops(place_ty.as_ref(), deinit_funcs, module, &mut field_drops);
                        if !field_drops.is_empty() {
                            expansions.push(Expansion::ExpandStructFieldDrop {
                                block: block_idx,
                                stmt: stmt_idx,
                                place: place.clone(),
                                field_drops,
                                flag: None,
                            });
                            continue;
                        }
                    }
                    expansions.push(Expansion::ReplaceDeinit {
                        block: block_idx,
                        stmt: stmt_idx,
                        deinit_entity,
                        place: place.clone(),
                        place_ty: place_ty.unwrap_or(MirTy::Error),
                    });
                }
                // Conditional deinit — locals (from deinit pass) or self-fields
                // (from init partial-drop)
                StatementKind::DeinitIf { place, flag } => {
                    let is_self_field = matches!(
                        place,
                        Place::Field { parent, .. }
                            if parent.root_local() == Some(LocalId::new(0))
                    );
                    // Self-field DeinitIf only expanded in failure-return blocks
                    if is_self_field && !failure_blocks.contains(&block_idx) {
                        continue;
                    }
                    let place_ty = resolve_place_type(place, body, module);
                    let deinit_entity = place_ty
                        .as_ref()
                        .and_then(|ty| struct_entity(ty))
                        .and_then(|e| deinit_funcs.get(&e).copied());
                    // Check for enum/struct drop glue on non-self-field locals
                    if !is_self_field && deinit_entity.is_none() {
                        if let Some(drops) = enum_variant_drops(place_ty.as_ref(), deinit_funcs, module) {
                            if !drops.is_empty() {
                                expansions.push(Expansion::ExpandEnumDrop {
                                    block: block_idx,
                                    stmt: stmt_idx,
                                    place: place.clone(),
                                    place_ty: place_ty.unwrap_or(MirTy::Error),
                                    variant_drops: drops,
                                    flag: Some(*flag),
                                });
                                continue;
                            }
                        }
                        let mut field_drops = Vec::new();
                        struct_field_drops(place_ty.as_ref(), deinit_funcs, module, &mut field_drops);
                        if !field_drops.is_empty() {
                            expansions.push(Expansion::ExpandStructFieldDrop {
                                block: block_idx,
                                stmt: stmt_idx,
                                place: place.clone(),
                                field_drops,
                                flag: Some(*flag),
                            });
                            continue;
                        }
                    }
                    // For locals: always expand (remove if no deinit func)
                    // For self-fields: only expand if deinit func exists
                    if !is_self_field || deinit_entity.is_some() {
                        expansions.push(Expansion::ExpandDeinitIf {
                            block: block_idx,
                            stmt: stmt_idx,
                            deinit_entity,
                            place: place.clone(),
                            place_ty: place_ty.unwrap_or(MirTy::Error),
                            flag: *flag,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    expansions
}

fn apply_expansions(body: &mut MirBody, expansions: Vec<Expansion>) {
    // Process in reverse order so indices stay valid
    for expansion in expansions.into_iter().rev() {
        match expansion {
            Expansion::ReplaceDeinit {
                block,
                stmt,
                deinit_entity,
                place,
                place_ty,
                ..
            } => {
                if let Some(deinit_func) = deinit_entity {
                    // Replace with a Call to the deinit function
                    let callee = deinit_callee(deinit_func, place_ty);
                    body.blocks[block].stmts[stmt] = Statement::new(StatementKind::Call {
                        dest: None,
                        callee,
                        args: vec![CallArg::mutating(Value::Place(place))],
                    });
                } else {
                    // No deinit function — remove the statement
                    body.blocks[block].stmts.remove(stmt);
                }
            }
            Expansion::ExpandDeinitIf {
                block,
                stmt,
                deinit_entity,
                place,
                place_ty,
                flag,
            } => {
                // No deinit function → just remove the DeinitIf
                let Some(deinit_func) = deinit_entity else {
                    body.blocks[block].stmts.remove(stmt);
                    continue;
                };

                // Split: remove the DeinitIf, create deinit + skip blocks,
                // branch on flag.
                //
                // Before:
                //   block: [...stmts, DeinitIf, more_stmts..., terminator]
                //
                // After:
                //   block:       [...stmts, Branch(flag, skip_block, deinit_block)]
                //   deinit_block: [Call(deinit), Jump(cont_block)]
                //   cont_block:  [more_stmts..., original_terminator]
                //
                // DeinitIf semantics: flag=false → live, needs deinit.
                // So branch: if flag (true=moved), skip; else deinit.

                let remaining_stmts = body.blocks[block].stmts.split_off(stmt + 1);
                body.blocks[block].stmts.remove(stmt); // remove the DeinitIf
                let original_terminator =
                    std::mem::replace(&mut body.blocks[block].terminator, Terminator::unreachable());

                // Create continuation block with remaining stmts + original terminator
                let cont_block_id = BlockId::new(body.blocks.len());
                let mut cont_block = BasicBlock::new();
                cont_block.stmts = remaining_stmts;
                cont_block.terminator = original_terminator;
                body.blocks.push(cont_block);

                // Create deinit block: call deinit, jump to continuation
                let deinit_block_id = BlockId::new(body.blocks.len());
                let mut deinit_block = BasicBlock::new();
                let callee = deinit_callee(deinit_func, place_ty);
                deinit_block
                    .stmts
                    .push(Statement::new(StatementKind::Call {
                        dest: None,
                        callee,
                        args: vec![CallArg::mutating(Value::Place(place))],
                    }));
                deinit_block.terminator = Terminator::jump(cont_block_id);
                body.blocks.push(deinit_block);

                // Original block: branch on flag (true → skip, false → deinit)
                body.blocks[block].terminator = Terminator {
                    kind: TerminatorKind::Branch {
                        condition: Value::Place(Place::local(flag)),
                        then_block: cont_block_id,  // flag=true → skip deinit
                        else_block: deinit_block_id, // flag=false → call deinit
                    },
                    span: None,
                };
            }
            Expansion::ExpandEnumDrop {
                block,
                stmt,
                place,
                variant_drops,
                flag,
                ..
            } => {
                // Remove the Deinit/DeinitIf statement
                let remaining_stmts = body.blocks[block].stmts.split_off(stmt + 1);
                body.blocks[block].stmts.remove(stmt);
                let original_terminator =
                    std::mem::replace(&mut body.blocks[block].terminator, Terminator::unreachable());

                // Create continuation block
                let cont_block_id = BlockId::new(body.blocks.len());
                let mut cont_block = BasicBlock::new();
                cont_block.stmts = remaining_stmts;
                cont_block.terminator = original_terminator;
                body.blocks.push(cont_block);

                // Create the switch block that branches on the enum discriminant.
                // For each variant with droppable fields, create a deinit block.
                let switch_block_id = BlockId::new(body.blocks.len());
                let mut switch_block = BasicBlock::new();
                let mut cases: Vec<(SwitchCase, BlockId)> = Vec::new();

                // Group drops by variant name
                let mut variants_seen = HashSet::new();
                for (variant_name, _, _, _) in &variant_drops {
                    if !variants_seen.insert(variant_name.clone()) {
                        continue; // already handled this variant
                    }
                    // Create deinit block for this variant
                    let deinit_block_id = BlockId::new(body.blocks.len());
                    let mut deinit_block = BasicBlock::new();
                    // Deinit all droppable fields in this variant
                    for (vn, field_path, df, ft) in &variant_drops {
                        if vn == variant_name {
                            let mut field_place = place.clone().downcast(vn);
                            for segment in field_path {
                                field_place = field_place.field(segment);
                            }
                            let callee = deinit_callee(*df, ft.clone());
                            deinit_block.stmts.push(Statement::new(StatementKind::Call {
                                dest: None,
                                callee,
                                args: vec![CallArg::mutating(Value::Place(field_place))],
                            }));
                        }
                    }
                    deinit_block.terminator = Terminator::jump(cont_block_id);
                    body.blocks.push(deinit_block);
                    cases.push((SwitchCase::Variant(variant_name.clone()), deinit_block_id));
                }
                // Wildcard for variants without droppable fields → skip to cont
                cases.push((SwitchCase::Wildcard, cont_block_id));

                switch_block.terminator = Terminator {
                    kind: TerminatorKind::Switch {
                        discriminant: place.clone(),
                        cases,
                    },
                    span: None,
                };
                body.blocks.push(switch_block);

                // Wire the original block to either:
                // - Directly to switch (unconditional Deinit)
                // - Branch on flag then switch (DeinitIf)
                if let Some(flag_id) = flag {
                    // DeinitIf: branch on flag first
                    body.blocks[block].terminator = Terminator {
                        kind: TerminatorKind::Branch {
                            condition: Value::Place(Place::local(flag_id)),
                            then_block: cont_block_id,    // flag=true → skip
                            else_block: switch_block_id,  // flag=false → drop
                        },
                        span: None,
                    };
                } else {
                    // Unconditional: jump to switch
                    body.blocks[block].terminator = Terminator::jump(switch_block_id);
                }
            }
            Expansion::ExpandStructFieldDrop {
                block,
                stmt,
                place,
                field_drops,
                flag,
            } => {
                if let Some(flag_id) = flag {
                    // Conditional: branch on flag, then emit field drops
                    let remaining_stmts = body.blocks[block].stmts.split_off(stmt + 1);
                    body.blocks[block].stmts.remove(stmt);
                    let original_terminator =
                        std::mem::replace(&mut body.blocks[block].terminator, Terminator::unreachable());

                    let cont_block_id = BlockId::new(body.blocks.len());
                    let mut cont_block = BasicBlock::new();
                    cont_block.stmts = remaining_stmts;
                    cont_block.terminator = original_terminator;
                    body.blocks.push(cont_block);

                    let deinit_block_id = BlockId::new(body.blocks.len());
                    let mut deinit_block = BasicBlock::new();
                    for (field_path, deinit_entity, field_ty) in field_drops.iter().rev() {
                        let mut field_place = place.clone();
                        for segment in field_path {
                            field_place = field_place.field(segment);
                        }
                        let callee = deinit_callee(*deinit_entity, field_ty.clone());
                        deinit_block.stmts.push(Statement::new(StatementKind::Call {
                            dest: None,
                            callee,
                            args: vec![CallArg::mutating(Value::Place(field_place))],
                        }));
                    }
                    deinit_block.terminator = Terminator::jump(cont_block_id);
                    body.blocks.push(deinit_block);

                    body.blocks[block].terminator = Terminator {
                        kind: TerminatorKind::Branch {
                            condition: Value::Place(Place::local(flag_id)),
                            then_block: cont_block_id,
                            else_block: deinit_block_id,
                        },
                        span: None,
                    };
                } else {
                    // Unconditional: replace Deinit with field-level calls in place
                    body.blocks[block].stmts.remove(stmt);
                    for (i, (field_path, deinit_entity, field_ty)) in field_drops.iter().rev().enumerate() {
                        let mut field_place = place.clone();
                        for segment in field_path {
                            field_place = field_place.field(segment);
                        }
                        let callee = deinit_callee(*deinit_entity, field_ty.clone());
                        body.blocks[block].stmts.insert(stmt + i, Statement::new(StatementKind::Call {
                            dest: None,
                            callee,
                            args: vec![CallArg::mutating(Value::Place(field_place))],
                        }));
                    }
                }
            }
        }
    }
}

/// Resolve the MIR type of a Place.
fn resolve_place_type(place: &Place, body: &MirBody, module: &MirModule) -> Option<MirTy> {
    match place {
        Place::Local(id) => Some(body.locals[id.index()].ty.clone()),
        Place::Field { parent, name } => {
            let parent_ty = resolve_place_type(parent, body, module)?;
            // Unwrap references (init self is RefMut(Named{...}))
            let inner_ty = unwrap_ref(&parent_ty);
            let entity = struct_entity(inner_ty)?;
            let struct_def = module.structs.iter().find(|s| s.entity == entity)?;
            let field_id = struct_def.field_by_name(name)?;
            Some(struct_def.fields[field_id.index()].ty.clone())
        }
        Place::Deref(inner) => {
            let inner_ty = resolve_place_type(inner, body, module)?;
            match inner_ty {
                MirTy::Ref(pointee) | MirTy::RefMut(pointee) | MirTy::Pointer(pointee) => {
                    Some(*pointee)
                }
                _ => None,
            }
        }
        Place::Index { parent, index } => {
            let parent_ty = resolve_place_type(parent, body, module)?;
            match parent_ty {
                MirTy::Tuple(elems) => elems.get(*index).cloned(),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Build a Callee for a deinit method, extracting type args from the place type.
fn deinit_callee(deinit_func: Entity, place_ty: MirTy) -> Callee {
    let type_args = match &place_ty {
        MirTy::Named { type_args, .. } => type_args.clone(),
        _ => Vec::new(),
    };
    Callee::method(deinit_func, type_args, place_ty)
}

fn unwrap_ref(ty: &MirTy) -> &MirTy {
    match ty {
        MirTy::Ref(inner) | MirTy::RefMut(inner) => inner,
        other => other,
    }
}

fn struct_entity(ty: &MirTy) -> Option<Entity> {
    match ty {
        MirTy::Named { entity, .. } => Some(*entity),
        _ => None,
    }
}

/// For a struct type without explicit deinit, find sub-fields that need deinit.
/// Uses collect_struct_field_drops which handles one level of recursion.
fn struct_field_drops(
    ty: Option<&MirTy>,
    deinit_funcs: &HashMap<Entity, Entity>,
    module: &MirModule,
    drops: &mut Vec<(Vec<String>, Entity, MirTy)>,
) {
    let Some(ty) = ty else { return };
    let Some(entity) = struct_entity(ty) else { return };
    // Only for structs WITHOUT explicit deinit (those are handled by ReplaceDeinit)
    if deinit_funcs.contains_key(&entity) {
        return;
    }
    let Some(struct_def) = module.structs.iter().find(|s| s.entity == entity) else {
        return;
    };
    // Substitute the struct's type params with concrete args
    let type_args = match ty {
        MirTy::Named { type_args, .. } => type_args.as_slice(),
        _ => &[],
    };
    let subst: Vec<(Entity, &MirTy)> = struct_def
        .type_params
        .iter()
        .zip(type_args.iter())
        .map(|(p, t)| (p.entity, t))
        .collect();
    for field in &struct_def.fields {
        let resolved_ty = if subst.is_empty() {
            field.ty.clone()
        } else {
            substitute_type_params(&field.ty, &subst)
        };
        collect_struct_field_drops(
            &[field.name.clone()],
            &resolved_ty,
            deinit_funcs,
            module,
            drops,
        );
    }
}

/// For an enum type, find which variants have fields needing deinit.
/// Returns (variant_name, field_name, deinit_func_entity, field_ty) for each droppable field.
/// Substitutes the enum's type_args into the payload struct's field types.
fn enum_variant_drops(
    ty: Option<&MirTy>,
    deinit_funcs: &HashMap<Entity, Entity>,
    module: &MirModule,
) -> Option<Vec<(String, Vec<String>, Entity, MirTy)>> {
    let (entity, type_args) = match ty? {
        MirTy::Named { entity, type_args } => (*entity, type_args),
        _ => return None,
    };
    let enum_def = module.enums.iter().find(|e| e.entity == entity)?;

    // Build substitution: type_param entity → concrete type
    let subst: Vec<(Entity, &MirTy)> = enum_def
        .type_params
        .iter()
        .zip(type_args.iter())
        .map(|(p, t)| (p.entity, t))
        .collect();

    let mut drops = Vec::new();
    for case in &enum_def.cases {
        let payload = &module.structs[case.payload_struct.index()];
        for field in &payload.fields {
            let resolved_ty = substitute_type_params(&field.ty, &subst);
            collect_field_drops_recursive(
                &case.name,
                &[field.name.clone()],
                &resolved_ty,
                deinit_funcs,
                module,
                &mut drops,
            );
        }
    }
    Some(drops)
}

/// Collect drop calls for a field type in an enum variant. If the type has a
/// direct deinit, emit a single drop. If it's a struct without deinit but
/// with sub-fields that have deinit, recurse one level into those sub-fields.
fn collect_field_drops_recursive(
    variant_name: &str,
    path: &[String],
    field_ty: &MirTy,
    deinit_funcs: &HashMap<Entity, Entity>,
    module: &MirModule,
    drops: &mut Vec<(String, Vec<String>, Entity, MirTy)>,
) {
    let Some(entity) = struct_entity(field_ty) else {
        return;
    };
    if let Some(&deinit_func) = deinit_funcs.get(&entity) {
        drops.push((
            variant_name.to_string(),
            path.to_vec(),
            deinit_func,
            field_ty.clone(),
        ));
        return;
    }
    // Struct without deinit — recurse into sub-fields that have deinit.
    // Substitute the struct's own type params with concrete args from field_ty.
    let Some(struct_def) = module.structs.iter().find(|s| s.entity == entity) else {
        return;
    };
    let type_args = match field_ty {
        MirTy::Named { type_args, .. } => type_args.as_slice(),
        _ => &[],
    };
    let subst: Vec<(Entity, &MirTy)> = struct_def
        .type_params
        .iter()
        .zip(type_args.iter())
        .map(|(p, t)| (p.entity, t))
        .collect();
    for sub_field in &struct_def.fields {
        let resolved_ty = if subst.is_empty() {
            sub_field.ty.clone()
        } else {
            substitute_type_params(&sub_field.ty, &subst)
        };
        if let Some(sub_entity) = struct_entity(&resolved_ty) {
            if let Some(&sub_deinit) = deinit_funcs.get(&sub_entity) {
                let mut sub_path = path.to_vec();
                sub_path.push(sub_field.name.clone());
                drops.push((
                    variant_name.to_string(),
                    sub_path,
                    sub_deinit,
                    resolved_ty,
                ));
            }
        }
    }
}

/// Substitute TypeParam references with concrete types throughout a MirTy tree.
fn substitute_type_params(ty: &MirTy, subst: &[(Entity, &MirTy)]) -> MirTy {
    match ty {
        MirTy::TypeParam(entity) => {
            for &(param_entity, concrete) in subst {
                if *entity == param_entity {
                    return concrete.clone();
                }
            }
            ty.clone()
        }
        MirTy::Named { entity, type_args } => MirTy::Named {
            entity: *entity,
            type_args: type_args
                .iter()
                .map(|t| substitute_type_params(t, subst))
                .collect(),
        },
        MirTy::Pointer(inner) => MirTy::Pointer(Box::new(substitute_type_params(inner, subst))),
        MirTy::Ref(inner) => MirTy::Ref(Box::new(substitute_type_params(inner, subst))),
        MirTy::RefMut(inner) => MirTy::RefMut(Box::new(substitute_type_params(inner, subst))),
        MirTy::Tuple(elems) => MirTy::Tuple(
            elems.iter().map(|t| substitute_type_params(t, subst)).collect(),
        ),
        MirTy::FuncThin { params, ret } => MirTy::FuncThin {
            params: params.iter().map(|t| substitute_type_params(t, subst)).collect(),
            ret: Box::new(substitute_type_params(ret, subst)),
        },
        MirTy::FuncThick { params, ret } => MirTy::FuncThick {
            params: params.iter().map(|t| substitute_type_params(t, subst)).collect(),
            ret: Box::new(substitute_type_params(ret, subst)),
        },
        MirTy::AssociatedProjection { base, protocol, name } => MirTy::AssociatedProjection {
            base: Box::new(substitute_type_params(base, subst)),
            protocol: *protocol,
            name: name.clone(),
        },
        _ => ty.clone(),
    }
}

/// Collect field deinit calls for a struct field. If the field's type has a
/// direct deinit, emit one call. If it's a struct without deinit but with
/// sub-fields that have direct deinit, emit calls for those sub-fields.
/// Does NOT recurse deeper to avoid reaching into stdlib internals.
fn collect_struct_field_drops(
    path: &[String],
    ty: &MirTy,
    deinit_funcs: &HashMap<Entity, Entity>,
    module: &MirModule,
    drops: &mut Vec<(Vec<String>, Entity, MirTy)>,
) {
    let Some(entity) = struct_entity(ty) else { return };
    if let Some(&deinit_func) = deinit_funcs.get(&entity) {
        drops.push((path.to_vec(), deinit_func, ty.clone()));
    } else {
        // One level of recursion: substitute the struct's type params with
        // concrete args from `ty` so sub-field types are fully resolved.
        let Some(struct_def) = module.structs.iter().find(|s| s.entity == entity) else {
            return;
        };
        let type_args = match ty {
            MirTy::Named { type_args, .. } => type_args.as_slice(),
            _ => &[],
        };
        let subst: Vec<(Entity, &MirTy)> = struct_def
            .type_params
            .iter()
            .zip(type_args.iter())
            .map(|(p, t)| (p.entity, t))
            .collect();
        for sub_field in &struct_def.fields {
            let resolved_ty = if subst.is_empty() {
                sub_field.ty.clone()
            } else {
                substitute_type_params(&sub_field.ty, &subst)
            };
            if let Some(sub_entity) = struct_entity(&resolved_ty) {
                if let Some(&sub_deinit) = deinit_funcs.get(&sub_entity) {
                    let mut sub_path = path.to_vec();
                    sub_path.push(sub_field.name.clone());
                    drops.push((sub_path, sub_deinit, resolved_ty));
                }
            }
        }
    }
}

/// For each deinit function, insert calls to field-level deinit functions
/// before each Return terminator. This ensures that when a struct with a
/// user-defined deinit is destroyed, its fields are also destroyed.
fn inject_field_deinits(module: &mut MirModule, deinit_funcs: &HashMap<Entity, Entity>) {
    // Collect (func_idx, field_name, field_deinit_entity, field_ty) for each
    // deinit function whose parent struct has fields with their own deinit.
    let mut injections: Vec<(usize, Vec<(Vec<String>, Entity, MirTy)>)> = Vec::new();

    for (func_idx, func) in module.functions.iter().enumerate() {
        let FunctionKind::Deinit { parent } = &func.kind else {
            continue;
        };
        if func.body.is_none() {
            continue;
        }

        let Some(struct_def) = module.structs.iter().find(|s| s.entity == *parent) else {
            continue;
        };

        // Collect field-level deinit calls, including one level of recursion
        // for fields that are structs without deinit but with droppable sub-fields.
        // Generic deinit functions are included — the monomorphizer discovers
        // them via BFS after expand_deinit injects the Call statements.
        let mut field_deinits: Vec<(Vec<String>, Entity, MirTy)> = Vec::new();
        for field in &struct_def.fields {
            collect_struct_field_drops(
                &[field.name.clone()],
                &field.ty,
                deinit_funcs,
                module,
                &mut field_deinits,
            );
        }

        if !field_deinits.is_empty() {
            injections.push((func_idx, field_deinits));
        }
    }

    // Apply injections: insert field deinit calls before each Return
    for (func_idx, field_deinits) in injections {
        let body = module.functions[func_idx].body.as_mut().unwrap();
        for block_idx in 0..body.blocks.len() {
            if !matches!(
                body.blocks[block_idx].terminator.kind,
                TerminatorKind::Return(_)
            ) {
                continue;
            }
            // Insert field deinit calls in reverse field order before the return
            for (field_path, deinit_entity, field_ty) in field_deinits.iter().rev() {
                let self_local = LocalId::new(0);
                let mut place = Place::local(self_local);
                for segment in field_path {
                    place = place.field(segment);
                }
                let callee = deinit_callee(*deinit_entity, field_ty.clone());
                body.blocks[block_idx]
                    .stmts
                    .push(Statement::new(StatementKind::Call {
                        dest: None,
                        callee,
                        args: vec![CallArg::mutating(Value::Place(place))],
                    }));
            }
        }
    }
}
