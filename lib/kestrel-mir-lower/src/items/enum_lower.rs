//! Enum lowering — converts ECS enum entities into MIR EnumDefs.
//!
//! Unlike kestrel-mir (old), kestrel-mir stores payload fields inline on
//! EnumCaseDef rather than referencing a separate payload struct.

use kestrel_ast_builder::{Callable, NodeKind};
use kestrel_hecs::Entity;
use kestrel_mir::item::enum_def::{EnumCaseDef, EnumDef, IndirectPayload};
use kestrel_mir::item::struct_def::FieldDef;
use kestrel_mir::{DropBehavior, TypeInfo};

use crate::context::LowerCtx;
use crate::items::struct_lower::{
    collect_type_params, find_user_deinit, lower_copy_behavior, lower_layout_attrs,
};
use crate::ty::resolve_callable_types;

pub fn lower_enum(ctx: &mut LowerCtx, entity: Entity) {
    let name = ctx.register_name(entity);
    let mut def = EnumDef::new(entity, &name);

    def.type_params = collect_type_params(ctx, entity);
    def.type_info = TypeInfo {
        copy: lower_copy_behavior(ctx, entity),
        drop: lower_enum_drop_behavior(ctx, entity),
        layout: None,
        ..TypeInfo::none()
    };
    def.conditionally_copyable = ctx
        .query
        .query(kestrel_semantics::ConditionalCopyableParams {
            entity,
            root: ctx.root,
        });
    def.layout_attrs = lower_layout_attrs(ctx, entity);

    let boxed = ctx.query.query(kestrel_semantics::IndirectCases {
        entity,
        root: ctx.root,
    });
    let children: Vec<Entity> = ctx.world.children_of(entity).to_vec();
    for child in children {
        if ctx.world.get::<NodeKind>(child) != Some(&NodeKind::EnumCase) {
            continue;
        }

        let case_name = ctx
            .world
            .get::<kestrel_ast_builder::Name>(child)
            .map(|n| n.0.clone())
            .unwrap_or_default();

        let mut payload_fields = Vec::new();
        if let Some(callable) = ctx.world.get::<Callable>(child) {
            let resolved_types = resolve_callable_types(ctx, child);
            for (i, param) in callable.params.iter().enumerate() {
                let field_name = if param.name.is_empty() {
                    format!("{}", i)
                } else {
                    param.name.clone()
                };
                let field_ty = resolved_types
                    .get(i)
                    .and_then(|t| *t)
                    .unwrap_or_else(|| ctx.module.ty_arena.error());
                payload_fields.push(FieldDef::new(field_name, field_ty));
            }
        }

        let discriminant = def.cases.len() as u32;
        let mut case = EnumCaseDef::with_payload(case_name, discriminant, payload_fields);
        if boxed.contains(&child) {
            box_case(ctx, &mut case);
        }
        def.add_case(case);
    }

    ctx.module.add_enum(def);
}

fn lower_enum_drop_behavior(ctx: &LowerCtx, entity: Entity) -> DropBehavior {
    let deinit = find_user_deinit(ctx, entity);
    if deinit.is_some() {
        DropBehavior::EnumDrop {
            deinit,
            variants: Vec::new(),
        }
    } else {
        DropBehavior::None
    }
}

/// Store a recursive `indirect enum` case behind one `OwnedBox` holding its
/// whole payload (A1 of the indirect-enums plan): the declared fields move to
/// `indirect`, and the physical payload becomes the single box field, so
/// layout, drop/clone shims, mono and codegen need no indirect-specific code.
/// Without a stdlib `OwnedBox` the case stays inline (E406 is reported).
fn box_case(ctx: &mut LowerCtx, case: &mut EnumCaseDef) {
    let Some(owned_box) = ctx.query.query(kestrel_name_res::ResolveBuiltin {
        builtin: kestrel_hir::Builtin::OwnedBox,
        root: ctx.root,
    }) else {
        return;
    };
    ctx.register_name(owned_box);
    let logical_fields = std::mem::take(&mut case.payload_fields);
    let payload_ty = match logical_fields.as_slice() {
        [single] => single.ty,
        many => ctx
            .module
            .ty_arena
            .intern(kestrel_mir::MirTy::Tuple(many.iter().map(|f| f.ty).collect())),
    };
    let box_ty = ctx.module.ty_arena.named(owned_box, vec![payload_ty]);
    case.payload_fields = vec![FieldDef::new("$box", box_ty)];
    case.indirect = Some(IndirectPayload {
        logical_fields,
        payload_ty,
        box_ty,
    });
}
