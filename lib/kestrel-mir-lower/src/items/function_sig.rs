//! Function signature lowering.
//!
//! Lowers function entities into MIR FunctionDefs with params and return types.
//! Function bodies are lowered in a separate phase.

use kestrel_ast_builder::{Attributes, Callable, Intrinsic, NodeKind, Static, TypeParams};
use kestrel_hecs::Entity;
use kestrel_mir::item::function::{
    CallingConvention, ExternInfo, FunctionDef, FunctionKind, ParamDef, WhereClause,
    WhereConstraint,
};
use kestrel_mir::{ParamConvention, TyId, TypeParamDef, ValueId};

use crate::context::LowerCtx;
use crate::ty::{resolve_callable_return_type, resolve_callable_types};

pub(crate) fn receiver_convention(kind: &kestrel_ast_builder::ReceiverKind) -> ParamConvention {
    match kind {
        kestrel_ast_builder::ReceiverKind::Borrowing => ParamConvention::Borrow,
        kestrel_ast_builder::ReceiverKind::Mutating => ParamConvention::MutBorrow,
        kestrel_ast_builder::ReceiverKind::Consuming => ParamConvention::Consuming,
    }
}

/// True when `entity` is a member of an extension whose target is a protocol
/// (`extend SomeProtocol { ... }`) — a protocol default whose body may use
/// `Self` even when its signature doesn't, so witness resolution must propagate
/// `self_type` to it (#146). Mirrors the `in_protocol_extension` check in
/// `body::lower_function_body`.
fn is_protocol_extension_member(ctx: &LowerCtx, entity: Entity) -> bool {
    let Some(parent) = ctx.world.parent_of(entity) else {
        return false;
    };
    if ctx.world.get::<NodeKind>(parent) != Some(&NodeKind::Extension) {
        return false;
    }
    ctx.query
        .query(kestrel_name_res::ExtensionTargetEntity {
            extension: parent,
            root: ctx.root,
        })
        .is_some_and(|target| ctx.world.get::<NodeKind>(target) == Some(&NodeKind::Protocol))
}

/// Lower a function entity into a MIR FunctionDef (signature only, no body).
pub fn lower_function_sig(ctx: &mut LowerCtx, entity: Entity) {
    let name = ctx.register_name(entity);
    let ret_ty = resolve_callable_return_type(ctx, entity);

    let mut def = FunctionDef::new(entity, &name, ret_ty);
    def.kind = determine_function_kind(ctx, entity);
    def.provides_protocol_default = is_protocol_extension_member(ctx, entity);

    collect_inherited_type_params(ctx, entity, &mut def);

    let is_extern = ctx
        .world
        .get::<Attributes>(entity)
        .is_some_and(|attrs| attrs.0.iter().any(|a| a.name == "extern"));

    // `@main` marks the program entry point. Set early so both the intrinsic
    // early-return path and the normal path carry it. Replaces discover-by-name.
    def.is_main = ctx
        .world
        .get::<Attributes>(entity)
        .is_some_and(|attrs| attrs.0.iter().any(|a| a.name == "main"));

    // Function's own type params
    if let Some(type_params) = ctx.world.get::<TypeParams>(entity) {
        for &tp_entity in &type_params.0 {
            ctx.register_name(tp_entity);
            let tp_name = ctx
                .world
                .get::<kestrel_ast_builder::Name>(tp_entity)
                .map(|n| n.0.clone())
                .unwrap_or_default();
            def.type_params.push(TypeParamDef::new(tp_entity, tp_name));
        }
    }

    populate_where_clause(ctx, entity, &mut def);

    // Parameters
    if let Some(callable) = ctx.world.get::<Callable>(entity) {
        // Self parameter for methods
        if let Some(receiver) = &callable.receiver {
            let is_user_deinit =
                matches!(ctx.world.get::<NodeKind>(entity), Some(NodeKind::Deinit));
            let convention = if is_user_deinit {
                ParamConvention::MutBorrow
            } else {
                receiver_convention(receiver)
            };
            let self_ty = resolve_self_type_for_function(ctx, entity);
            let value_id = ValueId::new(0); // placeholder
            let param = ParamDef::new("self", value_id, self_ty, convention);
            def.params.push(param);
        }

        let resolved_types = resolve_callable_types(ctx, entity);
        for (i, ast_param) in callable.params.iter().enumerate() {
            let inner_ty = resolved_types
                .get(i)
                .and_then(|t| *t)
                .unwrap_or_else(|| ctx.module.ty_arena.error());
            let value_id = ValueId::new(def.params.len());
            let convention = if is_extern || ast_param.is_consuming {
                ParamConvention::Consuming
            } else if ast_param.is_mut {
                ParamConvention::MutBorrow
            } else {
                ParamConvention::Borrow
            };
            let param = ParamDef::with_label(
                &ast_param.name,
                value_id,
                inner_ty,
                convention,
                ast_param.label.clone(),
            );
            def.params.push(param);
        }
    }

    // @extern
    if is_extern && let Some(attrs) = ctx.world.get::<Attributes>(entity) {
        let symbol_name = attrs
            .extern_mangle_name()
            .unwrap_or_else(|| def.name.rsplit('.').next().unwrap_or(&def.name).to_string());
        def.extern_info = Some(ExternInfo {
            calling_convention: CallingConvention::C,
            symbol_name,
        });
    }

    // Intrinsic functions have no body
    if ctx.world.get::<Intrinsic>(entity).is_some() {
        ctx.module.add_function(def);
        return;
    }

    ctx.module.add_function(def);

    // Lower function body if present
    if ctx
        .world
        .get::<kestrel_ast_builder::Valued>(entity)
        .is_some()
    {
        crate::body::lower_function_body(ctx, entity, entity);
    }
}

fn determine_function_kind(ctx: &LowerCtx, entity: Entity) -> FunctionKind {
    let kind = ctx
        .world
        .get::<NodeKind>(entity)
        .cloned()
        .unwrap_or(NodeKind::Function);

    match kind {
        NodeKind::Initializer => {
            let parent = ctx.world.parent_of(entity).unwrap_or(ctx.root);
            FunctionKind::Initializer { parent }
        },
        NodeKind::Deinit => {
            let parent = ctx.world.parent_of(entity).unwrap_or(ctx.root);
            FunctionKind::Deinit { parent }
        },
        NodeKind::Function => {
            let Some(parent) = ctx.world.parent_of(entity) else {
                return FunctionKind::Free;
            };
            let parent_kind = ctx.world.get::<NodeKind>(parent).cloned();
            match parent_kind {
                Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Extension) => {
                    if ctx.world.get::<Static>(entity).is_some() {
                        FunctionKind::StaticMethod { parent }
                    } else if let Some(callable) = ctx.world.get::<Callable>(entity) {
                        if let Some(recv) = &callable.receiver {
                            let receiver = receiver_convention(recv);
                            FunctionKind::Method { parent, receiver }
                        } else {
                            FunctionKind::StaticMethod { parent }
                        }
                    } else {
                        FunctionKind::StaticMethod { parent }
                    }
                },
                Some(NodeKind::Protocol) => FunctionKind::Free,
                _ => FunctionKind::Free,
            }
        },
        NodeKind::Field | NodeKind::Subscript | NodeKind::Setter | NodeKind::RefAccessor => {
            let parent = accessor_enclosing_container(ctx, entity).unwrap_or(ctx.root);
            if let Some(callable) = ctx.world.get::<Callable>(entity) {
                if let Some(recv) = &callable.receiver {
                    FunctionKind::Method {
                        parent,
                        receiver: receiver_convention(recv),
                    }
                } else {
                    FunctionKind::StaticMethod { parent }
                }
            } else {
                FunctionKind::Free
            }
        },
        _ => FunctionKind::Free,
    }
}

fn accessor_enclosing_container(ctx: &LowerCtx, entity: Entity) -> Option<Entity> {
    let direct = ctx.world.parent_of(entity)?;
    match ctx.world.get::<NodeKind>(direct).cloned() {
        Some(NodeKind::Field) | Some(NodeKind::Subscript) => ctx.world.parent_of(direct),
        _ => Some(direct),
    }
}

fn collect_inherited_type_params(ctx: &mut LowerCtx, entity: Entity, def: &mut FunctionDef) {
    let Some(parent) =
        accessor_enclosing_container(ctx, entity).or_else(|| ctx.world.parent_of(entity))
    else {
        return;
    };
    let parent_kind = ctx.world.get::<NodeKind>(parent).cloned();
    if !matches!(
        parent_kind,
        Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Extension)
    ) {
        return;
    }

    // For extensions, type params come from the target type
    let type_params_source = if matches!(parent_kind, Some(NodeKind::Extension)) {
        ctx.query.query(kestrel_name_res::ExtensionTargetEntity {
            extension: parent,
            root: ctx.root,
        })
    } else {
        Some(parent)
    };

    if let Some(source) = type_params_source
        && let Some(type_params) = ctx.world.get::<TypeParams>(source)
    {
        for &tp_entity in &type_params.0 {
            ctx.register_name(tp_entity);
            let tp_name = ctx
                .world
                .get::<kestrel_ast_builder::Name>(tp_entity)
                .map(|n| n.0.clone())
                .unwrap_or_default();
            def.type_params.push(TypeParamDef::new(tp_entity, tp_name));
        }
    }

    // Extension's own free type params (e.g. `extend Int64: ArrayIndex[T]`)
    if matches!(parent_kind, Some(NodeKind::Extension))
        && let Some(ext_params) = ctx.world.get::<TypeParams>(parent)
    {
        let already_added: std::collections::HashSet<Entity> =
            def.type_params.iter().map(|tp| tp.entity).collect();
        for &tp_entity in &ext_params.0 {
            if already_added.contains(&tp_entity) {
                continue;
            }
            ctx.register_name(tp_entity);
            let tp_name = ctx
                .world
                .get::<kestrel_ast_builder::Name>(tp_entity)
                .map(|n| n.0.clone())
                .unwrap_or_default();
            def.type_params.push(TypeParamDef::new(tp_entity, tp_name));
        }
    }

    // Setter / ref accessor under a generic subscript inherits its type params
    if matches!(
        ctx.world.get::<NodeKind>(entity),
        Some(NodeKind::Setter | NodeKind::RefAccessor)
    ) {
        let parent_subscript = ctx
            .world
            .parent_of(entity)
            .filter(|p| matches!(ctx.world.get::<NodeKind>(*p), Some(NodeKind::Subscript)));
        if let Some(parent_subscript) = parent_subscript
            && let Some(type_params) = ctx.world.get::<TypeParams>(parent_subscript)
        {
            for &tp_entity in &type_params.0 {
                ctx.register_name(tp_entity);
                let tp_name = ctx
                    .world
                    .get::<kestrel_ast_builder::Name>(tp_entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
                def.type_params.push(TypeParamDef::new(tp_entity, tp_name));
            }
        }
    }
}

fn populate_where_clause(ctx: &mut LowerCtx, entity: Entity, def: &mut FunctionDef) {
    let mut wc = WhereClause::new();
    let mut has_type_params = false;
    let mut current = Some(entity);
    while let Some(e) = current {
        lower_where_clauses(ctx, e, &mut wc);
        if ctx
            .world
            .get::<TypeParams>(e)
            .is_some_and(|tp| !tp.0.is_empty())
        {
            has_type_params = true;
        }
        current = ctx.world.parent_of(e);
    }
    if !wc.constraints.is_empty() || has_type_params {
        def.where_clause = Some(wc);
    }
}

/// `owner`'s explicit where clauses as MIR constraints, read from the front
/// end's resolved `ExplicitWhereClauses` — never re-resolved from the AST
/// (bidi §7.2: one resolution, no private name lookup in mir-lower). The
/// implicit `T: Copyable` / `T: Static` bounds `WhereClausesOf` injects are
/// deliberately absent: those are structural, and mono has no witnesses for
/// them.
pub(crate) fn lower_where_clauses(ctx: &mut LowerCtx, owner: Entity, out: &mut WhereClause) {
    use kestrel_type_infer::resolve::WhereClause as Clause;
    let clauses = ctx
        .query
        .query(kestrel_type_infer::where_clauses::ExplicitWhereClauses {
            entity: owner,
            root: ctx.root,
        })
        .clauses;
    // A `Self` subject constrains `TypeParam(protocol)` when `owner` sits in a
    // protocol or protocol extension (how abstract `Self` lowers).
    let self_param = self_protocol(ctx, owner);
    let subject_entity = |s: &kestrel_type_infer::resolve::WhereSubject| match s {
        kestrel_type_infer::resolve::WhereSubject::SelfType => self_param,
        _ => subject_entity(s),
    };
    for clause in &clauses {
        match clause {
            Clause::Bound {
                subject,
                protocol,
                protocol_type_args,
            } => {
                let Some(subject) = subject_entity(subject) else {
                    continue;
                };
                let args = protocol_type_args
                    .iter()
                    .filter_map(type_arg_entity)
                    .collect();
                out.add_constraint(WhereConstraint::implements_with_args(
                    subject, *protocol, args,
                ));
            },
            Clause::NotBound { subject, protocol } => {
                if let Some(subject) = subject_entity(subject) {
                    out.add_constraint(WhereConstraint::not_implements(subject, *protocol));
                }
            },
            Clause::Equality { subject, rhs } => {
                let Some(subject) = subject_hir(subject) else {
                    continue;
                };
                let subject = crate::ty::lower_type(ctx, &subject);
                let rhs = crate::ty::lower_type(ctx, rhs);
                out.add_constraint(WhereConstraint::Equal { subject, rhs });
            },
        }
    }
}

/// The entity a bound's subject constrains: the param, or the associated
/// type a projection names (`T.Item: P` constrains `Item`). `Self` has no
/// entity at this vantage — mono judges it as the receiver — and is skipped.
fn subject_entity(subject: &kestrel_type_infer::resolve::WhereSubject) -> Option<Entity> {
    use kestrel_type_infer::resolve::WhereSubject;
    match subject {
        WhereSubject::Param(p) => Some(*p),
        WhereSubject::Projection { assoc, .. } => Some(*assoc),
        WhereSubject::SelfType => None,
    }
}

/// A protocol type argument as the entity mono keys it by (`T` in
/// `SeqIndex[T]`, or a nominal).
fn type_arg_entity(ty: &kestrel_hir::ty::HirTy) -> Option<Entity> {
    use kestrel_hir::ty::HirTy;
    match ty {
        HirTy::Param(e, _)
        | HirTy::Struct { entity: e, .. }
        | HirTy::Enum { entity: e, .. }
        | HirTy::Protocol { entity: e, .. }
        | HirTy::AbstractAssoc { entity: e, .. } => Some(*e),
        HirTy::Alias { expanded, .. } => type_arg_entity(expanded),
        _ => None,
    }
}

/// A where-clause subject as a type. `Self` has no type at this vantage (it
/// is the receiver mono judges), so a `Self`-rooted subject is skipped.
fn subject_hir(
    subject: &kestrel_type_infer::resolve::WhereSubject,
) -> Option<kestrel_hir::ty::HirTy> {
    use kestrel_hir::ty::HirTy;
    use kestrel_type_infer::resolve::WhereSubject;
    let span = kestrel_span::Span::synthetic(0);
    match subject {
        WhereSubject::Param(p) => Some(HirTy::Param(*p, span)),
        WhereSubject::SelfType => None,
        WhereSubject::Projection { base, assoc } => Some(HirTy::AssocProjection {
            base: Box::new(subject_hir(base)?),
            assoc: *assoc,
            span,
        }),
    }
}

/// The protocol whose abstract `Self` is in scope at `owner` (a protocol, a
/// protocol extension, or anything nested in one), if any.
fn self_protocol(ctx: &LowerCtx, owner: Entity) -> Option<Entity> {
    let mut current = Some(owner);
    while let Some(entity) = current {
        let target = match ctx.world.get::<NodeKind>(entity) {
            Some(NodeKind::Protocol) => Some(entity),
            Some(NodeKind::Extension) => ctx.query.query(kestrel_name_res::ExtensionTargetEntity {
                extension: entity,
                root: ctx.root,
            }),
            Some(NodeKind::Struct | NodeKind::Enum) => return None,
            _ => {
                current = ctx.world.parent_of(entity);
                continue;
            },
        };
        return target.filter(|t| ctx.world.get::<NodeKind>(*t) == Some(&NodeKind::Protocol));
    }
    None
}

/// Walk the parent chain from a function entity to find the enclosing
/// struct/enum/protocol and build its Self type as `Named(entity, [TypeParam...])`.
fn resolve_self_type_for_function(ctx: &mut LowerCtx, func_entity: Entity) -> TyId {
    let mut current = ctx.world.parent_of(func_entity);
    while let Some(entity) = current {
        match ctx.world.get::<NodeKind>(entity).cloned() {
            Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol) => {
                return crate::ty::build_self_type(ctx, entity);
            },
            Some(NodeKind::Extension) => {
                // Extension target: resolve the target entity
                if let Some(target) = ctx.query.query(kestrel_name_res::ExtensionTargetEntity {
                    extension: entity,
                    root: ctx.root,
                }) {
                    return crate::ty::build_self_type(ctx, target);
                }
                return ctx.module.ty_arena.error();
            },
            _ => current = ctx.world.parent_of(entity),
        }
    }
    ctx.module.ty_arena.error()
}
