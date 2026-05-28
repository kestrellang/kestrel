use std::collections::HashMap;

use kestrel_hecs::Entity;

use crate::block::BlockParam;
use crate::body::OssaBody;
use crate::inst::{InstKind, Instruction};
use crate::item::function::{FunctionDef, FunctionKind, ParamDef};
use crate::item::witness::{WitnessDef, WitnessMethodBinding, WitnessMethodKey};
use crate::item::CopyBehavior;
use crate::terminator::{SwitchArm, Terminator, TerminatorKind};
use crate::ty::{MirTy, ParamConvention};
use crate::ty_query::find_cloneable_protocol;
use crate::value::{Ownership, ValueDef};
use crate::ty::TyArena;
use crate::{FieldIdx, MirModule, TyId, ValueId, VariantIdx};

/// Synthesize `__clone$T` functions for all structs/enums that aren't `not Copyable`.
/// Skips types that already have a user-written `.clone()` via a Cloneable witness.
/// Registers each synthesized shim as a Cloneable witness.
pub fn synthesize_clone_shims(module: &mut MirModule, next_entity: &mut u32) {
    let Some(cloneable_proto) = find_cloneable_protocol(module) else {
        return;
    };

    // Pre-intern Pointer(Named(entity)) for all candidate types (needed by BeginBorrow)
    for s in module.structs.values() {
        if s.type_info.copy == CopyBehavior::None {
            continue;
        }
        let tp_ty_ids: Vec<TyId> = s.type_params.iter()
            .map(|tp| module.ty_arena.intern(MirTy::TypeParam(tp.entity)))
            .collect();
        let named_ty = module.ty_arena.intern(MirTy::Named {
            entity: s.entity,
            type_args: tp_ty_ids,
        });
        module.ty_arena.pointer(named_ty);
    }
    for e in module.enums.values() {
        if e.type_info.copy == CopyBehavior::None {
            continue;
        }
        let tp_ty_ids: Vec<TyId> = e.type_params.iter()
            .map(|tp| module.ty_arena.intern(MirTy::TypeParam(tp.entity)))
            .collect();
        let named_ty = module.ty_arena.intern(MirTy::Named {
            entity: e.entity,
            type_args: tp_ty_ids,
        });
        module.ty_arena.pointer(named_ty);
    }

    // Pre-intern i32 for enum discriminants
    module.ty_arena.i32();

    // Collect types that already have a user-written Cloneable witness
    let has_user_clone: std::collections::HashSet<Entity> = module.witnesses.iter()
        .filter(|w| w.protocol == cloneable_proto)
        .filter_map(|w| {
            if let MirTy::Named { entity, .. } = module.ty_arena.get(w.implementing_type) {
                Some(*entity)
            } else {
                None
            }
        })
        .collect();

    // Build worklist: all structs/enums that don't already have a user clone.
    // Skip closure envs and types with unresolvable fields.
    let closure_env_entities: std::collections::HashSet<Entity> = module.functions.values()
        .filter_map(|f| match &f.kind {
            FunctionKind::ClosureCall { env_struct } => Some(*env_struct),
            _ => None,
        })
        .collect();

    let mut worklist: Vec<Entity> = Vec::new();
    for s in module.structs.values() {
        if has_user_clone.contains(&s.entity)
            || closure_env_entities.contains(&s.entity)
            || s.type_info.copy == CopyBehavior::None
            || has_unresolvable_fields_struct(s, &module.ty_arena)
        {
            continue;
        }
        worklist.push(s.entity);
    }
    for e in module.enums.values() {
        if has_user_clone.contains(&e.entity)
            || e.type_info.copy == CopyBehavior::None
            || has_unresolvable_fields_enum(e, &module.ty_arena)
        {
            continue;
        }
        worklist.push(e.entity);
    }

    let mut shim_map: HashMap<Entity, Entity> = HashMap::new();

    while let Some(type_entity) = worklist.pop() {
        if shim_map.contains_key(&type_entity) {
            continue;
        }

        let shim_entity = Entity::from_raw(*next_entity);
        *next_entity += 1;

        let Some(func) = generate_clone_shim(module, type_entity, shim_entity) else {
            continue;
        };

        let name = func.name.clone();
        module.register_name(shim_entity, &name);
        module.functions.insert(shim_entity, func);
        shim_map.insert(type_entity, shim_entity);
    }

    // Register each shim as a Cloneable witness
    for (&type_entity, &shim_entity) in &shim_map {
        let func = &module.functions[&shim_entity];
        let tp_ty_ids: Vec<TyId> = func.type_params.iter()
            .map(|tp| module.ty_arena.find(|t| matches!(t, MirTy::TypeParam(e) if *e == tp.entity))
                .expect("TypeParam should be interned"))
            .collect();
        let self_ty = module.ty_arena.find(|t| {
            matches!(t, MirTy::Named { entity, type_args }
                if *entity == type_entity && *type_args == tp_ty_ids)
        }).expect("Named type should be interned");

        let mut witness = WitnessDef::new(cloneable_proto, self_ty);
        witness.add_method(WitnessMethodBinding::new(
            WitnessMethodKey::simple("clone"),
            shim_entity,
            tp_ty_ids,
        ));
        module.add_witness(witness);
    }

    // Set CopyBehavior::Clone for all shim'd types that have non-trivial fields.
    // Types with only primitive fields stay Bitwise — the expand pass handles them.
    for (&type_entity, _) in &shim_map {
        // Compute predicate with shared borrow, then mutate separately
        let needs_struct = module.structs.get(&type_entity)
            .map(|s| needs_clone_shim_struct(s, &module.ty_arena))
            .unwrap_or(false);
        if needs_struct {
            module.structs.get_mut(&type_entity).unwrap().type_info.copy =
                CopyBehavior::Clone(cloneable_proto);
            continue;
        }

        let needs_enum = module.enums.get(&type_entity)
            .map(|e| needs_clone_shim_enum(e, &module.ty_arena))
            .unwrap_or(false);
        if needs_enum {
            module.enums.get_mut(&type_entity).unwrap().type_info.copy =
                CopyBehavior::Clone(cloneable_proto);
        }
    }
}

fn generate_clone_shim(
    module: &MirModule,
    type_entity: Entity,
    shim_entity: Entity,
) -> Option<FunctionDef> {
    if let Some(s) = module.structs.get(&type_entity) {
        return Some(generate_struct_clone_shim(module, s, shim_entity));
    }
    if let Some(e) = module.enums.get(&type_entity) {
        return Some(generate_enum_clone_shim(module, e, shim_entity));
    }
    None
}

fn generate_struct_clone_shim(
    module: &MirModule,
    struct_def: &crate::item::struct_def::StructDef,
    shim_entity: Entity,
) -> FunctionDef {
    let name = format!("__clone${}", struct_def.name);

    let tp_ty_ids: Vec<TyId> = struct_def.type_params.iter()
        .map(|tp| module.ty_arena.find(|t| matches!(t, MirTy::TypeParam(e) if *e == tp.entity))
            .expect("TypeParam should be interned"))
        .collect();
    let self_ty = module.ty_arena.find(|t| {
        matches!(t, MirTy::Named { entity, type_args }
            if *entity == struct_def.entity && *type_args == tp_ty_ids)
    }).expect("struct type should be interned");

    let mut body = OssaBody::new();

    // Self param: @guaranteed T (Borrow convention).
    // StructExtract on @guaranteed → @guaranteed fields, CopyValue → @owned clones.
    let self_val = body.alloc_value(ValueDef {
        ty: self_ty,
        ownership: Ownership::Guaranteed,
        borrow_source: None,
    });
    body.param_count = 1;

    let entry = body.alloc_block();
    body.entry = entry;
    body.block_mut(entry).params.push(BlockParam {
        value: self_val,
        ty: self_ty,
        ownership: Ownership::Guaranteed,
    });

    let mut insts = Vec::new();

    let mut cloned_fields: Vec<(FieldIdx, ValueId)> = Vec::new();
    for (i, field_def) in struct_def.fields.iter().enumerate() {
        let fi = FieldIdx::new(i);
        let field_val = body.alloc_value(ValueDef::guaranteed(field_def.ty, self_val));
        insts.push(Instruction::new(InstKind::StructExtract {
            result: field_val,
            operand: self_val,
            field: fi,
        }));
        let cloned_val = body.alloc_value(ValueDef::owned(field_def.ty));
        insts.push(Instruction::new(InstKind::CopyValue {
            result: cloned_val,
            operand: field_val,
        }));
        cloned_fields.push((fi, cloned_val));
    }

    let result_val = body.alloc_value(ValueDef::owned(self_ty));
    insts.push(Instruction::new(InstKind::Struct {
        result: result_val,
        ty: self_ty,
        fields: cloned_fields,
    }));

    body.block_mut(entry).insts = insts;
    body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(result_val));

    let mut func = FunctionDef::new(shim_entity, name, self_ty);
    func.kind = FunctionKind::CloneShim { nominal: struct_def.entity };
    func.type_params = struct_def.type_params.clone();
    func.params.push(ParamDef::new("self", self_val, self_ty, ParamConvention::Borrow));
    func.body = Some(body);
    func
}

fn generate_enum_clone_shim(
    module: &MirModule,
    enum_def: &crate::item::enum_def::EnumDef,
    shim_entity: Entity,
) -> FunctionDef {
    let name = format!("__clone${}", enum_def.name);

    let tp_ty_ids: Vec<TyId> = enum_def.type_params.iter()
        .map(|tp| module.ty_arena.find(|t| matches!(t, MirTy::TypeParam(e) if *e == tp.entity))
            .expect("TypeParam should be interned"))
        .collect();
    let self_ty = module.ty_arena.find(|t| {
        matches!(t, MirTy::Named { entity, type_args }
            if *entity == enum_def.entity && *type_args == tp_ty_ids)
    }).expect("enum type should be interned");

    let i32_ty = module.ty_arena.find(|t| matches!(t, MirTy::I32))
        .unwrap_or_else(|| module.ty_arena.find(|t| matches!(t, MirTy::I8))
            .expect("integer type should be interned"));

    let mut body = OssaBody::new();

    // Self param: @guaranteed T (Borrow convention)
    let self_val = body.alloc_value(ValueDef {
        ty: self_ty,
        ownership: Ownership::Guaranteed,
        borrow_source: None,
    });
    body.param_count = 1;

    let entry = body.alloc_block();
    body.entry = entry;
    body.block_mut(entry).params.push(BlockParam {
        value: self_val,
        ty: self_ty,
        ownership: Ownership::Guaranteed,
    });

    let mut entry_insts = Vec::new();

    // Discriminant — non-consuming read on @guaranteed
    let disc_val = body.alloc_value(ValueDef::owned(i32_ty));
    entry_insts.push(Instruction::new(InstKind::Discriminant {
        result: disc_val,
        operand: self_val,
    }));

    // Exit block — receives cloned result, returns
    let exit_block = body.alloc_block();
    let exit_result = body.alloc_value(ValueDef::owned(self_ty));
    body.block_mut(exit_block).params.push(BlockParam {
        value: exit_result,
        ty: self_ty,
        ownership: Ownership::Owned,
    });
    body.block_mut(exit_block).terminator =
        Terminator::new(TerminatorKind::Return(exit_result));

    // Per-variant blocks — EnumPayload on @guaranteed, CopyValue to clone
    let mut switch_arms = Vec::new();

    for (case_idx, case_def) in enum_def.cases.iter().enumerate() {
        let variant_idx = VariantIdx::new(case_idx);
        let variant_block = body.alloc_block();

        // Forward self (@guaranteed) and disc (@owned) as block args
        let variant_self = body.alloc_value(ValueDef {
            ty: self_ty,
            ownership: Ownership::Guaranteed,
            borrow_source: None,
        });
        let variant_disc = body.alloc_value(ValueDef::owned(i32_ty));
        body.block_mut(variant_block).params.push(BlockParam {
            value: variant_self,
            ty: self_ty,
            ownership: Ownership::Guaranteed,
        });
        body.block_mut(variant_block).params.push(BlockParam {
            value: variant_disc,
            ty: i32_ty,
            ownership: Ownership::Owned,
        });

        let mut variant_insts = Vec::new();
        variant_insts.push(Instruction::new(InstKind::DestroyValue { operand: variant_disc }));

        // EnumPayload on @guaranteed → @guaranteed projections
        let mut cloned_payloads = Vec::new();
        for (fi, pf) in case_def.payload_fields.iter().enumerate() {
            let field_idx = FieldIdx::new(fi);
            let payload_val = body.alloc_value(ValueDef::guaranteed(pf.ty, variant_self));
            variant_insts.push(Instruction::new(InstKind::EnumPayload {
                result: payload_val,
                operand: variant_self,
                variant: variant_idx,
                field: field_idx,
            }));
            let cloned_val = body.alloc_value(ValueDef::owned(pf.ty));
            variant_insts.push(Instruction::new(InstKind::CopyValue {
                result: cloned_val,
                operand: payload_val,
            }));
            cloned_payloads.push(cloned_val);
        }

        // Construct cloned enum variant
        let cloned_enum = body.alloc_value(ValueDef::owned(self_ty));
        variant_insts.push(Instruction::new(InstKind::Enum {
            result: cloned_enum,
            enum_ty: self_ty,
            variant: variant_idx,
            payload: cloned_payloads,
        }));

        body.block_mut(variant_block).insts = variant_insts;
        body.block_mut(variant_block).terminator =
            Terminator::new(TerminatorKind::Jump {
                target: exit_block,
                args: vec![cloned_enum],
            });

        switch_arms.push(SwitchArm {
            pattern: crate::SwitchCase::Variant(variant_idx),
            target: variant_block,
            args: vec![self_val, disc_val],
        });
    }

    // Wildcard block — CopyValue of entire @guaranteed self
    let wildcard_block = body.alloc_block();
    let wildcard_self = body.alloc_value(ValueDef {
        ty: self_ty,
        ownership: Ownership::Guaranteed,
        borrow_source: None,
    });
    let wildcard_disc = body.alloc_value(ValueDef::owned(i32_ty));
    body.block_mut(wildcard_block).params.push(BlockParam {
        value: wildcard_self,
        ty: self_ty,
        ownership: Ownership::Guaranteed,
    });
    body.block_mut(wildcard_block).params.push(BlockParam {
        value: wildcard_disc,
        ty: i32_ty,
        ownership: Ownership::Owned,
    });
    let mut wildcard_insts = Vec::new();
    wildcard_insts.push(Instruction::new(InstKind::DestroyValue { operand: wildcard_disc }));
    let wildcard_copy = body.alloc_value(ValueDef::owned(self_ty));
    wildcard_insts.push(Instruction::new(InstKind::CopyValue {
        result: wildcard_copy,
        operand: wildcard_self,
    }));
    body.block_mut(wildcard_block).insts = wildcard_insts;
    body.block_mut(wildcard_block).terminator =
        Terminator::new(TerminatorKind::Jump {
            target: exit_block,
            args: vec![wildcard_copy],
        });
    switch_arms.push(SwitchArm {
        pattern: crate::SwitchCase::Wildcard,
        target: wildcard_block,
        args: vec![self_val, disc_val],
    });

    body.block_mut(entry).insts = entry_insts;
    body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Switch {
        discriminant: disc_val,
        cases: switch_arms,
    });

    let mut func = FunctionDef::new(shim_entity, name, self_ty);
    func.kind = FunctionKind::CloneShim { nominal: enum_def.entity };
    func.type_params = enum_def.type_params.clone();
    func.params.push(ParamDef::new("self", self_val, self_ty, ParamConvention::Borrow));
    func.body = Some(body);
    func
}

fn ty_contains_unresolvable(ty: TyId, arena: &TyArena) -> bool {
    match arena.get(ty) {
        MirTy::AssociatedProjection { .. } => true,
        MirTy::Named { type_args, .. } => type_args.iter().any(|&t| ty_contains_unresolvable(t, arena)),
        MirTy::Pointer(inner) => ty_contains_unresolvable(*inner, arena),
        MirTy::Tuple(elems) => elems.iter().any(|&t| ty_contains_unresolvable(t, arena)),
        MirTy::FuncThick { params, ret, .. } => {
            params.iter().any(|&(t, _)| ty_contains_unresolvable(t, arena))
                || ty_contains_unresolvable(*ret, arena)
        }
        _ => false,
    }
}

fn has_unresolvable_fields_struct(
    s: &crate::item::struct_def::StructDef,
    arena: &TyArena,
) -> bool {
    s.fields.iter().any(|f| ty_contains_unresolvable(f.ty, arena))
}

fn has_unresolvable_fields_enum(
    e: &crate::item::enum_def::EnumDef,
    arena: &TyArena,
) -> bool {
    e.cases.iter().any(|c| c.payload_fields.iter().any(|f| ty_contains_unresolvable(f.ty, arena)))
}

/// A struct needs a clone shim only if it has at least one Named or TypeParam
/// field — those might need deep cloning. All-primitive structs are trivially
/// bitwise-copyable and don't need a shim.
fn needs_clone_shim_struct(
    s: &crate::item::struct_def::StructDef,
    arena: &TyArena,
) -> bool {
    s.fields.iter().any(|f| matches!(arena.get(f.ty), MirTy::Named { .. } | MirTy::TypeParam(_)))
}

fn needs_clone_shim_enum(
    e: &crate::item::enum_def::EnumDef,
    arena: &TyArena,
) -> bool {
    e.cases.iter().any(|c| {
        c.payload_fields.iter().any(|f| matches!(arena.get(f.ty), MirTy::Named { .. } | MirTy::TypeParam(_)))
    })
}
