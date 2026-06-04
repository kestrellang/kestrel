use std::collections::HashMap;

use kestrel_hecs::Entity;

use crate::block::BlockParam;
use crate::body::OssaBody;
use crate::inst::{InstKind, Instruction};
use crate::item::CopyBehavior;
use crate::item::function::{FunctionDef, FunctionKind, ParamDef};
use crate::item::witness::{WitnessDef, WitnessMethodBinding, WitnessMethodKey};
use crate::terminator::{SwitchArm, Terminator, TerminatorKind};
use crate::ty::TyArena;
use crate::ty::{MirTy, ParamConvention};
use crate::ty_query::find_cloneable_protocol;
use crate::value::{Ownership, ValueDef};
use crate::{FieldIdx, MirModule, TyId, ValueId, VariantIdx};

/// Synthesize `__clone$T` functions for all structs/enums that aren't `not Copyable`.
/// Skips types that already have a user-written `.clone()` via a Cloneable witness.
/// Registers each synthesized shim as a Cloneable witness.
///
/// Exception: a *conditionally* Copyable container (`: not Copyable` + `extend …:
/// Copyable where T: Copyable`) has a generic base of `None`, but its concrete
/// instances may be `Clone` (e.g. `Optional[String]`). Such a type needs a
/// generic `__clone$T` shim so that those instances get a real clone (the per-
/// instance `CopyValue` in its body resolves to a bitwise copy or a payload
/// clone during expand). We synthesize the shim but leave the generic base
/// `None` and register no unconditional witness — the conformance stays
/// per-instance via the stdlib's `where`-gated extension. (See the
/// `Optional[String]` if-let double-free: an unsynthesized shim left the
/// scrutinee copy as a non-refcounting bit-copy that was then double-dropped.)
pub fn synthesize_clone_shims(module: &mut MirModule, next_entity: &mut u32) {
    let Some(cloneable_proto) = find_cloneable_protocol(module) else {
        return;
    };

    // Pre-intern Pointer(Named(entity)) for all candidate types (needed by BeginBorrow)
    for s in module.structs.values() {
        if s.type_info.copy == CopyBehavior::None && s.conditionally_copyable.is_empty() {
            continue;
        }
        let tp_ty_ids: Vec<TyId> = s
            .type_params
            .iter()
            .map(|tp| module.ty_arena.intern(MirTy::TypeParam(tp.entity)))
            .collect();
        let named_ty = module.ty_arena.intern(MirTy::Named {
            entity: s.entity,
            type_args: tp_ty_ids,
        });
        module.ty_arena.pointer(named_ty);
    }
    for e in module.enums.values() {
        if e.type_info.copy == CopyBehavior::None && e.conditionally_copyable.is_empty() {
            continue;
        }
        let tp_ty_ids: Vec<TyId> = e
            .type_params
            .iter()
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
    let has_user_clone: std::collections::HashSet<Entity> = module
        .witnesses
        .iter()
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
    let closure_env_entities: std::collections::HashSet<Entity> = module
        .functions
        .values()
        .filter_map(|f| match &f.kind {
            FunctionKind::ClosureCall { env_struct } => Some(*env_struct),
            _ => None,
        })
        .collect();

    // A conditionally-Copyable container has a generic base of `None` but needs
    // a shim for its `Clone` instances — don't let the `None` base skip it.
    let mut worklist: Vec<Entity> = Vec::new();
    for s in module.structs.values() {
        if has_user_clone.contains(&s.entity)
            || closure_env_entities.contains(&s.entity)
            || (s.type_info.copy == CopyBehavior::None && s.conditionally_copyable.is_empty())
            || has_unresolvable_fields_struct(s, &module.ty_arena)
        {
            continue;
        }
        worklist.push(s.entity);
    }
    for e in module.enums.values() {
        if has_user_clone.contains(&e.entity)
            || (e.type_info.copy == CopyBehavior::None && e.conditionally_copyable.is_empty())
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

    // Register each shim as a Cloneable witness. A conditionally-Copyable
    // container is skipped: its conformance is per-instance (the stdlib's
    // `where T: Copyable` extension), so an unconditional witness here would
    // wrongly claim e.g. `Optional[File]: Cloneable`. The implicit-copy path
    // finds the shim by `FunctionKind::CloneShim`, not via this witness.
    for (&type_entity, &shim_entity) in &shim_map {
        if is_conditional_container(module, type_entity) {
            continue;
        }
        let func = &module.functions[&shim_entity];
        let tp_ty_ids: Vec<TyId> = func
            .type_params
            .iter()
            .map(|tp| {
                module
                    .ty_arena
                    .find(|t| matches!(t, MirTy::TypeParam(e) if *e == tp.entity))
                    .expect("TypeParam should be interned")
            })
            .collect();
        let self_ty = module
            .ty_arena
            .find(|t| {
                matches!(t, MirTy::Named { entity, type_args }
                if *entity == type_entity && *type_args == tp_ty_ids)
            })
            .expect("Named type should be interned");

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
    // A conditionally-Copyable container keeps its generic `None` base: its
    // per-instance behavior is derived later by `refine_mono_copy_behavior`
    // (`Optional[String]` → Clone, `Optional[Int64]` → Bitwise, `Optional[File]`
    // → None). Overwriting the base to Clone here would break that invariant.
    for &type_entity in shim_map.keys() {
        if is_conditional_container(module, type_entity) {
            continue;
        }
        // Compute predicate with shared borrow, then mutate separately
        let needs_struct = module
            .structs
            .get(&type_entity)
            .map(|s| needs_clone_shim_struct(s, &module.ty_arena))
            .unwrap_or(false);
        if needs_struct {
            module.structs.get_mut(&type_entity).unwrap().type_info.copy =
                CopyBehavior::Clone(cloneable_proto);
            continue;
        }

        let needs_enum = module
            .enums
            .get(&type_entity)
            .map(|e| needs_clone_shim_enum(e, &module.ty_arena))
            .unwrap_or(false);
        if needs_enum {
            module.enums.get_mut(&type_entity).unwrap().type_info.copy =
                CopyBehavior::Clone(cloneable_proto);
        }
    }

    // Explicit-clone types (user-written `clone()`) are skipped by shim
    // synthesis above, so they never had `CopyBehavior::Clone` set — leaving a
    // non-trivial one as `None`/`Bitwise`. The expand pass then degrades a
    // `CopyValue` on such a type to a bitwise alias / move instead of a clone,
    // corrupting a heap-backed value stored in a generic container (e.g. quill's
    // `Value` as a `Dictionary[String, Value]` value: its bucket clone shim
    // bit-copied the `Value`, aliasing the inner String). Mark them `Clone` so
    // the CopyValue expands to their user `clone()`.
    //
    // Include types with a `.clone` method even if no Cloneable *witness* is
    // recorded in `module.witnesses` — a `clone()` defined in an `extend` block
    // (vs inline) doesn't always surface a witness here, but it must still be
    // treated as Clone-behavior. Same trivial-field / conditional-container
    // guards as the shim loop.
    let user_clone_types: std::collections::HashSet<Entity> = has_user_clone
        .iter()
        .copied()
        .chain(
            module
                .functions
                .values()
                .filter_map(|f| f.clone_method_self_nominal(&module.ty_arena)),
        )
        .collect();
    for &type_entity in &user_clone_types {
        if is_conditional_container(module, type_entity) {
            continue;
        }
        let needs_struct = module
            .structs
            .get(&type_entity)
            .map(|s| needs_clone_shim_struct(s, &module.ty_arena))
            .unwrap_or(false);
        if needs_struct {
            module.structs.get_mut(&type_entity).unwrap().type_info.copy =
                CopyBehavior::Clone(cloneable_proto);
            continue;
        }
        let needs_enum = module
            .enums
            .get(&type_entity)
            .map(|e| needs_clone_shim_enum(e, &module.ty_arena))
            .unwrap_or(false);
        if needs_enum {
            module.enums.get_mut(&type_entity).unwrap().type_info.copy =
                CopyBehavior::Clone(cloneable_proto);
        }
    }
}

/// True for a `: not Copyable` type that is *conditionally* Copyable (has gating
/// positions). Such a type's generic base is `None`, but concrete instances may
/// be `Clone`/`Bitwise` — so it gets a shim while keeping its `None` base and no
/// unconditional Cloneable witness.
fn is_conditional_container(module: &MirModule, entity: Entity) -> bool {
    if let Some(s) = module.structs.get(&entity) {
        return !s.conditionally_copyable.is_empty();
    }
    if let Some(e) = module.enums.get(&entity) {
        return !e.conditionally_copyable.is_empty();
    }
    false
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

    let tp_ty_ids: Vec<TyId> = struct_def
        .type_params
        .iter()
        .map(|tp| {
            module
                .ty_arena
                .find(|t| matches!(t, MirTy::TypeParam(e) if *e == tp.entity))
                .expect("TypeParam should be interned")
        })
        .collect();
    let self_ty = module
        .ty_arena
        .find(|t| {
            matches!(t, MirTy::Named { entity, type_args }
            if *entity == struct_def.entity && *type_args == tp_ty_ids)
        })
        .expect("struct type should be interned");

    let mut body = OssaBody::new();

    // Self param: @guaranteed T (Borrow convention).
    // StructExtract on @guaranteed → @guaranteed fields, CopyValue → @owned clones.
    let self_val = body.alloc_value(ValueDef {
        ty: self_ty,
        ownership: Ownership::Guaranteed,
        borrow_source: None,
        span: None,
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
    func.kind = FunctionKind::CloneShim {
        nominal: struct_def.entity,
    };
    func.type_params = struct_def.type_params.clone();
    func.params.push(ParamDef::new(
        "self",
        self_val,
        self_ty,
        ParamConvention::Borrow,
    ));
    func.body = Some(body);
    func
}

fn generate_enum_clone_shim(
    module: &MirModule,
    enum_def: &crate::item::enum_def::EnumDef,
    shim_entity: Entity,
) -> FunctionDef {
    let name = format!("__clone${}", enum_def.name);

    let tp_ty_ids: Vec<TyId> = enum_def
        .type_params
        .iter()
        .map(|tp| {
            module
                .ty_arena
                .find(|t| matches!(t, MirTy::TypeParam(e) if *e == tp.entity))
                .expect("TypeParam should be interned")
        })
        .collect();
    let self_ty = module
        .ty_arena
        .find(|t| {
            matches!(t, MirTy::Named { entity, type_args }
            if *entity == enum_def.entity && *type_args == tp_ty_ids)
        })
        .expect("enum type should be interned");

    let i32_ty = module
        .ty_arena
        .find(|t| matches!(t, MirTy::I32))
        .unwrap_or_else(|| {
            module
                .ty_arena
                .find(|t| matches!(t, MirTy::I8))
                .expect("integer type should be interned")
        });

    let mut body = OssaBody::new();

    // Self param: @guaranteed T (Borrow convention)
    let self_val = body.alloc_value(ValueDef {
        ty: self_ty,
        ownership: Ownership::Guaranteed,
        borrow_source: None,
        span: None,
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
    body.block_mut(exit_block).terminator = Terminator::new(TerminatorKind::Return(exit_result));

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
            span: None,
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
        variant_insts.push(Instruction::new(InstKind::DestroyValue {
            operand: variant_disc,
        }));

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
        body.block_mut(variant_block).terminator = Terminator::new(TerminatorKind::Jump {
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
        span: None,
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
    wildcard_insts.push(Instruction::new(InstKind::DestroyValue {
        operand: wildcard_disc,
    }));
    let wildcard_copy = body.alloc_value(ValueDef::owned(self_ty));
    wildcard_insts.push(Instruction::new(InstKind::CopyValue {
        result: wildcard_copy,
        operand: wildcard_self,
    }));
    body.block_mut(wildcard_block).insts = wildcard_insts;
    body.block_mut(wildcard_block).terminator = Terminator::new(TerminatorKind::Jump {
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
    func.kind = FunctionKind::CloneShim {
        nominal: enum_def.entity,
    };
    func.type_params = enum_def.type_params.clone();
    func.params.push(ParamDef::new(
        "self",
        self_val,
        self_ty,
        ParamConvention::Borrow,
    ));
    func.body = Some(body);
    func
}

fn ty_contains_unresolvable(ty: TyId, arena: &TyArena) -> bool {
    match arena.get(ty) {
        MirTy::AssociatedProjection { .. } => true,
        MirTy::Named { type_args, .. } => type_args
            .iter()
            .any(|&t| ty_contains_unresolvable(t, arena)),
        MirTy::Pointer(inner) => ty_contains_unresolvable(*inner, arena),
        MirTy::Tuple(elems) => elems.iter().any(|&t| ty_contains_unresolvable(t, arena)),
        MirTy::FuncThick { params, ret, .. } => {
            params
                .iter()
                .any(|&(t, _)| ty_contains_unresolvable(t, arena))
                || ty_contains_unresolvable(*ret, arena)
        },
        _ => false,
    }
}

fn has_unresolvable_fields_struct(s: &crate::item::struct_def::StructDef, arena: &TyArena) -> bool {
    s.fields
        .iter()
        .any(|f| ty_contains_unresolvable(f.ty, arena))
}

fn has_unresolvable_fields_enum(e: &crate::item::enum_def::EnumDef, arena: &TyArena) -> bool {
    e.cases.iter().any(|c| {
        c.payload_fields
            .iter()
            .any(|f| ty_contains_unresolvable(f.ty, arena))
    })
}

/// True if a field of type `ty` might need deep cloning: a Named or TypeParam
/// type, or a tuple that (recursively) contains one. A tuple has no nominal
/// entity of its own, so a struct/enum whose only non-trivial field is a tuple
/// of resources (e.g. `(String, String)`) must still get a clone shim — its
/// `CopyValue` is then deep-cloned element-wise during expand.
fn ty_needs_clone_shim(arena: &TyArena, ty: TyId) -> bool {
    match arena.get(ty) {
        MirTy::Named { .. } | MirTy::TypeParam(_) => true,
        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            elems.iter().any(|&e| ty_needs_clone_shim(arena, e))
        },
        _ => false,
    }
}

/// A struct needs a clone shim only if it has at least one field that might
/// need deep cloning (see [`ty_needs_clone_shim`]). All-primitive structs are
/// trivially bitwise-copyable and don't need a shim.
fn needs_clone_shim_struct(s: &crate::item::struct_def::StructDef, arena: &TyArena) -> bool {
    s.fields.iter().any(|f| ty_needs_clone_shim(arena, f.ty))
}

fn needs_clone_shim_enum(e: &crate::item::enum_def::EnumDef, arena: &TyArena) -> bool {
    e.cases.iter().any(|c| {
        c.payload_fields
            .iter()
            .any(|f| ty_needs_clone_shim(arena, f.ty))
    })
}
