use std::collections::HashMap;

use kestrel_hecs::Entity;

use crate::block::BlockParam;
use crate::body::{OssaBody, ownership_for_type};
use crate::callee::Callee;
use crate::inst::{CallArg, InstKind, Instruction};
use crate::item::function::{FunctionDef, FunctionKind, ParamDef};
use crate::terminator::{SwitchArm, Terminator, TerminatorKind};
use crate::ty::{MirTy, ParamConvention};
use crate::value::{Ownership, ValueDef};
use crate::{DropBehavior, FieldIdx, Immediate, MirModule, TyId};

/// Synthesize `__drop$T` functions for all types that need cleanup.
/// Adds the generated functions to `module.functions`.
pub fn synthesize_drop_shims(module: &mut MirModule, next_entity: &mut u32) {
    module.ty_arena.unit();

    // Pre-intern Named types so shim generation can find them.
    for s in &module.structs {
        let type_args: Vec<TyId> = s.type_params.iter()
            .map(|tp| module.ty_arena.intern(MirTy::TypeParam(tp.entity)))
            .collect();
        module.ty_arena.intern(MirTy::Named {
            entity: s.entity,
            type_args,
        });
    }
    for e in &module.enums {
        let type_args: Vec<TyId> = e.type_params.iter()
            .map(|tp| module.ty_arena.intern(MirTy::TypeParam(tp.entity)))
            .collect();
        module.ty_arena.intern(MirTy::Named {
            entity: e.entity,
            type_args,
        });
    }

    // Pre-intern integer types for discriminant values
    module.ty_arena.i32();

    // Pre-intern Pointer(Named(entity)) for types with deinit (needed by BeginMutBorrow)
    for s in &module.structs {
        if let DropBehavior::StructDrop { deinit: Some(_), .. } = &s.type_info.drop {
            let tp_ty_ids: Vec<TyId> = s.type_params.iter()
                .map(|tp| module.ty_arena.intern(MirTy::TypeParam(tp.entity)))
                .collect();
            let named_ty = module.ty_arena.intern(MirTy::Named {
                entity: s.entity,
                type_args: tp_ty_ids,
            });
            module.ty_arena.pointer(named_ty);
        }
    }
    for e in &module.enums {
        if let DropBehavior::EnumDrop { deinit: Some(_), .. } = &e.type_info.drop {
            let tp_ty_ids: Vec<TyId> = e.type_params.iter()
                .map(|tp| module.ty_arena.intern(MirTy::TypeParam(tp.entity)))
                .collect();
            let named_ty = module.ty_arena.intern(MirTy::Named {
                entity: e.entity,
                type_args: tp_ty_ids,
            });
            module.ty_arena.pointer(named_ty);
        }
    }

    let mut shim_map: HashMap<Entity, usize> = HashMap::new();
    let mut worklist: Vec<Entity> = Vec::new();

    for s in &module.structs {
        if s.type_info.drop != DropBehavior::None {
            worklist.push(s.entity);
        }
    }
    for e in &module.enums {
        if e.type_info.drop != DropBehavior::None {
            worklist.push(e.entity);
        }
    }

    while let Some(type_entity) = worklist.pop() {
        if shim_map.contains_key(&type_entity) {
            continue;
        }

        let shim_entity = Entity::from_raw(*next_entity);
        *next_entity += 1;

        let (func, field_type_entities) =
            generate_shim(module, type_entity, shim_entity, next_entity);

        let func_idx = module.functions.len();
        let name = func.name.clone();
        module.register_name(shim_entity, &name);
        module.functions.push(func);
        shim_map.insert(type_entity, func_idx);

        for field_entity in field_type_entities {
            if !shim_map.contains_key(&field_entity) {
                worklist.push(field_entity);
            }
        }
    }

    patch_shim_callees(module, &shim_map);
}

fn generate_shim(
    module: &MirModule,
    type_entity: Entity,
    shim_entity: Entity,
    next_entity: &mut u32,
) -> (FunctionDef, Vec<Entity>) {
    for s in &module.structs {
        if s.entity == type_entity {
            return generate_struct_shim(module, s, shim_entity, next_entity);
        }
    }
    for e in &module.enums {
        if e.entity == type_entity {
            return generate_enum_shim(module, e, shim_entity, next_entity);
        }
    }
    panic!("drop shim requested for unknown type entity {type_entity:?}");
}

fn generate_struct_shim(
    module: &MirModule,
    struct_def: &crate::item::struct_def::StructDef,
    shim_entity: Entity,
    _next_entity: &mut u32,
) -> (FunctionDef, Vec<Entity>) {
    let name = format!("__drop${}", struct_def.name);

    let tp_ty_ids: Vec<TyId> = struct_def.type_params.iter()
        .map(|tp| module.ty_arena.find(|t| matches!(t, MirTy::TypeParam(e) if *e == tp.entity))
            .expect("TypeParam should be interned"))
        .collect();
    let self_ty = module.ty_arena.find(|t| {
        matches!(t, MirTy::Named { entity, type_args }
            if *entity == struct_def.entity && *type_args == tp_ty_ids)
    }).expect("struct type should be interned");

    let unit_ty = module.ty_arena.find(|t| matches!(t, MirTy::Tuple(e) if e.is_empty()))
        .expect("unit type should be interned");

    let DropBehavior::StructDrop { deinit, fields } = &struct_def.type_info.drop else {
        panic!("expected StructDrop");
    };

    let mut body = OssaBody::new();
    let mut field_type_entities = Vec::new();

    // %self = entry block param, @owned
    let self_val = body.alloc_value(ValueDef::owned(self_ty));
    body.param_count = 1;

    let entry = body.alloc_block();
    body.entry = entry;
    body.block_mut(entry).params.push(BlockParam {
        value: self_val,
        ty: self_ty,
        ownership: Ownership::Owned,
    });

    let mut insts = Vec::new();

    // 1. Optional deinit call: borrow self, call deinit, end borrow
    if let Some(deinit_entity) = deinit {
        let ptr_ty = module.ty_arena.find(|t| matches!(t, MirTy::Pointer(p) if *p == self_ty))
            .unwrap_or_else(|| {
                // The pointer type might not be interned yet — that's fine,
                // the body uses the value but the type won't be looked up
                // outside of display/verify which can handle missing types.
                // Actually we need it for the ValueDef. Let's just create it
                // with a known ID approach: we'll intern it in the body's
                // value defs using the arena we have.
                //
                // But we can't mutate module.ty_arena here since we only
                // have &MirModule. We'll need to intern it before calling.
                panic!("Pointer({self_ty:?}) not interned; pre-intern pointer types before shim synthesis");
            });
        let borrow_val = body.alloc_value(ValueDef::guaranteed(ptr_ty, self_val));
        insts.push(Instruction::new(InstKind::BeginMutBorrow {
            result: borrow_val,
            operand: self_val,
        }));
        insts.push(Instruction::new(InstKind::Call {
            result: None,
            callee: Callee::direct_with_args(*deinit_entity, tp_ty_ids.clone(), None),
            args: vec![CallArg {
                value: borrow_val,
                convention: ParamConvention::MutBorrow,
            }],
        }));
        insts.push(Instruction::new(InstKind::EndMutBorrow {
            operand: borrow_val,
        }));
    }

    // 2. Destructure struct — consumes self, produces per-field values.
    // Fields in DropBehavior.fields are forced @owned so DestroyValue is
    // valid pre-mono. After monomorphization, ownership is re-derived from
    // the concrete type, and the expand pass removes/replaces DestroyValue.
    let field_count = struct_def.fields.len();
    let mut field_vals = Vec::with_capacity(field_count);
    for (i, field_def) in struct_def.fields.iter().enumerate() {
        let fi = FieldIdx::new(i);
        let ownership = if fields.contains(&fi) {
            Ownership::Owned
        } else {
            ownership_for_type(field_def.ty, &module.ty_arena, module)
        };
        field_vals.push(body.alloc_value(ValueDef {
            ty: field_def.ty,
            ownership,
            borrow_source: None,
        }));
    }
    insts.push(Instruction::new(InstKind::DestructureStruct {
        results: field_vals.clone(),
        operand: self_val,
    }));

    // 3. Destroy all fields — droppable fields trigger transitive drop shims,
    //    non-droppable fields must still be consumed since all values are @owned.
    let droppable_set: std::collections::HashSet<FieldIdx> = fields.iter().copied().collect();
    for (i, fv) in field_vals.iter().enumerate() {
        let fi = FieldIdx::new(i);
        let field_ty = struct_def.fields[i].ty;
        insts.push(Instruction::new(InstKind::DestroyValue { operand: *fv }));
        if droppable_set.contains(&fi) {
            if let MirTy::Named { entity, .. } = module.ty_arena.get(field_ty) {
                field_type_entities.push(*entity);
            }
        }
    }

    // 4. Return unit
    let unit_val = body.alloc_value(ValueDef::owned(unit_ty));
    insts.push(Instruction::new(InstKind::Literal {
        result: unit_val,
        value: Immediate::unit(),
    }));

    body.block_mut(entry).insts = insts;
    body.block_mut(entry).terminator = Terminator::new(TerminatorKind::Return(unit_val));

    let mut func = FunctionDef::new(shim_entity, name, unit_ty);
    func.kind = FunctionKind::DropShim { nominal: struct_def.entity };
    func.type_params = struct_def.type_params.clone();
    func.params.push(ParamDef::new("self", self_val, self_ty, ParamConvention::Consuming));
    func.body = Some(body);

    (func, field_type_entities)
}

fn generate_enum_shim(
    module: &MirModule,
    enum_def: &crate::item::enum_def::EnumDef,
    shim_entity: Entity,
    _next_entity: &mut u32,
) -> (FunctionDef, Vec<Entity>) {
    let name = format!("__drop${}", enum_def.name);

    let tp_ty_ids: Vec<TyId> = enum_def.type_params.iter()
        .map(|tp| module.ty_arena.find(|t| matches!(t, MirTy::TypeParam(e) if *e == tp.entity))
            .expect("TypeParam should be interned"))
        .collect();
    let self_ty = module.ty_arena.find(|t| {
        matches!(t, MirTy::Named { entity, type_args }
            if *entity == enum_def.entity && *type_args == tp_ty_ids)
    }).expect("enum type should be interned");

    let unit_ty = module.ty_arena.find(|t| matches!(t, MirTy::Tuple(e) if e.is_empty()))
        .expect("unit type should be interned");

    let DropBehavior::EnumDrop { deinit, variants } = &enum_def.type_info.drop else {
        panic!("expected EnumDrop");
    };

    let mut body = OssaBody::new();
    let mut field_type_entities = Vec::new();

    // %self = entry block param, @owned
    let self_val = body.alloc_value(ValueDef::owned(self_ty));
    body.param_count = 1;

    let entry = body.alloc_block();
    body.entry = entry;
    body.block_mut(entry).params.push(BlockParam {
        value: self_val,
        ty: self_ty,
        ownership: Ownership::Owned,
    });

    let mut entry_insts = Vec::new();

    // Optional deinit call
    if let Some(deinit_entity) = deinit {
        let ptr_ty = module.ty_arena.find(|t| matches!(t, MirTy::Pointer(p) if *p == self_ty))
            .expect("Pointer type should be interned");
        let borrow_val = body.alloc_value(ValueDef::guaranteed(ptr_ty, self_val));
        entry_insts.push(Instruction::new(InstKind::BeginMutBorrow {
            result: borrow_val,
            operand: self_val,
        }));
        entry_insts.push(Instruction::new(InstKind::Call {
            result: None,
            callee: Callee::direct_with_args(*deinit_entity, tp_ty_ids.clone(), None),
            args: vec![CallArg {
                value: borrow_val,
                convention: ParamConvention::MutBorrow,
            }],
        }));
        entry_insts.push(Instruction::new(InstKind::EndMutBorrow {
            operand: borrow_val,
        }));
    }

    // Discriminant — non-consuming read of the tag
    let i32_ty = module.ty_arena.find(|t| matches!(t, MirTy::I32))
        .unwrap_or_else(|| module.ty_arena.find(|t| matches!(t, MirTy::I8))
            .expect("integer type should be interned"));
    let disc_val = body.alloc_value(ValueDef::owned(i32_ty));
    entry_insts.push(Instruction::new(InstKind::Discriminant {
        result: disc_val,
        operand: self_val,
    }));

    // Exit block — returns unit
    let exit_block = body.alloc_block();
    let exit_unit = body.alloc_value(ValueDef::owned(unit_ty));
    body.block_mut(exit_block).insts.push(Instruction::new(InstKind::Literal {
        result: exit_unit,
        value: Immediate::unit(),
    }));
    body.block_mut(exit_block).terminator =
        Terminator::new(TerminatorKind::Return(exit_unit));

    // Build per-variant blocks. Each arm receives both self and the discriminant
    // as block args so both @owned values are consumed by forwarding.
    let mut switch_arms = Vec::new();

    for (variant_idx, field_indices) in variants {
        let variant_block = body.alloc_block();

        // Variant block receives self and discriminant via block params
        let variant_self = body.alloc_value(ValueDef::owned(self_ty));
        let variant_disc = body.alloc_value(ValueDef::owned(i32_ty));
        body.block_mut(variant_block).params.push(BlockParam {
            value: variant_self,
            ty: self_ty,
            ownership: Ownership::Owned,
        });
        body.block_mut(variant_block).params.push(BlockParam {
            value: variant_disc,
            ty: i32_ty,
            ownership: Ownership::Owned,
        });

        let mut variant_insts = Vec::new();

        // Destroy the forwarded discriminant — no longer needed.
        variant_insts.push(Instruction::new(InstKind::DestroyValue { operand: variant_disc }));

        // Destructure the enum for this variant. All payload values are @owned
        // since ownership_for_type always returns Owned.
        let case_def = &enum_def.cases[variant_idx.index()];
        let payload_count = case_def.payload_fields.len();
        let droppable_set: std::collections::HashSet<FieldIdx> = field_indices.iter().copied().collect();
        let mut payload_vals = Vec::with_capacity(payload_count);
        for (i, pf) in case_def.payload_fields.iter().enumerate() {
            let fi = FieldIdx::new(i);
            let ownership = if field_indices.contains(&fi) {
                Ownership::Owned
            } else {
                ownership_for_type(pf.ty, &module.ty_arena, module)
            };
            payload_vals.push(body.alloc_value(ValueDef {
                ty: pf.ty,
                ownership,
                borrow_source: None,
            }));
        }
        variant_insts.push(Instruction::new(InstKind::DestructureEnum {
            results: payload_vals.clone(),
            operand: variant_self,
            variant: *variant_idx,
        }));

        // Destroy all payload fields — droppable fields trigger transitive shims,
        // non-droppable fields must still be consumed since all values are @owned.
        for (i, pv) in payload_vals.iter().enumerate() {
            let fi = FieldIdx::new(i);
            let field_ty = case_def.payload_fields[i].ty;
            variant_insts.push(Instruction::new(InstKind::DestroyValue { operand: *pv }));
            if droppable_set.contains(&fi) {
                if let MirTy::Named { entity, .. } = module.ty_arena.get(field_ty) {
                    field_type_entities.push(*entity);
                }
            }
        }

        body.block_mut(variant_block).insts = variant_insts;
        body.block_mut(variant_block).terminator =
            Terminator::new(TerminatorKind::Jump {
                target: exit_block,
                args: vec![],
            });

        switch_arms.push(SwitchArm {
            pattern: crate::SwitchCase::Variant(*variant_idx),
            target: variant_block,
            args: vec![self_val, disc_val],
        });
    }

    // Wildcard block — consumes the enum value without field drops
    let wildcard_block = body.alloc_block();
    let wildcard_self = body.alloc_value(ValueDef::owned(self_ty));
    let wildcard_disc = body.alloc_value(ValueDef::owned(i32_ty));
    body.block_mut(wildcard_block).params.push(BlockParam {
        value: wildcard_self,
        ty: self_ty,
        ownership: Ownership::Owned,
    });
    body.block_mut(wildcard_block).params.push(BlockParam {
        value: wildcard_disc,
        ty: i32_ty,
        ownership: Ownership::Owned,
    });
    body.block_mut(wildcard_block).insts.push(
        Instruction::new(InstKind::DestroyValue { operand: wildcard_disc }),
    );
    body.block_mut(wildcard_block).insts.push(
        Instruction::new(InstKind::DestroyValue { operand: wildcard_self }),
    );
    body.block_mut(wildcard_block).terminator =
        Terminator::new(TerminatorKind::Jump {
            target: exit_block,
            args: vec![],
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

    let mut func = FunctionDef::new(shim_entity, name, unit_ty);
    func.kind = FunctionKind::DropShim { nominal: enum_def.entity };
    func.type_params = enum_def.type_params.clone();
    func.params.push(ParamDef::new("self", self_val, self_ty, ParamConvention::Consuming));
    func.body = Some(body);

    (func, field_type_entities)
}

/// Replace placeholder callee entities in drop shim Call instructions.
fn patch_shim_callees(module: &mut MirModule, shim_map: &HashMap<Entity, usize>) {
    let entity_map: HashMap<Entity, Entity> = shim_map
        .iter()
        .map(|(&type_entity, &func_idx)| (type_entity, module.functions[func_idx].entity))
        .collect();

    for func in &mut module.functions {
        if !matches!(func.kind, FunctionKind::DropShim { .. }) {
            continue;
        }
        let Some(body) = func.body.as_mut() else {
            continue;
        };
        for block in &mut body.blocks {
            for inst in &mut block.insts {
                if let InstKind::Call {
                    callee: Callee::Direct { func: callee_entity, .. },
                    ..
                } = &mut inst.kind
                    && let Some(&shim_entity) = entity_map.get(callee_entity)
                {
                    *callee_entity = shim_entity;
                }
            }
        }
    }
}

/// Find the drop shim function for a given type entity, if one exists.
pub fn find_drop_shim(module: &MirModule, type_entity: Entity) -> Option<Entity> {
    module
        .functions
        .iter()
        .find(|f| matches!(f.kind, FunctionKind::DropShim { nominal } if nominal == type_entity))
        .map(|f| f.entity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::enum_def::{EnumCaseDef, EnumDef};
    use crate::item::struct_def::{FieldDef, StructDef};
    use crate::item::{CopyBehavior, DropBehavior, TypeInfo};
    use crate::{FieldIdx, VariantIdx};

    fn find_shim_by_name<'a>(module: &'a MirModule, name_substr: &str) -> &'a FunctionDef {
        module
            .functions
            .iter()
            .find(|f| f.name.contains(name_substr))
            .unwrap_or_else(|| panic!("no shim containing '{name_substr}'"))
    }

    fn verify_shim(module: &MirModule, func: &FunctionDef) {
        let body = func.body.as_ref().unwrap();
        let errors = crate::verify::verify_ossa(body, module, &func.name, func.entity);
        assert!(errors.is_empty(), "verifier errors in {}: {:?}", func.name, errors);
    }

    // ---- Struct with droppable fields, no deinit ----

    #[test]
    fn struct_field_drop_shim() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();

        let inner_entity = Entity::from_raw(1);
        module.register_name(inner_entity, "Inner");
        let inner_ty = module.ty_arena.named(inner_entity, vec![]);
        let mut inner_def = StructDef::new(inner_entity, "Inner");
        inner_def.add_field(FieldDef::new("data", i64_ty));
        inner_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop { deinit: None, fields: vec![] },
            layout: None,
        };
        module.add_struct(inner_def);

        let outer_entity = Entity::from_raw(2);
        module.register_name(outer_entity, "Outer");
        let _outer_ty = module.ty_arena.named(outer_entity, vec![]);
        let mut outer_def = StructDef::new(outer_entity, "Outer");
        outer_def.add_field(FieldDef::new("value", inner_ty));
        outer_def.add_field(FieldDef::new("count", i64_ty));
        outer_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop {
                deinit: None,
                fields: vec![FieldIdx::new(0)],
            },
            layout: None,
        };
        module.add_struct(outer_def);

        let mut next_entity = 100;
        synthesize_drop_shims(&mut module, &mut next_entity);

        // Should have shims for both Inner and Outer
        let outer_shim = find_shim_by_name(&module, "__drop$Outer");
        assert!(matches!(
            outer_shim.kind,
            FunctionKind::DropShim { nominal } if nominal == outer_entity
        ));
        let body = outer_shim.body.as_ref().unwrap();
        // Should have: DestructureStruct + DestroyValue(field 0) + Literal(unit)
        let has_destructure = body.blocks[0].insts.iter()
            .any(|i| matches!(&i.kind, InstKind::DestructureStruct { .. }));
        let has_destroy = body.blocks[0].insts.iter()
            .any(|i| matches!(&i.kind, InstKind::DestroyValue { .. }));
        assert!(has_destructure, "should have DestructureStruct");
        assert!(has_destroy, "should have DestroyValue for droppable field");

        verify_shim(&module, outer_shim);

        // Inner shim should also exist and verify
        let inner_shim = find_shim_by_name(&module, "__drop$Inner");
        assert!(inner_shim.body.is_some());
        verify_shim(&module, inner_shim);
    }

    // ---- Struct with deinit + fields ----

    #[test]
    fn struct_deinit_then_field_drops() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();

        let inner_entity = Entity::from_raw(1);
        module.register_name(inner_entity, "Inner");
        let inner_ty = module.ty_arena.named(inner_entity, vec![]);
        let mut inner_def = StructDef::new(inner_entity, "Inner");
        inner_def.add_field(FieldDef::new("data", i64_ty));
        inner_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop { deinit: None, fields: vec![] },
            layout: None,
        };
        module.add_struct(inner_def);

        let deinit_entity = Entity::from_raw(2);
        module.register_name(deinit_entity, "MyType.deinit");

        let my_entity = Entity::from_raw(3);
        module.register_name(my_entity, "MyType");
        let my_ty = module.ty_arena.named(my_entity, vec![]);
        // Pre-intern pointer type for BeginMutBorrow
        module.ty_arena.pointer(my_ty);
        let mut my_def = StructDef::new(my_entity, "MyType");
        my_def.add_field(FieldDef::new("inner", inner_ty));
        my_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop {
                deinit: Some(deinit_entity),
                fields: vec![FieldIdx::new(0)],
            },
            layout: None,
        };
        module.add_struct(my_def);

        // Add a stub FunctionDef for the deinit so the module is consistent
        let deinit_unit = module.ty_arena.unit();
        module.add_function(FunctionDef::new(deinit_entity, "MyType.deinit", deinit_unit));

        let mut next_entity = 100;
        synthesize_drop_shims(&mut module, &mut next_entity);

        let shim = find_shim_by_name(&module, "__drop$MyType");
        let body = shim.body.as_ref().unwrap();

        // Should have: BeginMutBorrow, Call(deinit), EndMutBorrow, DestructureStruct, DestroyValue, Literal
        let has_borrow = body.blocks[0].insts.iter()
            .any(|i| matches!(&i.kind, InstKind::BeginMutBorrow { .. }));
        let has_call = body.blocks[0].insts.iter()
            .any(|i| matches!(&i.kind, InstKind::Call { callee: Callee::Direct { func, .. }, .. } if *func == deinit_entity));
        let has_end_borrow = body.blocks[0].insts.iter()
            .any(|i| matches!(&i.kind, InstKind::EndMutBorrow { .. }));
        assert!(has_borrow, "should have BeginMutBorrow");
        assert!(has_call, "should have deinit Call");
        assert!(has_end_borrow, "should have EndMutBorrow");

        verify_shim(&module, shim);
    }

    // ---- Enum with per-variant drops ----

    #[test]
    fn enum_variant_drop_shim() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();
        // Pre-intern i32 for discriminant
        module.ty_arena.i32();

        let inner_entity = Entity::from_raw(1);
        module.register_name(inner_entity, "Inner");
        let inner_ty = module.ty_arena.named(inner_entity, vec![]);
        let mut inner_def = StructDef::new(inner_entity, "Inner");
        inner_def.add_field(FieldDef::new("data", i64_ty));
        inner_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop { deinit: None, fields: vec![] },
            layout: None,
        };
        module.add_struct(inner_def);

        let enum_entity = Entity::from_raw(2);
        module.register_name(enum_entity, "Result");
        let _enum_ty = module.ty_arena.named(enum_entity, vec![]);
        let mut enum_def = EnumDef::new(enum_entity, "Result");
        enum_def.add_case(EnumCaseDef::with_payload(
            "Ok", 0,
            vec![FieldDef::new("value", inner_ty)],
        ));
        enum_def.add_case(EnumCaseDef::new("Err", 1));
        enum_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::EnumDrop {
                deinit: None,
                variants: vec![
                    (VariantIdx::new(0), vec![FieldIdx::new(0)]),
                ],
            },
            layout: None,
        };
        module.add_enum(enum_def);

        let mut next_entity = 100;
        synthesize_drop_shims(&mut module, &mut next_entity);

        let shim = find_shim_by_name(&module, "__drop$Result");
        let body = shim.body.as_ref().unwrap();
        // Entry block has Switch terminator
        assert!(matches!(
            body.blocks[0].terminator.kind,
            TerminatorKind::Switch { .. }
        ));
        // Should have blocks: entry, Ok variant, wildcard, exit
        assert!(body.blocks.len() >= 3);

        verify_shim(&module, shim);
    }

    // ---- Transitive: A has field B, both need shims ----

    #[test]
    fn transitive_shim_generation() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();

        let c_entity = Entity::from_raw(1);
        module.register_name(c_entity, "C");
        let c_ty = module.ty_arena.named(c_entity, vec![]);
        let mut c_def = StructDef::new(c_entity, "C");
        c_def.add_field(FieldDef::new("val", i64_ty));
        c_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop { deinit: None, fields: vec![] },
            layout: None,
        };
        module.add_struct(c_def);

        let b_entity = Entity::from_raw(2);
        module.register_name(b_entity, "B");
        let b_ty = module.ty_arena.named(b_entity, vec![]);
        let mut b_def = StructDef::new(b_entity, "B");
        b_def.add_field(FieldDef::new("c", c_ty));
        b_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop {
                deinit: None,
                fields: vec![FieldIdx::new(0)],
            },
            layout: None,
        };
        module.add_struct(b_def);

        let a_entity = Entity::from_raw(3);
        module.register_name(a_entity, "A");
        let _a_ty = module.ty_arena.named(a_entity, vec![]);
        let mut a_def = StructDef::new(a_entity, "A");
        a_def.add_field(FieldDef::new("b", b_ty));
        a_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop {
                deinit: None,
                fields: vec![FieldIdx::new(0)],
            },
            layout: None,
        };
        module.add_struct(a_def);

        let mut next_entity = 100;
        synthesize_drop_shims(&mut module, &mut next_entity);

        assert!(find_shim_by_name(&module, "__drop$A").body.is_some());
        assert!(find_shim_by_name(&module, "__drop$B").body.is_some());
        assert!(find_shim_by_name(&module, "__drop$C").body.is_some());

        for func in &module.functions {
            if matches!(func.kind, FunctionKind::DropShim { .. }) {
                verify_shim(&module, func);
            }
        }
    }

    // ---- No drop needed → no shim ----

    #[test]
    fn no_drop_no_shim() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();

        let point_entity = Entity::from_raw(1);
        module.register_name(point_entity, "Point");
        let _point_ty = module.ty_arena.named(point_entity, vec![]);
        let mut point_def = StructDef::new(point_entity, "Point");
        point_def.add_field(FieldDef::new("x", i64_ty));
        point_def.type_info = TypeInfo::bitwise();
        module.add_struct(point_def);

        let func_count_before = module.functions.len();
        let mut next_entity = 100;
        synthesize_drop_shims(&mut module, &mut next_entity);
        assert_eq!(module.functions.len(), func_count_before);
    }

    // ---- Deinit only, no field drops ----

    #[test]
    fn deinit_only_shim() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();

        let deinit_entity = Entity::from_raw(1);
        let s_entity = Entity::from_raw(2);
        module.register_name(s_entity, "Handle");
        let s_ty = module.ty_arena.named(s_entity, vec![]);
        // Pre-intern pointer type
        module.ty_arena.pointer(s_ty);
        let mut s_def = StructDef::new(s_entity, "Handle");
        s_def.add_field(FieldDef::new("fd", i64_ty));
        s_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop {
                deinit: Some(deinit_entity),
                fields: vec![],
            },
            layout: None,
        };
        module.add_struct(s_def);

        // Stub deinit function
        let unit_ty = module.ty_arena.unit();
        module.add_function(FunctionDef::new(deinit_entity, "Handle.deinit", unit_ty));

        let mut next_entity = 100;
        synthesize_drop_shims(&mut module, &mut next_entity);

        let shim = find_shim_by_name(&module, "__drop$Handle");
        let body = shim.body.as_ref().unwrap();
        // Should have: BeginMutBorrow, Call(deinit), EndMutBorrow, DestructureStruct, Literal(unit)
        let has_call = body.blocks[0].insts.iter()
            .any(|i| matches!(&i.kind, InstKind::Call { callee: Callee::Direct { func, .. }, .. } if *func == deinit_entity));
        assert!(has_call, "should have deinit call");

        verify_shim(&module, shim);
    }
}
