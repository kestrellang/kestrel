//! Struct lowering — converts ECS struct entities into MIR StructDefs.

use kestrel_ast_builder::attrs::{LayoutOption, attrs_of};
use kestrel_ast_builder::{FieldClass, NodeKind, TypeParams};
use kestrel_hecs::Entity;
use kestrel_hir::builtin::Builtin;
use kestrel_mir::item::struct_def::{FieldDef, StructDef};
use kestrel_mir::{CopyBehavior, DropBehavior, LayoutAttrs, LayoutKind, TypeInfo, TypeParamDef};
use kestrel_name_res::{ConformingProtocols, ResolveBuiltin};
use kestrel_semantics::{ConditionalCopyableParams, CopySemantics, NominalCopySemantics};

use crate::context::LowerCtx;
use crate::ty::resolve_type_annotation;

pub fn lower_struct(ctx: &mut LowerCtx, entity: Entity) {
    let name = ctx.register_name(entity);
    let mut def = StructDef::new(entity, name);
    ctx.record_observable_drop(entity);

    def.type_params = collect_type_params(ctx, entity);
    def.type_info = TypeInfo {
        copy: lower_copy_behavior(ctx, entity),
        drop: lower_drop_behavior(ctx, entity),
        layout: None,
        ..TypeInfo::none()
    };
    def.conditionally_copyable = ctx.query.query(ConditionalCopyableParams {
        entity,
        root: ctx.root,
    });
    def.layout_attrs = lower_layout_attrs(ctx, entity);

    // Stored instance fields only. This loop DEFINES every FieldIdx, so it must
    // use `FieldClass` and nothing else — the old `!Callable && !Static` test
    // admitted bodyless `{ get set }` fields that the memberwise-init roster
    // (type-infer/generate.rs) excluded, shifting every later argument by one.
    for &child in ctx.world.children_of(entity) {
        if !ctx
            .world
            .get::<FieldClass>(child)
            .is_some_and(FieldClass::is_stored_instance)
        {
            continue;
        }

        let field_name = ctx
            .world
            .get::<kestrel_ast_builder::Name>(child)
            .map(|n| n.0.clone())
            .unwrap_or_default();
        let field_ty = resolve_type_annotation(ctx, child);
        def.add_field(FieldDef::new(field_name, field_ty));
    }

    ctx.module.add_struct(def);
}

pub(crate) fn lower_copy_behavior(ctx: &mut LowerCtx, entity: Entity) -> CopyBehavior {
    let info = ctx.query.query(NominalCopySemantics {
        entity,
        root: ctx.root,
    });
    match info.semantics {
        CopySemantics::Copyable => CopyBehavior::Bitwise,
        CopySemantics::Cloneable => CopyBehavior::Clone(entity),
        CopySemantics::NotCopyable => CopyBehavior::None,
    }
}

fn lower_drop_behavior(ctx: &LowerCtx, entity: Entity) -> DropBehavior {
    let deinit = find_user_deinit(ctx, entity);
    if deinit.is_some() {
        DropBehavior::StructDrop {
            deinit,
            fields: Vec::new(),
        }
    } else {
        DropBehavior::None
    }
}

pub(crate) fn find_user_deinit(ctx: &LowerCtx, entity: Entity) -> Option<Entity> {
    ctx.world
        .children_of(entity)
        .iter()
        .find(|&&child| ctx.world.get::<NodeKind>(child) == Some(&NodeKind::Deinit))
        .copied()
}

pub(crate) fn collect_type_params(ctx: &mut LowerCtx, entity: Entity) -> Vec<TypeParamDef> {
    let Some(type_params) = ctx.world.get::<TypeParams>(entity) else {
        return Vec::new();
    };
    type_params
        .0
        .iter()
        .map(|&tp_entity| {
            ctx.register_name(tp_entity);
            let tp_name = ctx
                .world
                .get::<kestrel_ast_builder::Name>(tp_entity)
                .map(|n| n.0.clone())
                .unwrap_or_default();
            TypeParamDef::new(tp_entity, tp_name)
        })
        .collect()
}

/// `@layout` and `@align` of a struct or enum, as the one MIR value every
/// layout computation reads. A type that conforms to `FFISafe` crosses the C
/// boundary, so without an explicit `@layout` it gets `.C` (D-Q11).
pub(crate) fn lower_layout_attrs(ctx: &LowerCtx, entity: Entity) -> LayoutAttrs {
    let attrs = attrs_of(&ctx.query, entity);
    let kind = match attrs.layout() {
        Some(LayoutOption::C) => LayoutKind::C,
        Some(LayoutOption::Packed) => LayoutKind::Packed,
        None if conforms_to_ffi_safe(ctx, entity) => LayoutKind::C,
        None => LayoutKind::Default,
    };
    LayoutAttrs {
        kind,
        min_align: attrs.align(),
    }
}

fn conforms_to_ffi_safe(ctx: &LowerCtx, entity: Entity) -> bool {
    let Some(ffi_safe) = ctx.query.query(ResolveBuiltin {
        builtin: Builtin::FFISafe,
        root: ctx.root,
    }) else {
        return false;
    };
    ctx.query
        .query(ConformingProtocols {
            entity,
            root: ctx.root,
        })
        .contains(&ffi_safe)
}
