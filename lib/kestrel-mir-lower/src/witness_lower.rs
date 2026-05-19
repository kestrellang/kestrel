//! Witness generation — creates WitnessDef entries from conformance data.
//!
//! For each type that conforms to a protocol, generates a witness table
//! mapping protocol method names to implementing function entities.

use std::collections::HashMap;

use kestrel_ast::AstType;
use kestrel_ast_builder::{
    Callable, Name, NodeKind, Settable, Subscript as SubscriptMarker, TypeParams,
};
use kestrel_hecs::Entity;
use kestrel_mir::{MethodBinding, MirTy, TypeParamDef, WitnessDef, WitnessMethodKey};
use kestrel_name_res::conformances::ConformingProtocolInstantiations;
use kestrel_name_res::extensions::ExtensionsFor;
use kestrel_name_res::{
    ProtocolAssociatedTypes, ProtocolMember, ProtocolMembers, TypeMemberSource, TypeMembersByName,
};

use crate::context::LowerCtx;
use crate::ty::{lower_type, resolve_callable_types, resolve_type_annotation};

/// Generate witness tables for all struct and enum entities.
pub fn lower_witnesses(ctx: &mut LowerCtx) {
    // Collect all struct/enum entities
    let type_entities: Vec<(Entity, MirTy)> = ctx
        .module
        .structs
        .iter()
        .map(|s| {
            let type_args: Vec<MirTy> = s
                .type_params
                .iter()
                .map(|tp| MirTy::TypeParam(tp.entity))
                .collect();
            let ty = if type_args.is_empty() {
                MirTy::Named {
                    entity: s.entity,
                    type_args: vec![],
                }
            } else {
                MirTy::Named {
                    entity: s.entity,
                    type_args,
                }
            };
            (s.entity, ty)
        })
        .collect();

    let enum_entities: Vec<(Entity, MirTy)> = ctx
        .module
        .enums
        .iter()
        .map(|e| {
            let type_args: Vec<MirTy> = e
                .type_params
                .iter()
                .map(|tp| MirTy::TypeParam(tp.entity))
                .collect();
            let ty = MirTy::Named {
                entity: e.entity,
                type_args,
            };
            (e.entity, ty)
        })
        .collect();

    // Generate witnesses for each type
    for (entity, impl_ty) in type_entities.into_iter().chain(enum_entities) {
        lower_witnesses_for_type(ctx, entity, impl_ty);
    }
}

/// Generate witnesses for a single type entity.
fn lower_witnesses_for_type(ctx: &mut LowerCtx, type_entity: Entity, impl_ty: MirTy) {
    // Each (protocol, type_args) pair becomes its own witness. This is what
    // distinguishes `Int64: Convertible[Int8]` from `Int64: Convertible[Int16]`
    // — without it, both would collapse to a single Convertible witness with
    // one method binding, and `Int64(from: x)` would always dispatch through
    // the first `init(from:)` overload regardless of x's type.
    let instantiations = ctx.query.query(ConformingProtocolInstantiations {
        entity: type_entity,
        root: ctx.root,
    });

    // Get extensions on this type (for finding method implementations)
    let extensions = ctx.query.query(ExtensionsFor {
        target: type_entity,
        root: ctx.root,
    });

    for (protocol, source, ast_type_args) in &instantiations {
        // Use the conformance source as the hir-lower context when it's an
        // extension introducing free type params on the protocol RHS
        // (e.g., `extend Int64: ArrayIndex[T]`); otherwise fall back to
        // `type_entity` so existing direct-conformance and inheritance
        // paths keep their historical resolution scope.
        let owner_for_args = if matches!(
            ctx.world.get::<NodeKind>(*source),
            Some(NodeKind::Extension)
        ) {
            *source
        } else {
            type_entity
        };
        let proto_type_args = lower_protocol_type_args(ctx, owner_for_args, ast_type_args);

        let mut witness = WitnessDef::new(impl_ty.clone(), *protocol);
        ctx.register_name(*protocol);

        // Populate protocol_type_args by the protocol's type param names.
        // `Convertible[From]` with `[Int16]` → {"From": Int16}.
        let proto_tp_entities = protocol_type_param_entities(ctx, *protocol);
        let proto_subst: HashMap<Entity, MirTy> = proto_tp_entities
            .iter()
            .zip(proto_type_args.iter())
            .map(|(e, t)| (*e, t.clone()))
            .collect();
        for (tp_entity, ty) in proto_tp_entities.iter().zip(proto_type_args.iter()) {
            let tp_name = ctx
                .world
                .get::<Name>(*tp_entity)
                .map(|n| n.0.clone())
                .unwrap_or_default();
            witness.protocol_type_args.insert(tp_name, ty.clone());
        }

        // Collect type params from the implementing type
        if let Some(tp) = ctx.world.get::<TypeParams>(type_entity) {
            for &tp_entity in &tp.0 {
                ctx.register_name(tp_entity);
                let tp_name = ctx
                    .world
                    .get::<Name>(tp_entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
                witness
                    .type_params
                    .push(TypeParamDef::new(tp_entity, tp_name));
            }
        }

        // When the conformance comes from an extension that introduces
        // its own free type params (`extend Int64: ArrayIndex[T]`), those
        // params live on the extension entity, not on the conforming type.
        // Record them on the witness so monomorphization can substitute
        // them from the call site's protocol type args.
        if matches!(
            ctx.world.get::<NodeKind>(*source),
            Some(NodeKind::Extension)
        ) && let Some(tp) = ctx.world.get::<TypeParams>(*source)
        {
            for &tp_entity in &tp.0 {
                if witness.type_params.iter().any(|t| t.entity == tp_entity) {
                    continue;
                }
                ctx.register_name(tp_entity);
                let tp_name = ctx
                    .world
                    .get::<Name>(tp_entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
                witness
                    .type_params
                    .push(TypeParamDef::new(tp_entity, tp_name));
            }
        }

        // Every method/property requirement the protocol exposes — direct,
        // extension defaults, inherited from parent protocols, and parents'
        // extension defaults — all in one pass. Fields that are Settable
        // need a second `<name>.set` entry so assignment dispatches through
        // the witness.
        let proto_members = ctx.query.query(ProtocolMembers {
            protocol: *protocol,
            root: ctx.root,
        });
        let method_entries: Vec<(WitnessMethodKey, String, ProtocolMember)> = proto_members
            .into_iter()
            .flat_map(|m| {
                let name = protocol_member_name(ctx, &m);
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
                        m,
                    ));
                }
                entries
            })
            .collect();

        // Try to bind each protocol method
        for (method_key, method_name, member) in &method_entries {
            // Compute expected param types after substituting the protocol's
            // type args. For `Convertible[Int16].init(from: From)` this becomes
            // `init(from: Int16)`, so we pick the right overload instead of
            // the first one.
            let expected_param_types = expected_param_types_for(ctx, member.entity, &proto_subst);

            // Discover type-side candidates (direct children + own extensions)
            // via the unified TypeMembersByName query. Setter dispatch keys off
            // the field's base name; the setter resolver finds the Setter child
            // of the Field below.
            let lookup_name = method_name.strip_suffix(".set").unwrap_or(method_name);
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
                        TypeMemberSource::Direct | TypeMemberSource::Extension(_)
                    )
                })
                .map(|tm| tm.entity)
                .collect();

            let impl_func = if method_name.ends_with(".set") {
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

            if let Some(impl_func) = impl_func {
                ctx.register_name(impl_func);
                witness.bind_method(method_key.clone(), MethodBinding::direct(impl_func, vec![]));
                continue;
            }

            // Fall back to the protocol extension's default implementation,
            // if this member came from one. ProtocolMembers already walked
            // the extensions, so the default impl is right there on `member`.
            // For setter keys (`<name>.set`), bind to the Setter child entity
            // so the monomorphizer calls the setter function (which takes
            // `[self, index, newValue]`), not the getter.
            if member.extension.is_some() {
                let bind_entity = if method_name.ends_with(".set") {
                    find_setter_among(ctx, &[member.entity]).unwrap_or(member.entity)
                } else {
                    member.entity
                };
                ctx.register_name(bind_entity);
                witness.bind_method(
                    method_key.clone(),
                    MethodBinding::extension(bind_entity, vec![], *protocol),
                );
            }
        }

        // Bind associated types
        bind_associated_types(ctx, &mut witness, type_entity, &extensions, *protocol);

        ctx.module.add_witness(witness);
    }
}

/// Lower a list of conformance-declaration AstTypes (e.g. the `Int16` in
/// `Convertible[Int16]`) to MirTy. Uses `type_entity` as the resolution
/// context so names resolve relative to the conforming type's scope.
fn lower_protocol_type_args(
    ctx: &mut LowerCtx,
    type_entity: Entity,
    ast_type_args: &[AstType],
) -> Vec<MirTy> {
    let hir_tys: Vec<_> = ast_type_args
        .iter()
        .map(|ast_ty| kestrel_hir_lower::lower_ast_type(&ctx.query, type_entity, ctx.root, ast_ty))
        .collect();
    hir_tys
        .iter()
        .map(|hir_ty| lower_type(ctx, hir_ty))
        .collect()
}

/// Return the protocol's declared type-parameter entities, in order.
fn protocol_type_param_entities(ctx: &LowerCtx, protocol: Entity) -> Vec<Entity> {
    ctx.world
        .get::<TypeParams>(protocol)
        .map(|tp| tp.0.clone())
        .unwrap_or_default()
}

/// Compute the protocol method's declared parameter types with the witness's
/// protocol type args substituted in. Returns `None` for member kinds that
/// aren't callable (which short-circuits the param-type check in
/// `find_method_by_name`).
fn expected_param_types_for(
    ctx: &mut LowerCtx,
    member_entity: Entity,
    proto_subst: &HashMap<Entity, MirTy>,
) -> Option<Vec<MirTy>> {
    // Only Callable members have param types worth substituting. For
    // `.set` method names the member is a Field, which we let match by
    // label (the Field has no param list of its own).
    ctx.world.get::<Callable>(member_entity)?;
    let tys = resolve_callable_types(ctx, member_entity);
    // If any param lacks a type annotation, skip the check — fall back to
    // label-only matching so we don't spuriously reject valid impls.
    let tys: Option<Vec<MirTy>> = tys.into_iter().collect();
    tys.map(|v| {
        v.into_iter()
            .map(|t| substitute_type_params(&t, proto_subst))
            .collect()
    })
}

/// Minimal TypeParam → concrete substitution for MirTy. We need this inside
/// mir-lower where `kestrel-codegen`'s `substitute_type` isn't available.
fn substitute_type_params(ty: &MirTy, subst: &HashMap<Entity, MirTy>) -> MirTy {
    match ty {
        MirTy::TypeParam(e) => subst.get(e).cloned().unwrap_or_else(|| ty.clone()),
        MirTy::Pointer(inner) => MirTy::Pointer(Box::new(substitute_type_params(inner, subst))),
        MirTy::Ref(inner) => MirTy::Ref(Box::new(substitute_type_params(inner, subst))),
        MirTy::RefMut(inner) => MirTy::RefMut(Box::new(substitute_type_params(inner, subst))),
        MirTy::Tuple(elems) => MirTy::Tuple(
            elems
                .iter()
                .map(|t| substitute_type_params(t, subst))
                .collect(),
        ),
        MirTy::Named { entity, type_args } => MirTy::Named {
            entity: *entity,
            type_args: type_args
                .iter()
                .map(|t| substitute_type_params(t, subst))
                .collect(),
        },
        MirTy::FuncThin { params, ret } => MirTy::FuncThin {
            params: params
                .iter()
                .map(|t| substitute_type_params(t, subst))
                .collect(),
            ret: Box::new(substitute_type_params(ret, subst)),
        },
        MirTy::FuncThick { params, ret } => MirTy::FuncThick {
            params: params
                .iter()
                .map(|t| substitute_type_params(t, subst))
                .collect(),
            ret: Box::new(substitute_type_params(ret, subst)),
        },
        _ => ty.clone(),
    }
}

/// Resolve a ProtocolMember's dispatch name — `Name` component if present,
/// else `"init"` for nameless initializers or `"subscript"` for nameless
/// subscripts.
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

/// Find a method implementation among a list of candidate entities.
///
/// Two-pass: first with parameter-type matching (distinguishes
/// `init(from: Int8)` from `init(from: Int16)` so witnesses for
/// `Convertible[Int8]` vs `[Int16]` pick the right overload), then
/// label-only as fallback for members whose param types don't exist
/// in MirTy form (e.g. associated-type params that reference `Self`).
fn find_impl_among(
    ctx: &mut LowerCtx,
    candidates: &[Entity],
    method_name: &str,
    required_labels: Option<&[Option<String>]>,
    expected_param_types: Option<&[MirTy]>,
) -> Option<Entity> {
    for &c in candidates {
        if matches_candidate(ctx, c, method_name, required_labels, expected_param_types) {
            return Some(c);
        }
    }
    for &c in candidates {
        if matches_candidate(ctx, c, method_name, required_labels, None) {
            return Some(c);
        }
    }
    None
}

/// For setter dispatch (`<field>.set`): pick the Setter child of the first
/// Field candidate. Field candidates come from `TypeMembersByName` keyed on
/// the field's base name (Fields carry `Gettable`, so they surface there).
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

/// Check whether `child` satisfies name + optional label + optional
/// param-type constraints.
fn matches_candidate(
    ctx: &mut LowerCtx,
    child: Entity,
    method_name: &str,
    required_labels: Option<&[Option<String>]>,
    expected_param_types: Option<&[MirTy]>,
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
        // Computed property: Field with a body (Callable) — its getter
        // satisfies the protocol's property requirement.
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
            // Label check
            if !candidate_labels_match(ctx, child, required_labels) {
                return false;
            }
            // Param-type check (only when expected types are provided)
            match expected_param_types {
                Some(expected) => candidate_param_types_match(ctx, child, expected),
                None => true,
            }
        },
        _ => false,
    }
}

fn candidate_labels_match(
    ctx: &mut LowerCtx,
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

/// Check whether the candidate callable's parameter types equal `expected`
/// pairwise. Params without a type annotation don't participate (they match
/// anything) — the expected list already carries concrete protocol substitution.
fn candidate_param_types_match(ctx: &mut LowerCtx, candidate: Entity, expected: &[MirTy]) -> bool {
    let tys = resolve_callable_types(ctx, candidate);
    if tys.len() != expected.len() {
        return false;
    }
    tys.iter().zip(expected).all(|(got, want)| match got {
        Some(g) => g == want,
        None => true,
    })
}

/// Get the parameter labels for a callable protocol member.
fn get_param_labels(ctx: &LowerCtx, method_entity: Entity) -> Option<Vec<Option<String>>> {
    let callable = ctx.world.get::<Callable>(method_entity)?;
    Some(callable.params.iter().map(|p| p.label.clone()).collect())
}

/// Bind associated types from the implementing type or its extensions.
fn bind_associated_types(
    ctx: &mut LowerCtx,
    witness: &mut WitnessDef,
    type_entity: Entity,
    extensions: &[Entity],
    protocol: Entity,
) {
    // Associated types the protocol (and its parents/extensions) declares.
    let assoc_members = ctx.query.query(ProtocolAssociatedTypes {
        protocol,
        root: ctx.root,
    });

    for member in assoc_members {
        let Some(name) = ctx.world.get::<Name>(member.entity).map(|n| n.0.clone()) else {
            continue;
        };
        // The conforming type (or one of its extensions) must supply the
        // binding — `type Item = Int64`-style. Parent-protocol extensions'
        // default bindings are handled via the same name-lookup below since
        // those also live on children of extensions the type carries.
        if let Some(ty) = find_associated_type(ctx, type_entity, &name) {
            witness.bind_type(&name, ty);
            continue;
        }
        for &ext in extensions {
            if let Some(ty) = find_associated_type(ctx, ext, &name) {
                witness.bind_type(&name, ty);
                break;
            }
        }
    }
}

/// Find an associated type binding on an entity (type alias with TypeAnnotation).
fn find_associated_type(ctx: &mut LowerCtx, parent: Entity, name: &str) -> Option<MirTy> {
    for &child in ctx.world.children_of(parent) {
        let Some(kind) = ctx.world.get::<NodeKind>(child) else {
            continue;
        };
        if *kind != NodeKind::TypeAlias {
            continue;
        }
        let child_name = ctx.world.get::<Name>(child)?.0.as_str();
        if child_name != name {
            continue;
        }
        // Resolve the type alias's TypeAnnotation
        let ty = resolve_type_annotation(ctx, child);
        if !ty.is_unit() {
            return Some(ty);
        }
    }
    None
}
