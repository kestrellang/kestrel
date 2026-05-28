//! Post-lowering fixup for DropBehavior.
//!
//! The MIR lowering sets DropBehavior based on user-defined deinit only,
//! leaving `fields`/`variants` empty and returning `None` for types that
//! lack a deinit but contain droppable fields. This pass runs after all
//! types are lowered, scanning fields with `needs_drop` and promoting
//! DropBehavior as needed. Fixed-point handles transitive chains
//! (A contains B which just became droppable).

use kestrel_debug::ktrace;

use crate::ty::MirTy;
use crate::ty_query::needs_drop;
use crate::{DropBehavior, FieldIdx, MirModule, VariantIdx};

/// Populate `fields`/`variants` in DropBehavior for all struct and enum
/// types. Must run before drop shim synthesis.
pub fn fix_drop_behaviors(module: &mut MirModule) {
    loop {
        let mut changed = false;
        changed |= fix_structs(module);
        changed |= fix_enums(module);
        if !changed {
            break;
        }
    }
}

/// Check if a field needs dropping based on concrete type info only.
/// Skips TypeParam and AssociatedProjection — those are resolved at
/// monomorphization time.
fn field_needs_drop(module: &MirModule, ty: crate::TyId) -> bool {
    match module.ty_arena.get(ty) {
        MirTy::TypeParam(_) | MirTy::AssociatedProjection { .. } => true,
        _ => needs_drop(&module.ty_arena, module, ty),
    }
}

fn fix_structs(module: &mut MirModule) -> bool {
    let mut changed = false;

    // Collect keys first so we can borrow `module` immutably for field_needs_drop,
    // then mutably for the update.
    let keys: Vec<_> = module.structs.keys().copied().collect();

    for entity in keys {
        let droppable_fields: Vec<FieldIdx> = module.structs[&entity]
            .fields
            .iter()
            .enumerate()
            .filter(|(_, f)| field_needs_drop(module, f.ty))
            .map(|(i, _)| FieldIdx::new(i))
            .collect();

        if droppable_fields.is_empty() {
            continue;
        }

        let s = module.structs.get_mut(&entity).unwrap();
        match &mut s.type_info.drop {
            DropBehavior::None => {
                ktrace!("drop-fix", "promoting struct '{}' to droppable (fields: {:?})",
                    s.name, droppable_fields);
                s.type_info.drop = DropBehavior::StructDrop {
                    deinit: None,
                    fields: droppable_fields,
                };
                changed = true;
            }
            DropBehavior::StructDrop { fields, .. } => {
                for field in droppable_fields {
                    if !fields.contains(&field) {
                        fields.push(field);
                        changed = true;
                    }
                }
            }
            _ => {}
        }
    }
    changed
}

fn fix_enums(module: &mut MirModule) -> bool {
    let mut changed = false;

    // Collect keys first so we can borrow `module` immutably for field_needs_drop,
    // then mutably for the update.
    let keys: Vec<_> = module.enums.keys().copied().collect();

    for entity in keys {
        let mut droppable_variants: Vec<(VariantIdx, Vec<FieldIdx>)> = Vec::new();
        for (vi, case) in module.enums[&entity].cases.iter().enumerate() {
            let droppable_fields: Vec<FieldIdx> = case
                .payload_fields
                .iter()
                .enumerate()
                .filter(|(_, f)| field_needs_drop(module, f.ty))
                .map(|(i, _)| FieldIdx::new(i))
                .collect();
            if !droppable_fields.is_empty() {
                droppable_variants.push((VariantIdx::new(vi), droppable_fields));
            }
        }

        if droppable_variants.is_empty() {
            continue;
        }

        let e = module.enums.get_mut(&entity).unwrap();
        match &mut e.type_info.drop {
            DropBehavior::None => {
                e.type_info.drop = DropBehavior::EnumDrop {
                    deinit: None,
                    variants: droppable_variants,
                };
                changed = true;
            }
            DropBehavior::EnumDrop { variants, .. } => {
                for (variant, fields_to_add) in droppable_variants {
                    if let Some((_, fields)) = variants.iter_mut().find(|(v, _)| *v == variant) {
                        for field in fields_to_add {
                            if !fields.contains(&field) {
                                fields.push(field);
                                changed = true;
                            }
                        }
                    } else {
                        variants.push((variant, fields_to_add));
                        changed = true;
                    }
                }
            }
            _ => {}
        }
    }
    changed
}
