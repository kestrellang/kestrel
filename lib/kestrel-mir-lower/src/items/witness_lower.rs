//! Witness generation — creates WitnessDef entries from conformance data.

use kestrel_ast::AstType;
use kestrel_ast_builder::{
    Callable, Name, NodeKind, Settable, Subscript as SubscriptMarker, TypeParams,
};
use kestrel_hecs::Entity;
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::LowerExtensionTargetTypeArgs;
use kestrel_mir::item::witness::{WitnessDef, WitnessMethodBinding};
use kestrel_mir::{MirTy, SubstMap, TyId, TypeParamDef, WitnessMethodKey, substitute};
use kestrel_name_res::conformances::ConformingProtocolInstantiations;
use kestrel_name_res::extensions::ExtensionsFor;
use kestrel_name_res::{
    ExtensionTargetEntity, ProtocolAssociatedTypes, ProtocolMember, ProtocolMembers,
    TypeMemberSource, TypeMembersByName,
};

use crate::context::LowerCtx;
use crate::ty::{lower_named_type, lower_type, resolve_callable_types, resolve_type_annotation};

/// Generate witness tables for all struct and enum entities.
pub fn lower_witnesses(ctx: &mut LowerCtx) {
    // Collect (entity, type_params) first so we can take `&mut ctx` to build each
    // implementing type below (the borrow checker won't let us hold the
    // `structs`/`enums` iterator while mutating the arena).
    let mut entities: Vec<(Entity, Vec<TypeParamDef>)> = Vec::new();
    for s in ctx.module.structs.values() {
        entities.push((s.entity, s.type_params.clone()));
    }
    for e in ctx.module.enums.values() {
        entities.push((e.entity, e.type_params.clone()));
    }

    for (entity, type_params) in entities {
        let type_args: Vec<TyId> = type_params
            .iter()
            .map(|tp| ctx.module.ty_arena.intern(MirTy::TypeParam(tp.entity)))
            .collect();
        // `lower_named_type` maps intrinsic `lang.*` entities to their primitive
        // `MirTy` (e.g. `lang.i64` → `I64`) so a witness from `extend lang.i64: P`
        // matches the primitive self type at call sites; normal structs/enums
        // stay `Named { entity, .. }`.
        let impl_ty = lower_named_type(ctx, entity, type_args);
        lower_witnesses_for_type(ctx, entity, impl_ty, &type_params);
    }
}

fn lower_witnesses_for_type(
    ctx: &mut LowerCtx,
    type_entity: Entity,
    impl_ty: TyId,
    impl_type_params: &[TypeParamDef],
) {
    let instantiations = ctx.query.query(ConformingProtocolInstantiations {
        entity: type_entity,
        root: ctx.root,
    });

    let extensions = ctx.query.query(ExtensionsFor {
        target: type_entity,
        root: ctx.root,
    });

    for (protocol, source, ast_type_args) in &instantiations {
        let owner_for_args = if matches!(
            ctx.world.get::<NodeKind>(*source),
            Some(NodeKind::Extension)
        ) {
            *source
        } else {
            type_entity
        };
        let proto_type_args = lower_protocol_type_args(ctx, owner_for_args, ast_type_args);

        // A specialized extension (`extend Box[lang.i64]: P`) gets a CONCRETE
        // implementing type and binds its OWN method impls, so the mono witness
        // selector prefers it over a generic `extend Box[T]: P`. Generic
        // extensions and direct conformances keep the generic implementing type.
        let concrete_args = if matches!(
            ctx.world.get::<NodeKind>(*source),
            Some(NodeKind::Extension)
        ) {
            lower_concrete_target_args(ctx, *source)
        } else {
            None
        };
        // Prefer the source extension's own method impls when EITHER the
        // implementing type is specialized (`extend Box[lang.i64]`) OR the
        // PROTOCOL args are concrete (`extend S: Producer[Int64]`). The latter
        // keeps each witness of a type that conforms to the same parameterized
        // protocol more than once bound to its own instantiation's method —
        // otherwise both collapse to the first `produce` found via the merged
        // type-member discovery (which matches on params, not return type).
        let proto_args_concrete = proto_type_args
            .iter()
            .any(|t| !matches!(ctx.module.ty_arena.get(*t), MirTy::TypeParam(_)));
        let prefer_source = concrete_args.is_some() || proto_args_concrete;
        let witness_impl_ty = match &concrete_args {
            Some(args) => ctx.module.ty_arena.named(type_entity, args.clone()),
            None => impl_ty,
        };

        // Build type args for witness method bindings.
        // These are TypeParam TyIds that the monomorphizer will substitute through
        // the pattern-match bindings to get concrete types.
        // Combine: impl type params (from the struct/enum) + protocol type args
        // (which may include extension free params like T in `extend Int64: SeqIndex[T]`).
        let impl_type_arg_tys: Vec<TyId> = {
            // Concrete witnesses use the concrete args (no pattern bindings to
            // substitute at mono time); generic witnesses use the struct params.
            let mut tys: Vec<TyId> = match &concrete_args {
                Some(args) => args.clone(),
                None => impl_type_params
                    .iter()
                    .map(|tp| ctx.module.ty_arena.intern(MirTy::TypeParam(tp.entity)))
                    .collect(),
            };
            let impl_entities: std::collections::HashSet<kestrel_hecs::Entity> =
                impl_type_params.iter().map(|tp| tp.entity).collect();
            for &pta in &proto_type_args {
                if let MirTy::TypeParam(e) = ctx.module.ty_arena.get(pta) {
                    if !impl_entities.contains(e) {
                        tys.push(pta);
                    }
                } else {
                    tys.push(pta);
                }
            }
            tys
        };

        let mut witness = WitnessDef::new(*protocol, witness_impl_ty);
        witness.proto_type_args = proto_type_args.clone();
        ctx.register_name(*protocol);

        // Build substitution map for protocol type params
        let proto_tp_entities = protocol_type_param_entities(ctx, *protocol);
        let proto_subst: std::collections::HashMap<Entity, TyId> = proto_tp_entities
            .iter()
            .zip(proto_type_args.iter())
            .map(|(e, t)| (*e, *t))
            .collect();

        // Extension free type params
        if matches!(
            ctx.world.get::<NodeKind>(*source),
            Some(NodeKind::Extension)
        ) && let Some(tp) = ctx.world.get::<TypeParams>(*source)
        {
            let already: std::collections::HashSet<Entity> =
                impl_type_params.iter().map(|t| t.entity).collect();
            for &tp_entity in &tp.0 {
                if already.contains(&tp_entity) {
                    continue;
                }
                ctx.register_name(tp_entity);
                let _tp_name = ctx
                    .world
                    .get::<Name>(tp_entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
            }
        }

        // Bind methods
        let proto_members = ctx.query.query(ProtocolMembers {
            protocol: *protocol,
            root: ctx.root,
        });

        let method_entries: Vec<(WitnessMethodKey, String, ProtocolMember)> = proto_members
            .iter()
            .flat_map(|m| {
                let name = protocol_member_name(ctx, m);
                let labels = get_param_labels(ctx, m.entity).unwrap_or_default();
                let mut entries = vec![(
                    WitnessMethodKey::new(name.clone(), labels.clone()),
                    name.clone(),
                    m.clone(),
                )];
                if ctx.world.get::<Settable>(m.entity).is_some() {
                    let setter_name = format!("{name}.set");
                    entries.push((
                        WitnessMethodKey::new(setter_name.clone(), labels.clone()),
                        setter_name,
                        m.clone(),
                    ));
                }
                entries
            })
            .collect();

        bind_witness_methods(
            ctx,
            &mut witness,
            &method_entries,
            type_entity,
            *source,
            &proto_subst,
            &impl_type_arg_tys,
            prefer_source,
        );

        // Bind associated types
        bind_associated_types(
            ctx,
            &mut witness,
            type_entity,
            &extensions,
            *protocol,
            *source,
            witness_impl_ty,
        );

        ctx.module.add_witness(witness);
    }
}

/// Bind each protocol method to a concrete impl function on `witness`. Faithful
/// extraction of the original inline binding (type-member discovery →
/// conformance-providing protocol-extension → protocol-extension default).
///
/// When `prefer_source` is set (specialized concrete witnesses), the source
/// extension's OWN impls win over the merged type-member discovery, so
/// `extend Box[lang.i64]: P` binds its own method rather than the generic one.
/// With `prefer_source == false` this reproduces the original behavior exactly.
#[allow(clippy::too_many_arguments)]
fn bind_witness_methods(
    ctx: &mut LowerCtx,
    witness: &mut WitnessDef,
    method_entries: &[(WitnessMethodKey, String, ProtocolMember)],
    type_entity: Entity,
    source: Entity,
    proto_subst: &std::collections::HashMap<Entity, TyId>,
    impl_type_arg_tys: &[TyId],
    prefer_source: bool,
) {
    for (method_key, method_name, member) in method_entries {
        let expected_param_types = expected_param_types_for(ctx, member.entity, proto_subst);

        let lookup_name = method_name.strip_suffix(".set").unwrap_or(method_name);

        // Specialized concrete witness: try the source extension's own impls first.
        let mut impl_func = None;
        if prefer_source {
            let source_children: Vec<Entity> = ctx.world.children_of(source).to_vec();
            impl_func = if method_name.ends_with(".set") {
                find_setter_among(ctx, &source_children)
            } else {
                find_impl_among(
                    ctx,
                    &source_children,
                    method_name,
                    Some(&method_key.labels),
                    expected_param_types.as_deref(),
                )
            };
        }

        if impl_func.is_none() {
            let candidates = ctx.query.query(TypeMembersByName {
                type_entity,
                name: lookup_name.to_string(),
                context: type_entity,
                root: ctx.root,
            });
            let type_side: Vec<Entity> = candidates
                .iter()
                .filter(|tm| {
                    matches!(
                        tm.source,
                        TypeMemberSource::Direct
                            | TypeMemberSource::Extension(_)
                            | TypeMemberSource::ProtocolExtension { .. }
                    )
                })
                .map(|tm| tm.entity)
                .collect();

            impl_func = if method_name.ends_with(".set") {
                find_setter_among(ctx, &type_side)
            } else {
                find_impl_among(
                    ctx,
                    &type_side,
                    method_name,
                    Some(&method_key.labels),
                    expected_param_types.as_deref(),
                )
            };
        }

        if let Some(impl_func) = impl_func {
            ctx.register_name(impl_func);
            witness.add_method(WitnessMethodBinding::new(
                method_key.clone(),
                impl_func,
                impl_type_arg_tys.to_vec(),
            ));
            continue;
        }

        // Conformance-providing protocol extension: when a blanket like
        // `extend Equatable: NotEqual[Self]` provides the conformance,
        // search the extension's children for the method implementation.
        // Only applies when the extension targets a protocol, not a concrete type.
        let source_is_protocol_ext =
            matches!(ctx.world.get::<NodeKind>(source), Some(NodeKind::Extension))
                && source != type_entity
                && ctx
                    .query
                    .query(ExtensionTargetEntity {
                        extension: source,
                        root: ctx.root,
                    })
                    .is_some_and(|target| {
                        matches!(ctx.world.get::<NodeKind>(target), Some(NodeKind::Protocol))
                    });
        if source_is_protocol_ext {
            let ext_children: Vec<Entity> = ctx.world.children_of(source).to_vec();
            let ext_impl = if method_name.ends_with(".set") {
                find_setter_among(ctx, &ext_children)
            } else {
                find_impl_among(
                    ctx,
                    &ext_children,
                    lookup_name,
                    Some(&method_key.labels),
                    None,
                )
            };
            if let Some(impl_func) = ext_impl {
                ctx.register_name(impl_func);
                witness.add_method(WitnessMethodBinding::new(
                    method_key.clone(),
                    impl_func,
                    impl_type_arg_tys.to_vec(),
                ));
                continue;
            }
        }

        // Protocol extension default
        if member.extension.is_some() {
            let bind_entity = if method_name.ends_with(".set") {
                find_setter_among(ctx, &[member.entity]).unwrap_or(member.entity)
            } else {
                member.entity
            };
            ctx.register_name(bind_entity);
            witness.add_method(WitnessMethodBinding::new(
                method_key.clone(),
                bind_entity,
                impl_type_arg_tys.to_vec(),
            ));
        }
    }
}

/// If `source` is an extension whose target type has ≥1 concrete (non-param)
/// type arg (e.g. `extend Box[lang.i64]`), lower those args to `TyId`s. Returns
/// `None` for fully-generic extensions (`extend Box[T]`) and arg-less targets.
fn lower_concrete_target_args(ctx: &mut LowerCtx, source: Entity) -> Option<Vec<TyId>> {
    let hir_args = ctx.query.query(LowerExtensionTargetTypeArgs {
        extension: source,
        root: ctx.root,
    })?;
    if hir_args.is_empty() || !hir_args.iter().any(|t| !matches!(t, HirTy::Param(..))) {
        return None;
    }
    Some(hir_args.iter().map(|h| lower_type(ctx, h)).collect())
}

fn lower_protocol_type_args(
    ctx: &mut LowerCtx,
    type_entity: Entity,
    ast_type_args: &[AstType],
) -> Vec<TyId> {
    ast_type_args
        .iter()
        .map(|ast_ty| {
            let hir_ty =
                kestrel_hir_lower::lower_ast_type(&ctx.query, type_entity, ctx.root, ast_ty);
            lower_type(ctx, &hir_ty)
        })
        .collect()
}

fn protocol_type_param_entities(ctx: &LowerCtx, protocol: Entity) -> Vec<Entity> {
    ctx.world
        .get::<TypeParams>(protocol)
        .map(|tp| tp.0.clone())
        .unwrap_or_default()
}

fn expected_param_types_for(
    ctx: &mut LowerCtx,
    member_entity: Entity,
    proto_subst: &std::collections::HashMap<Entity, TyId>,
) -> Option<Vec<TyId>> {
    ctx.world.get::<Callable>(member_entity)?;
    let tys = resolve_callable_types(ctx, member_entity);
    let tys: Option<Vec<TyId>> = tys.into_iter().collect();
    tys.map(|v| {
        v.into_iter()
            .map(|t| {
                let mut subst = SubstMap::new();
                for (entity, ty_id) in proto_subst {
                    subst.type_params.insert(*entity, *ty_id);
                }
                substitute(&mut ctx.module.ty_arena, t, &subst)
            })
            .collect()
    })
}

fn protocol_member_name(ctx: &LowerCtx, member: &ProtocolMember) -> String {
    ctx.world
        .get::<Name>(member.entity)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| {
            if ctx.world.get::<SubscriptMarker>(member.entity).is_some() {
                "subscript".to_string()
            } else {
                "init".to_string()
            }
        })
}

fn find_impl_among(
    ctx: &mut LowerCtx,
    candidates: &[Entity],
    method_name: &str,
    required_labels: Option<&[Option<String>]>,
    expected_param_types: Option<&[TyId]>,
) -> Option<Entity> {
    // Two-pass: param-type matching first, label-only fallback
    for &c in candidates {
        if matches_candidate(ctx, c, method_name, required_labels, expected_param_types) {
            return Some(c);
        }
    }
    candidates
        .iter()
        .find(|&&c| matches_candidate(ctx, c, method_name, required_labels, None))
        .copied()
}

fn find_setter_among(ctx: &LowerCtx, candidates: &[Entity]) -> Option<Entity> {
    for &c in candidates {
        if !matches!(
            ctx.world.get::<NodeKind>(c),
            Some(NodeKind::Field | NodeKind::Subscript)
        ) {
            continue;
        }
        for &gc in ctx.world.children_of(c) {
            if ctx.world.get::<NodeKind>(gc) == Some(&NodeKind::Setter) {
                return Some(gc);
            }
        }
    }
    None
}

fn matches_candidate(
    ctx: &mut LowerCtx,
    child: Entity,
    method_name: &str,
    required_labels: Option<&[Option<String>]>,
    expected_param_types: Option<&[TyId]>,
) -> bool {
    let Some(kind) = ctx.world.get::<NodeKind>(child).cloned() else {
        return false;
    };
    match kind {
        NodeKind::Function | NodeKind::Subscript => {
            let name = ctx
                .world
                .get::<Name>(child)
                .map(|n| n.0.clone())
                .unwrap_or_default();
            name == method_name
                && candidate_labels_match(ctx, child, required_labels)
                && expected_param_types
                    .map(|expected| candidate_param_types_match(ctx, child, expected))
                    .unwrap_or(true)
        },
        NodeKind::Field if ctx.world.get::<Callable>(child).is_some() => {
            let name = ctx
                .world
                .get::<Name>(child)
                .map(|n| n.0.clone())
                .unwrap_or_default();
            name == method_name
                && candidate_labels_match(ctx, child, required_labels)
                && expected_param_types
                    .map(|expected| candidate_param_types_match(ctx, child, expected))
                    .unwrap_or(true)
        },
        NodeKind::Initializer if method_name == "init" => {
            candidate_labels_match(ctx, child, required_labels)
                && expected_param_types
                    .map(|expected| candidate_param_types_match(ctx, child, expected))
                    .unwrap_or(true)
        },
        _ => false,
    }
}

fn candidate_labels_match(
    ctx: &LowerCtx,
    candidate: Entity,
    required_labels: Option<&[Option<String>]>,
) -> bool {
    match required_labels {
        Some(labels) => ctx
            .world
            .get::<Callable>(candidate)
            .map(|c| {
                c.params.len() == labels.len()
                    && c.params
                        .iter()
                        .zip(labels)
                        .all(|(p, l)| p.label.as_ref() == l.as_ref())
            })
            .unwrap_or(false),
        None => true,
    }
}

fn candidate_param_types_match(ctx: &mut LowerCtx, candidate: Entity, expected: &[TyId]) -> bool {
    let tys = resolve_callable_types(ctx, candidate);
    if tys.len() != expected.len() {
        return false;
    }
    tys.iter().zip(expected).all(|(got, want)| match got {
        Some(g) => *g == *want,
        None => true,
    })
}

fn get_param_labels(ctx: &LowerCtx, method_entity: Entity) -> Option<Vec<Option<String>>> {
    let callable = ctx.world.get::<Callable>(method_entity)?;
    Some(callable.params.iter().map(|p| p.label.clone()).collect())
}

fn bind_associated_types(
    ctx: &mut LowerCtx,
    witness: &mut WitnessDef,
    type_entity: Entity,
    extensions: &[Entity],
    protocol: Entity,
    source: Entity,
    impl_ty: TyId,
) {
    let assoc_members = ctx.query.query(ProtocolAssociatedTypes {
        protocol,
        root: ctx.root,
    });

    for member in assoc_members {
        let Some(_name) = ctx.world.get::<Name>(member.entity).map(|n| n.0.clone()) else {
            continue;
        };
        if let Some(ty) = find_associated_type(ctx, type_entity, member.entity) {
            witness.add_type_binding(member.entity, ty);
            continue;
        }
        let mut found = false;
        for &ext in extensions {
            if let Some(ty) = find_associated_type(ctx, ext, member.entity) {
                witness.add_type_binding(member.entity, ty);
                found = true;
                break;
            }
        }
        if found {
            continue;
        }
        // Blanket conformances
        if source != type_entity
            && let Some(ty) = find_associated_type(ctx, source, member.entity)
        {
            let ty = replace_self_type(ctx, ty, impl_ty, protocol);
            witness.add_type_binding(member.entity, ty);
        }
    }
}

pub(crate) fn find_associated_type(
    ctx: &mut LowerCtx,
    parent: Entity,
    assoc_entity: Entity,
) -> Option<TyId> {
    let target_name = ctx.world.get::<Name>(assoc_entity)?.0.clone();
    for &child in ctx.world.children_of(parent) {
        if ctx.world.get::<NodeKind>(child) != Some(&NodeKind::TypeAlias) {
            continue;
        }
        let child_name = ctx.world.get::<Name>(child)?.0.clone();
        if child_name != target_name {
            continue;
        }
        let ty = resolve_type_annotation(ctx, child);
        let is_unit = ctx.module.ty_arena.get(ty) == &MirTy::Tuple(vec![]);
        if !is_unit {
            return Some(ty);
        }
    }
    None
}

/// Replace the protocol's Self type with the implementing type in a witness binding.
///
/// Protocol Self is `TypeParam(protocol_entity)`. We substitute it with `impl_ty`.
fn replace_self_type(ctx: &mut LowerCtx, ty: TyId, impl_ty: TyId, protocol: Entity) -> TyId {
    let mut subst = SubstMap::new();
    subst.type_params.insert(protocol, impl_ty);
    substitute(&mut ctx.module.ty_arena, ty, &subst)
}
