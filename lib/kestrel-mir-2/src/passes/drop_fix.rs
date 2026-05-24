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
/// monomorphization time. Marking generic containers as droppable here
/// would conflict with the verifier's "no projected moves from droppable
/// aggregates" rule on match/switch patterns.
fn field_needs_drop(module: &MirModule, ty: crate::TyId) -> bool {
    match module.ty_arena.get(ty) {
        MirTy::TypeParam(_) | MirTy::AssociatedProjection { .. } => false,
        _ => needs_drop(&module.ty_arena, module, ty),
    }
}

fn fix_structs(module: &mut MirModule) -> bool {
    let mut changed = false;
    for si in 0..module.structs.len() {
        let droppable_fields: Vec<FieldIdx> = module.structs[si]
            .fields
            .iter()
            .enumerate()
            .filter(|(_, f)| field_needs_drop(module, f.ty))
            .map(|(i, _)| FieldIdx::new(i))
            .collect();

        if droppable_fields.is_empty() {
            continue;
        }

        match &module.structs[si].type_info.drop {
            DropBehavior::None => {
                ktrace!("drop-fix", "promoting struct '{}' to droppable (fields: {:?})",
                    module.structs[si].name, droppable_fields);
                module.structs[si].type_info.drop = DropBehavior::StructDrop {
                    deinit: None,
                    fields: droppable_fields,
                };
                changed = true;
            }
            _ => {}
        }
    }
    changed
}

fn fix_enums(module: &mut MirModule) -> bool {
    let mut changed = false;
    for ei in 0..module.enums.len() {
        let mut droppable_variants: Vec<(VariantIdx, Vec<FieldIdx>)> = Vec::new();
        for (vi, case) in module.enums[ei].cases.iter().enumerate() {
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

        match &module.enums[ei].type_info.drop {
            DropBehavior::None => {
                module.enums[ei].type_info.drop = DropBehavior::EnumDrop {
                    deinit: None,
                    variants: droppable_variants,
                };
                changed = true;
            }
            _ => {}
        }
    }
    changed
}
