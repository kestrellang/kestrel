//! Witness generation — creates WitnessDef entries from conformance data.
//!
//! For each type that conforms to a protocol, generates a witness table
//! mapping protocol method names to implementing function entities.

use kestrel_ast_builder::{Callable, NodeKind, Name, TypeParams};
use kestrel_hecs::Entity;
use kestrel_mir::{MethodBinding, MirTy, TypeParamDef, WitnessDef};
use kestrel_name_res::conformances::ConformingProtocols;
use kestrel_name_res::extensions::ExtensionsFor;

use crate::context::LowerCtx;
use crate::ty::resolve_type_annotation;

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
    // Query all protocols this type conforms to
    let protocols = ctx.query.query(ConformingProtocols {
        entity: type_entity,
        root: ctx.root,
    });

    // Get extensions on this type (for finding method implementations)
    let extensions = ctx.query.query(ExtensionsFor {
        target: type_entity,
        root: ctx.root,
    });

    for protocol in &protocols {
        let mut witness = WitnessDef::new(impl_ty.clone(), *protocol);
        ctx.register_name(*protocol);

        // Collect type params from the implementing type
        if let Some(tp) = ctx.world.get::<TypeParams>(type_entity) {
            for &tp_entity in &tp.0 {
                ctx.register_name(tp_entity);
                let tp_name = ctx
                    .world
                    .get::<Name>(tp_entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
                witness.type_params.push(TypeParamDef::new(tp_entity, tp_name));
            }
        }

        // Get protocol's required methods
        let proto_methods = collect_protocol_methods(ctx, *protocol);

        // Try to bind each protocol method
        for (method_name, method_entity) in &proto_methods {
            // Get protocol method's parameter labels for init disambiguation
            let proto_labels = get_init_labels(ctx, *method_entity);

            // Search the type's own children first
            if let Some(impl_func) =
                find_method_by_name(ctx, type_entity, method_name, proto_labels.as_deref())
            {
                ctx.register_name(impl_func);
                witness.bind_method(
                    method_name,
                    MethodBinding::direct(impl_func, vec![]),
                );
                continue;
            }

            // Search extensions on the type
            let mut found = false;
            for &ext in &extensions {
                if let Some(impl_func) = find_method_by_name(ctx, ext, method_name, proto_labels.as_deref()) {
                    ctx.register_name(impl_func);
                    witness.bind_method(
                        method_name,
                        MethodBinding::direct(impl_func, vec![]),
                    );
                    found = true;
                    break;
                }
            }
            if found {
                continue;
            }

            // Search protocol extensions for default implementations
            let proto_extensions = ctx.query.query(ExtensionsFor {
                target: *protocol,
                root: ctx.root,
            });
            for &proto_ext in &proto_extensions {
                if let Some(impl_func) = find_method_by_name(ctx, proto_ext, method_name, proto_labels.as_deref()) {
                    ctx.register_name(impl_func);
                    witness.bind_method(
                        method_name,
                        MethodBinding::extension(impl_func, vec![], *protocol),
                    );
                    break;
                }
            }
        }

        // Bind associated types
        bind_associated_types(ctx, &mut witness, type_entity, &extensions, *protocol);

        ctx.module.add_witness(witness);
    }
}

/// Collect all method names from a protocol and its parent protocols (recursively).
/// This ensures witnesses include bindings for inherited methods (e.g., Comparable
/// witnesses include Less.lessThan, Greater.greaterThan, etc.).
fn collect_protocol_methods(ctx: &mut LowerCtx, protocol: Entity) -> Vec<(String, Entity)> {
    let mut methods = Vec::new();
    let mut seen = std::collections::HashSet::new();
    collect_protocol_methods_recursive(ctx, protocol, &mut methods, &mut seen);
    methods
}

fn collect_protocol_methods_recursive(
    ctx: &mut LowerCtx,
    protocol: Entity,
    methods: &mut Vec<(String, Entity)>,
    seen: &mut std::collections::HashSet<Entity>,
) {
    if !seen.insert(protocol) {
        return; // avoid cycles
    }

    // Collect this protocol's own methods
    for &child in ctx.world.children_of(protocol) {
        let Some(kind) = ctx.world.get::<NodeKind>(child) else {
            continue;
        };
        if *kind == NodeKind::Function || *kind == NodeKind::Subscript || *kind == NodeKind::Initializer {
            let name = ctx
                .world
                .get::<Name>(child)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| match kind {
                    NodeKind::Initializer => "init".to_string(),
                    NodeKind::Subscript => "subscript".to_string(),
                    _ => String::new(),
                });
            methods.push((name, child));
        }
    }

    // Recurse into parent protocols
    let conformances = ctx.query.query(ConformingProtocols {
        entity: protocol,
        root: ctx.root,
    });
    for parent in conformances {
        collect_protocol_methods_recursive(ctx, parent, methods, seen);
    }
}

/// Find a method by name among an entity's children.
/// Find a method implementation by name. For initializers, also match by
/// parameter labels to disambiguate between multiple inits (e.g.,
/// init(from:) vs init(floatLiteral:) vs init(intLiteral:)).
fn find_method_by_name(
    ctx: &LowerCtx,
    parent: Entity,
    method_name: &str,
    required_labels: Option<&[Option<String>]>,
) -> Option<Entity> {
    for &child in ctx.world.children_of(parent) {
        let Some(kind) = ctx.world.get::<NodeKind>(child) else {
            continue;
        };
        match kind {
            NodeKind::Function | NodeKind::Subscript => {
                let name = ctx.world.get::<Name>(child).map(|n| n.0.as_str()).unwrap_or_default();
                if name == method_name {
                    return Some(child);
                }
            }
            NodeKind::Initializer if method_name == "init" => {
                // Match by parameter labels if provided
                if let Some(labels) = required_labels {
                    if let Some(callable) = ctx.world.get::<Callable>(child) {
                        if callable.params.len() == labels.len()
                            && callable.params.iter().zip(labels).all(|(p, l)| p.label.as_ref() == l.as_ref())
                        {
                            return Some(child);
                        }
                    }
                } else {
                    return Some(child);
                }
            }
            _ => {}
        }
    }
    None
}

/// Get the parameter labels for a protocol method (used to disambiguate inits).
fn get_init_labels(ctx: &LowerCtx, method_entity: Entity) -> Option<Vec<Option<String>>> {
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
    // Collect associated type names from the protocol
    let assoc_types: Vec<String> = ctx
        .world
        .children_of(protocol)
        .iter()
        .filter(|&&child| {
            ctx.world.get::<NodeKind>(child) == Some(&NodeKind::TypeAlias)
        })
        .filter_map(|&child| {
            ctx.world.get::<Name>(child).map(|n| n.0.clone())
        })
        .collect();

    for assoc_name in &assoc_types {
        // Look for a type alias child with this name on the type or its extensions
        if let Some(ty) = find_associated_type(ctx, type_entity, assoc_name) {
            witness.bind_type(assoc_name, ty);
            continue;
        }
        for &ext in extensions {
            if let Some(ty) = find_associated_type(ctx, ext, assoc_name) {
                witness.bind_type(assoc_name, ty);
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
        if ty != MirTy::Unit {
            return Some(ty);
        }
    }
    None
}
