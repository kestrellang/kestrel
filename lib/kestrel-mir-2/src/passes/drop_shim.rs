use std::collections::HashMap;

use kestrel_hecs::Entity;

use crate::body::{BasicBlock, LocalDef, MirBody};
use crate::item::function::{FunctionDef, FunctionKind, ParamDef};
use crate::operand::{ArgMode, Operand};
use crate::place::Place;
use crate::statement::{Callee, Statement, StatementKind};
use crate::terminator::{SwitchCase, Terminator};
use crate::ty::{MirTy, ParamConvention};
use crate::ty_query::needs_drop;
use crate::{DropBehavior, MirModule, TyId};

/// Synthesize `__drop$T` functions for all types that need cleanup.
/// Adds the generated functions to `module.functions`.
/// `next_entity` is bumped for each allocated entity (function + type params).
pub fn synthesize_drop_shims(module: &mut MirModule, next_entity: &mut u32) {
    // Ensure unit type is interned (shims return unit)
    module.ty_arena.unit();

    // Pre-intern Named types for all structs/enums so shim generation can find them.
    // For generic types, intern with TypeParam type_args so shim bodies are generic.
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

    // Collect types that need shims
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

    // Fixed-point: generate shims, discover transitive needs
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

        // Enqueue field types that also need shims
        for field_entity in field_type_entities {
            if !shim_map.contains_key(&field_entity) {
                worklist.push(field_entity);
            }
        }
    }

    // Patch shim bodies: replace placeholder callee entities with actual shim entities
    patch_shim_callees(module, &shim_map);
}

fn generate_shim(
    module: &MirModule,
    type_entity: Entity,
    shim_entity: Entity,
    next_entity: &mut u32,
) -> (FunctionDef, Vec<Entity>) {
    // Find the type def
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

    // Build self_ty: Named with TypeParam type_args for generic types
    let tp_ty_ids: Vec<TyId> = struct_def.type_params.iter()
        .map(|tp| module.ty_arena.find(|t| matches!(t, MirTy::TypeParam(e) if *e == tp.entity))
            .expect("TypeParam should be interned"))
        .collect();
    let self_ty = module.ty_arena.find(|t| {
        matches!(t, MirTy::Named { entity, type_args }
            if *entity == struct_def.entity && *type_args == tp_ty_ids)
    }).expect("struct type should be interned");

    let unit_ty = find_or_intern_unit(module);

    let mut body = MirBody::new();
    let self_local = body.add_local(LocalDef::new("self", self_ty));
    body.param_count = 1;

    let entry = body.add_block(BasicBlock::new());

    let DropBehavior::StructDrop { deinit, fields } = &struct_def.type_info.drop else {
        panic!("expected StructDrop");
    };

    let mut stmts = Vec::new();
    let mut field_type_entities = Vec::new();

    // 1. Call user-defined deinit if any — pass the struct's type params
    //    so monomorphization substitutes them with concrete types
    if let Some(deinit_entity) = deinit {
        stmts.push(Statement::new(StatementKind::Call {
            dest: None,
            callee: Callee::direct_with_args(*deinit_entity, tp_ty_ids.clone(), None),
            args: vec![(Operand::Place(Place::local(self_local)), ArgMode::RefMut)],
        }));
    }

    // 2. Drop fields that need cleanup. Keep these as MIR Drop statements so
    // monomorphization can resolve generic fields to either concrete shims or
    // no-ops.
    for &field_idx in fields {
        let field_ty = struct_def.fields[field_idx.index()].ty;
        if needs_drop(&module.ty_arena, module, field_ty) {
            stmts.push(Statement::new(StatementKind::Drop {
                place: Place::local(self_local).field(field_idx),
            }));
            if let MirTy::Named { entity, .. } = module.ty_arena.get(field_ty) {
                field_type_entities.push(*entity);
            }
        }
    }

    body.block_mut(entry).stmts = stmts;
    body.block_mut(entry).terminator =
        Terminator::ret(Operand::Const(crate::Immediate::unit()));

    let mut func = FunctionDef::new(shim_entity, name, unit_ty);
    func.kind = FunctionKind::DropShim {
        nominal: struct_def.entity,
    };
    // Copy type_params from parent struct so the shim is generic
    func.type_params = struct_def.type_params.clone();
    func.params.push(ParamDef::new("self", self_local, self_ty, ParamConvention::Consuming));
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

    // Build self_ty: Named with TypeParam type_args for generic types
    let tp_ty_ids: Vec<TyId> = enum_def.type_params.iter()
        .map(|tp| module.ty_arena.find(|t| matches!(t, MirTy::TypeParam(e) if *e == tp.entity))
            .expect("TypeParam should be interned"))
        .collect();
    let self_ty = module.ty_arena.find(|t| {
        matches!(t, MirTy::Named { entity, type_args }
            if *entity == enum_def.entity && *type_args == tp_ty_ids)
    }).expect("enum type should be interned");

    let unit_ty = find_or_intern_unit(module);

    let mut body = MirBody::new();
    let self_local = body.add_local(LocalDef::new("self", self_ty));
    body.param_count = 1;

    let DropBehavior::EnumDrop { deinit, variants } = &enum_def.type_info.drop else {
        panic!("expected EnumDrop");
    };

    let mut field_type_entities = Vec::new();

    let entry = body.add_block(BasicBlock::new());

    // Optional deinit call — pass type params so monomorphization substitutes them
    let mut entry_stmts = Vec::new();
    if let Some(deinit_entity) = deinit {
        entry_stmts.push(Statement::new(StatementKind::Call {
            dest: None,
            callee: Callee::direct_with_args(*deinit_entity, tp_ty_ids.clone(), None),
            args: vec![(Operand::Place(Place::local(self_local)), ArgMode::RefMut)],
        }));
    }

    // Build switch cases + per-variant blocks
    let exit_block = body.add_block(BasicBlock::new());
    body.block_mut(exit_block).terminator =
        Terminator::ret(Operand::Const(crate::Immediate::unit()));

    let mut switch_cases = Vec::new();

    for (variant_idx, field_indices) in variants {
        let variant_block = body.add_block(BasicBlock::new());
        let mut variant_stmts = Vec::new();

        for &field_idx in field_indices {
            let case_def = &enum_def.cases[variant_idx.index()];
            let field_ty = case_def.payload_fields[field_idx.index()].ty;
            if needs_drop(&module.ty_arena, module, field_ty) {
                variant_stmts.push(Statement::new(StatementKind::Drop {
                    place: Place::local(self_local)
                        .downcast(*variant_idx)
                        .field(field_idx),
                }));
                if let MirTy::Named { entity, .. } = module.ty_arena.get(field_ty) {
                    field_type_entities.push(*entity);
                }
            }
        }

        body.block_mut(variant_block).stmts = variant_stmts;
        body.block_mut(variant_block).terminator = Terminator::jump(exit_block);
        switch_cases.push((SwitchCase::Variant(*variant_idx), variant_block));
    }

    // Wildcard for variants with no drops
    switch_cases.push((SwitchCase::Wildcard, exit_block));

    body.block_mut(entry).stmts = entry_stmts;
    body.block_mut(entry).terminator =
        Terminator::switch(Place::local(self_local), switch_cases);

    let mut func = FunctionDef::new(shim_entity, name, unit_ty);
    func.kind = FunctionKind::DropShim {
        nominal: enum_def.entity,
    };
    // Copy type_params from parent enum so the shim is generic
    func.type_params = enum_def.type_params.clone();
    func.params.push(ParamDef::new("self", self_local, self_ty, ParamConvention::Consuming));
    func.body = Some(body);

    (func, field_type_entities)
}

/// Replace placeholder callee entities (type entities) with actual shim function entities.
fn patch_shim_callees(module: &mut MirModule, shim_map: &HashMap<Entity, usize>) {
    // Build type_entity → shim_entity mapping first to avoid borrow conflict
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
            for stmt in &mut block.stmts {
                if let StatementKind::Call {
                    callee: Callee::Direct { func: callee_entity, .. },
                    ..
                } = &mut stmt.kind
                    && let Some(&shim_entity) = entity_map.get(callee_entity)
                {
                    *callee_entity = shim_entity;
                }
            }
        }
    }
}

fn find_or_intern_unit(module: &MirModule) -> TyId {
    module
        .ty_arena
        .find(|t| matches!(t, MirTy::Tuple(elems) if elems.is_empty()))
        .expect("unit type should be interned")
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
    use crate::terminator::TerminatorKind;
    use crate::{FieldIdx, VariantIdx};
    use crate::builder::ModuleBuilder;
    use crate::item::enum_def::{EnumCaseDef, EnumDef};
    use crate::item::struct_def::{FieldDef, StructDef};
    use crate::item::{CopyBehavior, DropBehavior, TypeInfo};

    fn find_shim_by_name<'a>(module: &'a MirModule, name_substr: &str) -> &'a FunctionDef {
        module
            .functions
            .iter()
            .find(|f| f.name.contains(name_substr))
            .unwrap_or_else(|| panic!("no shim containing '{name_substr}'"))
    }

    // ---- Struct with droppable fields, no deinit ----

    #[test]
    fn struct_field_drop_shim() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();

        // Inner type that needs dropping
        let inner_entity = m.fresh_entity();
        m.register_name(inner_entity, "Inner");
        let inner_ty = m.named(inner_entity, vec![]);
        let mut inner_def = StructDef::new(inner_entity, "Inner");
        inner_def.add_field(FieldDef::new("data", i64_ty));
        inner_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop {
                deinit: None,
                fields: vec![],
            },
            layout: None,
        };
        m.add_struct(inner_def);

        // Outer type with Inner field
        let outer_entity = m.fresh_entity();
        m.register_name(outer_entity, "Outer");
        let _outer_ty = m.named(outer_entity, vec![]);
        let mut outer_def = StructDef::new(outer_entity, "Outer");
        outer_def.add_field(FieldDef::new("value", inner_ty));
        outer_def.add_field(FieldDef::new("count", i64_ty));
        outer_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop {
                deinit: None,
                fields: vec![FieldIdx::new(0)], // only 'value' needs drop
            },
            layout: None,
        };
        m.add_struct(outer_def);

        let mut module = m.finish();
        let mut next_entity = 100;
        synthesize_drop_shims(&mut module, &mut next_entity);

        // Should have shims for both Inner and Outer
        let outer_shim = find_shim_by_name(&module, "__drop$Outer");
        assert!(matches!(
            outer_shim.kind,
            FunctionKind::DropShim { nominal } if nominal == outer_entity
        ));
        let body = outer_shim.body.as_ref().unwrap();
        // Should have 1 statement: drop self.0. Mono expansion resolves the
        // concrete field shim after generic substitution.
        assert_eq!(body.blocks[0].stmts.len(), 1);
        assert_eq!(
            body.blocks[0].stmts[0].kind,
            StatementKind::Drop {
                place: Place::local(crate::LocalId::new(0)).field(FieldIdx::new(0)),
            }
        );

        // Inner shim should also exist
        assert!(find_shim_by_name(&module, "__drop$Inner")
            .body
            .is_some());
    }

    // ---- Struct with deinit + fields ----

    #[test]
    fn struct_deinit_then_field_drops() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();

        let inner_entity = m.fresh_entity();
        m.register_name(inner_entity, "Inner");
        let inner_ty = m.named(inner_entity, vec![]);
        let mut inner_def = StructDef::new(inner_entity, "Inner");
        inner_def.add_field(FieldDef::new("data", i64_ty));
        inner_def.type_info.drop = DropBehavior::StructDrop {
            deinit: None,
            fields: vec![],
        };
        m.add_struct(inner_def);

        let deinit_entity = m.fresh_entity();
        m.register_name(deinit_entity, "MyType.deinit");

        let my_entity = m.fresh_entity();
        m.register_name(my_entity, "MyType");
        let _my_ty = m.named(my_entity, vec![]);
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
        m.add_struct(my_def);

        let mut module = m.finish();
        let mut next_entity = 100;
        synthesize_drop_shims(&mut module, &mut next_entity);

        let shim = find_shim_by_name(&module, "__drop$MyType");
        let body = shim.body.as_ref().unwrap();
        // 2 statements: deinit call + projected field drop
        assert_eq!(body.blocks[0].stmts.len(), 2);

        // First: deinit call with RefMut
        match &body.blocks[0].stmts[0].kind {
            StatementKind::Call { callee: Callee::Direct { func, .. }, args, .. } => {
                assert_eq!(*func, deinit_entity);
                assert_eq!(args[0].1, ArgMode::RefMut);
            }
            other => panic!("expected deinit call, got {other:?}"),
        }

        assert_eq!(
            body.blocks[0].stmts[1].kind,
            StatementKind::Drop {
                place: Place::local(crate::LocalId::new(0)).field(FieldIdx::new(0)),
            }
        );
    }

    // ---- Enum with per-variant drops ----

    #[test]
    fn enum_variant_drop_shim() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();

        let inner_entity = m.fresh_entity();
        m.register_name(inner_entity, "Inner");
        let inner_ty = m.named(inner_entity, vec![]);
        let mut inner_def = StructDef::new(inner_entity, "Inner");
        inner_def.add_field(FieldDef::new("data", i64_ty));
        inner_def.type_info.drop = DropBehavior::StructDrop {
            deinit: None,
            fields: vec![],
        };
        m.add_struct(inner_def);

        let enum_entity = m.fresh_entity();
        m.register_name(enum_entity, "Result");
        let _enum_ty = m.named(enum_entity, vec![]);
        let mut enum_def = EnumDef::new(enum_entity, "Result");
        enum_def.add_case(EnumCaseDef::with_payload(
            "Ok",
            0,
            vec![FieldDef::new("value", inner_ty)],
        ));
        enum_def.add_case(EnumCaseDef::new("Err", 1));
        enum_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::EnumDrop {
                deinit: None,
                variants: vec![
                    (VariantIdx::new(0), vec![FieldIdx::new(0)]), // Ok.value needs drop
                ],
            },
            layout: None,
        };
        m.add_enum(enum_def);

        let mut module = m.finish();
        let mut next_entity = 100;
        synthesize_drop_shims(&mut module, &mut next_entity);

        let shim = find_shim_by_name(&module, "__drop$Result");
        let body = shim.body.as_ref().unwrap();
        // Entry block has switch terminator
        assert!(matches!(
            body.blocks[0].terminator.kind,
            TerminatorKind::Switch { .. }
        ));
        // Should have blocks: entry (switch), Ok variant, exit, possibly Err→exit
        assert!(body.blocks.len() >= 3);
    }

    // ---- Transitive: A has field B, both need shims ----

    #[test]
    fn transitive_shim_generation() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();

        // C (leaf droppable)
        let c_entity = m.fresh_entity();
        m.register_name(c_entity, "C");
        let c_ty = m.named(c_entity, vec![]);
        let mut c_def = StructDef::new(c_entity, "C");
        c_def.add_field(FieldDef::new("val", i64_ty));
        c_def.type_info.drop = DropBehavior::StructDrop {
            deinit: None,
            fields: vec![],
        };
        m.add_struct(c_def);

        // B has field of type C
        let b_entity = m.fresh_entity();
        m.register_name(b_entity, "B");
        let b_ty = m.named(b_entity, vec![]);
        let mut b_def = StructDef::new(b_entity, "B");
        b_def.add_field(FieldDef::new("c", c_ty));
        b_def.type_info.drop = DropBehavior::StructDrop {
            deinit: None,
            fields: vec![FieldIdx::new(0)],
        };
        m.add_struct(b_def);

        // A has field of type B
        let a_entity = m.fresh_entity();
        m.register_name(a_entity, "A");
        let _a_ty = m.named(a_entity, vec![]);
        let mut a_def = StructDef::new(a_entity, "A");
        a_def.add_field(FieldDef::new("b", b_ty));
        a_def.type_info.drop = DropBehavior::StructDrop {
            deinit: None,
            fields: vec![FieldIdx::new(0)],
        };
        m.add_struct(a_def);

        let mut module = m.finish();
        let mut next_entity = 100;
        synthesize_drop_shims(&mut module, &mut next_entity);

        // All three should have shims
        assert!(find_shim_by_name(&module, "__drop$A").body.is_some());
        assert!(find_shim_by_name(&module, "__drop$B").body.is_some());
        assert!(find_shim_by_name(&module, "__drop$C").body.is_some());
    }

    // ---- No drop needed → no shim ----

    #[test]
    fn no_drop_no_shim() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();

        let point_entity = m.fresh_entity();
        m.register_name(point_entity, "Point");
        let _point_ty = m.named(point_entity, vec![]);
        let mut point_def = StructDef::new(point_entity, "Point");
        point_def.add_field(FieldDef::new("x", i64_ty));
        point_def.type_info = TypeInfo::bitwise();
        m.add_struct(point_def);

        let mut module = m.finish();
        let func_count_before = module.functions.len();
        let mut next_entity = 100;
        synthesize_drop_shims(&mut module, &mut next_entity);
        assert_eq!(module.functions.len(), func_count_before);
    }

    // ---- Deinit only, no field drops ----

    #[test]
    fn deinit_only_shim() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();

        let deinit_entity = m.fresh_entity();
        let s_entity = m.fresh_entity();
        m.register_name(s_entity, "Handle");
        let _s_ty = m.named(s_entity, vec![]);
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
        m.add_struct(s_def);

        let mut module = m.finish();
        let mut next_entity = 100;
        synthesize_drop_shims(&mut module, &mut next_entity);

        let shim = find_shim_by_name(&module, "__drop$Handle");
        let body = shim.body.as_ref().unwrap();
        // 1 statement: deinit call only
        assert_eq!(body.blocks[0].stmts.len(), 1);
        match &body.blocks[0].stmts[0].kind {
            StatementKind::Call { callee: Callee::Direct { func, .. }, .. } => {
                assert_eq!(*func, deinit_entity);
            }
            other => panic!("expected deinit call, got {other:?}"),
        }
    }

    // ---- Field cleanup stays projected until mono drop expansion ----

    #[test]
    fn shim_field_cleanup_uses_projected_drop() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();

        let inner_entity = m.fresh_entity();
        m.register_name(inner_entity, "Inner");
        let inner_ty = m.named(inner_entity, vec![]);
        let mut inner_def = StructDef::new(inner_entity, "Inner");
        inner_def.add_field(FieldDef::new("data", i64_ty));
        inner_def.type_info.drop = DropBehavior::StructDrop {
            deinit: None,
            fields: vec![],
        };
        m.add_struct(inner_def);

        let outer_entity = m.fresh_entity();
        m.register_name(outer_entity, "Outer");
        let _outer_ty = m.named(outer_entity, vec![]);
        let mut outer_def = StructDef::new(outer_entity, "Outer");
        outer_def.add_field(FieldDef::new("inner", inner_ty));
        outer_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop {
                deinit: None,
                fields: vec![FieldIdx::new(0)],
            },
            layout: None,
        };
        m.add_struct(outer_def);

        let mut module = m.finish();
        let mut next_entity = 100;
        synthesize_drop_shims(&mut module, &mut next_entity);

        // The outer shim keeps a generic projected Drop. Mono expansion resolves
        // the concrete inner shim from the projected place type.
        assert!(find_drop_shim(&module, inner_entity).is_some());
        let outer_shim = find_shim_by_name(&module, "__drop$Outer");
        let body = outer_shim.body.as_ref().unwrap();
        assert_eq!(
            body.blocks[0].stmts[0].kind,
            StatementKind::Drop {
                place: Place::local(crate::LocalId::new(0)).field(FieldIdx::new(0)),
            }
        );
    }
}
