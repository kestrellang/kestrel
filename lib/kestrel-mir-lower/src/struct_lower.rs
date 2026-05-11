//! Struct lowering — converts ECS struct entities into MIR StructDefs.

use kestrel_ast_builder::{Callable, NodeKind, Static, TypeParams};
use kestrel_hecs::Entity;
use kestrel_mir::{CopyBehavior, FieldDef, StructDef, StructId, TypeParamDef};
use kestrel_semantics::{CopySemantics, NominalCopySemantics};

use crate::context::LowerCtx;
use crate::ty::resolve_type_annotation;

/// Lower a struct entity into a MIR StructDef.
pub fn lower_struct(ctx: &mut LowerCtx, entity: Entity) -> StructId {
    let name = ctx.register_name(entity);
    let mut def = StructDef::new(entity, name);

    // Populate copy_behavior from the semantic layer's NominalCopySemantics.
    // DeinitBehavior is left default (trivial) for Stage 1; the legacy deinit
    // pass still handles drops based on its own per-type analysis. Stage 7
    // populates field_drops and user_method from the semantic model.
    def.copy_behavior = lower_copy_behavior(ctx, entity);

    // Type parameters
    if let Some(type_params) = ctx.world.get::<TypeParams>(entity) {
        for &tp_entity in &type_params.0 {
            ctx.register_name(tp_entity);
            let tp_name = ctx
                .world
                .get::<kestrel_ast_builder::Name>(tp_entity)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| format!("{:?}", tp_entity));
            def.type_params.push(TypeParamDef::new(tp_entity, tp_name));
        }
    }

    // Fields: children with NodeKind::Field (stored fields only, not computed properties)
    for &child in ctx.world.children_of(entity) {
        let Some(kind) = ctx.world.get::<NodeKind>(child) else {
            continue;
        };
        if *kind != NodeKind::Field {
            continue;
        }

        // Skip computed properties (have a Callable component / getter body)
        // and static properties (class-level, not per-instance).
        // Both are lowered as separate functions, not struct fields.
        if ctx.world.get::<Callable>(child).is_some() || ctx.world.get::<Static>(child).is_some() {
            continue;
        }

        let field_name = ctx
            .world
            .get::<kestrel_ast_builder::Name>(child)
            .map(|n| n.0.clone())
            .unwrap_or_default();

        // Resolve field type: TypeAnnotation(AstType) → HirTy → MirTy
        let field_ty = resolve_type_annotation(ctx, child);

        def.add_field(FieldDef::new(field_name, field_ty));
    }

    ctx.module.add_struct(def)
}

/// Resolve a nominal entity's [`CopyBehavior`] using `kestrel-semantics`.
///
/// - `Copyable` → `Bitwise`
/// - `Cloneable` → `Clone(entity)` — Stage 1 uses the nominal entity itself as
///   a placeholder for the clone-method reference; Stage 5+ resolves the
///   actual method through the witness table.
/// - `NotCopyable` → `None`
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
