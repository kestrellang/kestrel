//! Type path resolution.
//!
//! Resolves type names (from AstType::Named paths) to type entities.
//! Used by HIR lowering to convert AstType → HirTy.
//!
//! All `lang.*` types (i64, str, ptr[T], etc.) are real entities seeded
//! by `seed_lang_module()` and resolved through normal name resolution.
//! Only `Self` gets special handling as a keyword.

use kestrel_ast_builder::{
    ConformanceItem, Conformances, Name, NodeKind, TypeParams, Typed, WhereClause, WhereConstraint,
};
use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::resolve_name::{NameResolution, ResolveName};
use crate::visibility::VisibleChildrenByName;

// ===== TypeResolution =====

/// Result of resolving a type path.
#[derive(Clone, Debug, Hash)]
pub enum TypeResolution {
    /// Resolved to a type entity (struct, enum, protocol, alias, type param)
    Found(Entity),
    /// Bare `Self` keyword — resolved contextually by the caller
    SelfType,
    /// Name not found
    NotFound(String),
    /// Resolved but entity is not a type
    NotAType(Entity),
}

// ===== ResolveTypePath =====

/// Query: resolve a type path (dotted segments) to a type entity.
///
/// Handles:
/// - `Self` keyword: returns SelfType (or resolves through type param bounds)
/// - Named types: looks up via ResolveName, must have Typed marker
/// - Multi-segment paths: first via ResolveName, rest via VisibleChildrenByName
/// - `lang.*` types: resolved as normal entities (seeded by `seed_lang_module`)
/// - Type parameter associated types: if current is a TypeParameter,
///   checks protocol bounds for matching associated type name
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ResolveTypePath {
    pub segments: Vec<String>,
    pub context: Entity,
    pub root: Entity,
}

impl QueryFn for ResolveTypePath {
    type Output = TypeResolution;

    fn describe(&self) -> String {
        format!(
            "ResolveTypePath({:?}, ctx={:?})",
            self.segments.join("."),
            self.context
        )
    }

    fn execute(&self, ctx: &QueryContext<'_>) -> TypeResolution {
        if self.segments.is_empty() {
            return TypeResolution::NotFound("<empty>".into());
        }

        // Handle "Self" keyword
        if self.segments[0] == "Self" {
            // Multi-segment Self.Item — try resolving through synthetic type param
            if self.segments.len() > 1 {
                if let Some(result) =
                    try_resolve_self_as_type_param(ctx, &self.segments, self.context, self.root)
                {
                    return result;
                }
                // Fallback: resolve Self.Item through the enclosing extension target.
                // For `extend Iterator: Iterable { type Iterable.Item = Self.Item }`,
                // Self resolves to Iterator, then .Item walks Iterator's children.
                if let Some(result) = try_resolve_self_via_extension_target(
                    ctx,
                    &self.segments,
                    self.context,
                    self.root,
                ) {
                    return result;
                }
            }
            // Bare "Self" — return SelfType for contextual resolution by caller
            if self.segments.len() == 1 {
                return TypeResolution::SelfType;
            }
        }

        // Resolve first segment via name resolution
        let first = &self.segments[0];
        let result = ctx.query(ResolveName {
            name: first.clone(),
            context: self.context,
            root: self.root,
        });

        let entity = match result {
            NameResolution::Found(entities) => {
                if self.segments.len() == 1 {
                    // Single segment must be a type
                    match find_type_entity(ctx, &entities) {
                        Some(e) => return TypeResolution::Found(e),
                        None => return TypeResolution::NotAType(entities[0]),
                    }
                } else {
                    // Multi-segment: first can be a module or type, prefer types
                    find_type_entity(ctx, &entities).unwrap_or(entities[0])
                }
            },
            NameResolution::Ambiguous(entities) => {
                if self.segments.len() == 1 {
                    match find_type_entity(ctx, &entities) {
                        Some(e) => return TypeResolution::Found(e),
                        None => return TypeResolution::NotAType(entities[0]),
                    }
                } else {
                    find_type_entity(ctx, &entities).unwrap_or(entities[0])
                }
            },
            NameResolution::NotFound => {
                return TypeResolution::NotFound(first.clone());
            },
        };

        // Multi-segment: walk remaining segments
        let mut current = entity;
        for segment in &self.segments[1..] {
            // Check if current is a type parameter — look for associated types
            // in where-clause bounds (e.g. T.Item where T: Iterator)
            if ctx.get::<NodeKind>(current) == Some(&NodeKind::TypeParameter) {
                if let Some(assoc) =
                    resolve_type_param_assoc(ctx, current, segment, self.context, self.root)
                {
                    current = assoc;
                    continue;
                }
                return TypeResolution::NotFound(segment.clone());
            }

            // Check if current is an associated type (TypeAlias in a protocol) —
            // look for nested associated types via its bounds (e.g. T.Iter.Item)
            if ctx.get::<NodeKind>(current) == Some(&NodeKind::TypeAlias)
                && let Some(assoc) =
                    resolve_assoc_type_nested(ctx, current, segment, self.context, self.root)
                {
                    current = assoc;
                    continue;
                }
                // Fall through to child walk — the alias might have children

            // Otherwise walk children
            let visible = ctx.query(VisibleChildrenByName {
                parent: current,
                name: segment.clone(),
                context: self.context,
            });

            match find_type_entity(ctx, &visible) {
                Some(e) => current = e,
                None if visible.is_empty() => {
                    return TypeResolution::NotFound(segment.clone());
                },
                None => {
                    // Allow modules as intermediate segments (e.g. std.collections.Array)
                    if ctx.get::<NodeKind>(visible[0]) == Some(&NodeKind::Module) {
                        current = visible[0];
                    } else {
                        return TypeResolution::NotAType(visible[0]);
                    }
                },
            }
        }

        TypeResolution::Found(current)
    }
}

/// Find the first entity in the list that is a type (has Typed marker
/// or is a TypeParameter).
fn find_type_entity(ctx: &QueryContext<'_>, entities: &[Entity]) -> Option<Entity> {
    entities
        .iter()
        .find(|&&e| ctx.has::<Typed>(e) || ctx.get::<NodeKind>(e) == Some(&NodeKind::TypeParameter))
        .copied()
}

/// Try to resolve "Self" as a synthetic type parameter in protocol extensions.
///
/// For multi-segment paths like `Self.Item`, checks if "Self" resolves to a
/// TypeParameter (created by the AST builder for protocol extensions). If so,
/// uses the type parameter's bounds to resolve remaining segments.
fn try_resolve_self_as_type_param(
    ctx: &QueryContext<'_>,
    segments: &[String],
    context: Entity,
    root: Entity,
) -> Option<TypeResolution> {
    // Try to resolve "Self" via name resolution
    let result = ctx.query(ResolveName {
        name: "Self".to_string(),
        context,
        root,
    });

    let NameResolution::Found(entities) = result else {
        return None;
    };

    // Only use if it resolves to a single TypeParameter
    let &self_entity = entities.first()?;
    if ctx.get::<NodeKind>(self_entity) != Some(&NodeKind::TypeParameter) {
        return None;
    }

    // Walk remaining segments through type param bounds
    let mut current = self_entity;
    for segment in &segments[1..] {
        if let Some(assoc) = resolve_type_param_assoc(ctx, current, segment, context, root) {
            current = assoc;
        } else if let Some(assoc) = resolve_assoc_type_nested(ctx, current, segment, context, root)
        {
            current = assoc;
        } else {
            return Some(TypeResolution::NotFound(segment.clone()));
        }
    }

    Some(TypeResolution::Found(current))
}

/// Resolve `Self.Item` by finding the enclosing extension, resolving its
/// target type, and walking the remaining segments as children of that type.
///
/// This handles `Self.Item` in extensions where Self isn't a TypeParameter
/// (e.g., `extend Iterator: Iterable { type Iterable.Item = Self.Item }`).
fn try_resolve_self_via_extension_target(
    ctx: &QueryContext<'_>,
    segments: &[String],
    context: Entity,
    root: Entity,
) -> Option<TypeResolution> {
    // Walk up from context to find an enclosing Extension
    let mut current = Some(context);
    let mut extension = None;
    while let Some(entity) = current {
        if ctx.get::<NodeKind>(entity) == Some(&NodeKind::Extension) {
            extension = Some(entity);
            break;
        }
        current = ctx.parent_of(entity);
    }
    let extension = extension?;

    // Resolve the extension's target type
    let target = ctx.query(crate::ExtensionTargetEntity { extension, root })?;

    // Walk remaining segments (after "Self") through the target's children
    // and associated types. For protocols, this finds associated types;
    // for structs, this finds nested types.
    let mut resolved = target;
    for segment in &segments[1..] {
        // Check if resolved is a protocol — look for associated types
        if ctx.get::<NodeKind>(resolved) == Some(&NodeKind::Protocol)
            && let Some(assoc) = crate::resolve_name::find_assoc_type(ctx, resolved, segment) {
                resolved = assoc;
                continue;
            }

        // Check children by name (handles nested types, etc.)
        let visible = ctx.query(crate::VisibleChildrenByName {
            parent: resolved,
            name: segment.clone(),
            context,
        });
        if let Some(entity) = find_type_entity(ctx, &visible) {
            resolved = entity;
        } else {
            return Some(TypeResolution::NotFound(segment.clone()));
        }
    }

    Some(TypeResolution::Found(resolved))
}

/// Resolve an associated type on a type parameter.
///
/// For `T.Item` where `T: Iterator`, find the `Item` associated type
/// by walking the ancestor chain to collect all where-clause bounds.
pub fn resolve_type_param_assoc(
    ctx: &QueryContext<'_>,
    type_param: Entity,
    assoc_name: &str,
    context: Entity,
    root: Entity,
) -> Option<Entity> {
    let tp_name = ctx.get::<Name>(type_param)?;

    // Walk the type param's ancestor chain for where-clause bounds.
    let mut ancestor = ctx.parent_of(type_param);
    while let Some(anc) = ancestor {
        if let Some(found) = search_entity_bounds(ctx, anc, &tp_name.0, assoc_name, root) {
            return Some(found);
        }
        ancestor = ctx.parent_of(anc);
    }

    // Also walk the context's ancestor chain — the context (e.g. a method inside
    // an extension) may have where-clause bounds on a type param defined elsewhere
    // (e.g. `extend Array[T] where T: Iterable` — T is Array's param, but the
    // where-clause is on the extension which is an ancestor of context, not of T).
    let mut ancestor = Some(context);
    while let Some(anc) = ancestor {
        if let Some(found) = search_entity_bounds(ctx, anc, &tp_name.0, assoc_name, root) {
            return Some(found);
        }
        ancestor = ctx.parent_of(anc);
    }

    None
}

/// Search a single entity's where-clause and conformances for bounds
/// on `type_param_name` that contain an associated type `assoc_name`.
fn search_entity_bounds(
    ctx: &QueryContext<'_>,
    entity: Entity,
    type_param_name: &str,
    assoc_name: &str,
    root: Entity,
) -> Option<Entity> {
    if let Some(where_clause) = ctx.get::<WhereClause>(entity) {
        if let Some(found) =
            search_bounds_for_assoc(ctx, where_clause, type_param_name, assoc_name, entity, root)
        {
            return Some(found);
        }
        if let Some(found) = search_inherited_assoc_bounds(
            ctx,
            where_clause,
            type_param_name,
            assoc_name,
            entity,
            root,
        ) {
            return Some(found);
        }
    }
    None
}

/// Search a where-clause for bounds on `type_param_name` that contain
/// a protocol with an associated type named `assoc_name`.
fn search_bounds_for_assoc(
    ctx: &QueryContext<'_>,
    where_clause: &WhereClause,
    type_param_name: &str,
    assoc_name: &str,
    scope: Entity,
    root: Entity,
) -> Option<Entity> {
    for constraint in &where_clause.0 {
        let WhereConstraint::Bound {
            subject, protocols, ..
        } = constraint
        else {
            continue;
        };

        // Check if subject is our type param (single-segment named type)
        let kestrel_ast::AstType::Named { segments, .. } = subject else {
            continue;
        };
        if segments.len() != 1 || segments[0].name != type_param_name {
            continue;
        }

        // Search each protocol bound for the associated type
        if let Some(found) = search_protocols_for_assoc(ctx, protocols, assoc_name, scope, root) {
            return Some(found);
        }
    }
    None
}

/// Search `where T.Item: Protocol` style bounds for an associated type.
///
/// If we're looking for `T.Item.Foo` and there's `where T.Item: HasFoo`,
/// this finds `Foo` in `HasFoo`.
fn search_inherited_assoc_bounds(
    ctx: &QueryContext<'_>,
    where_clause: &WhereClause,
    type_param_name: &str,
    assoc_name: &str,
    scope: Entity,
    root: Entity,
) -> Option<Entity> {
    for constraint in &where_clause.0 {
        let WhereConstraint::Bound {
            subject, protocols, ..
        } = constraint
        else {
            continue;
        };

        // Check if subject is a dotted path starting with our type param
        // (e.g. T.Item, T.Iter, etc.)
        let kestrel_ast::AstType::Named { segments, .. } = subject else {
            continue;
        };
        if segments.len() < 2 || segments[0].name != type_param_name {
            continue;
        }

        // The last segment of the subject is the associated type name
        // If it matches what we resolved to, search the protocols for assoc_name
        // For now, we search all dotted-path bounds for the assoc_name
        if let Some(found) = search_protocols_for_assoc(ctx, protocols, assoc_name, scope, root) {
            return Some(found);
        }
    }
    None
}

/// Search a list of protocol types for an associated type by name.
fn search_protocols_for_assoc(
    ctx: &QueryContext<'_>,
    protocols: &[kestrel_ast::AstType],
    assoc_name: &str,
    scope: Entity,
    root: Entity,
) -> Option<Entity> {
    for proto_type in protocols {
        let kestrel_ast::AstType::Named {
            segments: proto_segs,
            ..
        } = proto_type
        else {
            continue;
        };
        if proto_segs.is_empty() {
            continue;
        }

        // Resolve the protocol via full type path.
        // Use scope's parent to avoid cycles when scope is a protocol
        // (resolving from a protocol re-enters inherited member search).
        let seg_names: Vec<String> = proto_segs.iter().map(|s| s.name.clone()).collect();
        let resolve_ctx = ctx.parent_of(scope).unwrap_or(scope);
        let proto_result = ctx.query(ResolveTypePath {
            segments: seg_names,
            context: resolve_ctx,
            root,
        });

        let TypeResolution::Found(proto_entity) = proto_result else {
            continue;
        };

        if ctx.get::<NodeKind>(proto_entity) != Some(&NodeKind::Protocol) {
            continue;
        }

        // Check protocol's direct children for matching TypeAlias
        if let Some(found) = find_assoc_type(ctx, proto_entity, assoc_name) {
            return Some(found);
        }

        // Also check inherited associated types from parent protocols
        if let Some(found) = find_inherited_assoc_type(ctx, proto_entity, assoc_name, scope, root) {
            return Some(found);
        }
    }
    None
}

/// Resolve a nested associated type on an existing associated type entity.
///
/// For `T.Iter.Item` — after resolving `Iter` to a TypeAlias in a protocol,
/// this looks at `Iter`'s bounds (e.g. `type Iter: Iterator`) to find `Item`.
pub fn resolve_assoc_type_nested(
    ctx: &QueryContext<'_>,
    assoc_type: Entity,
    assoc_name: &str,
    context: Entity,
    root: Entity,
) -> Option<Entity> {
    // The associated type is a TypeAlias inside a protocol.
    // Check if it has bounds via the protocol's where-clause constraints
    // of the form `Subject.AssocTypeName: Protocol`.
    let parent = ctx.parent_of(assoc_type)?;

    // Check direct bounds on the associated type itself (from protocol where-clause)
    if let Some(where_clause) = ctx.get::<WhereClause>(parent) {
        let assoc_self_name = ctx.get::<Name>(assoc_type)?;
        for constraint in &where_clause.0 {
            let WhereConstraint::Bound {
                subject, protocols, ..
            } = constraint
            else {
                continue;
            };

            // Match constraints like `Self.Iter: Iterator` or just `Iter: Iterator`
            let kestrel_ast::AstType::Named { segments, .. } = subject else {
                continue;
            };

            let matches = match segments.len() {
                1 => segments[0].name == assoc_self_name.0,
                2 => {
                    (segments[0].name == "Self"
                        || is_type_param_name(ctx, parent, &segments[0].name))
                        && segments[1].name == assoc_self_name.0
                },
                _ => false,
            };

            if !matches {
                continue;
            }

            if let Some(found) =
                search_protocols_for_assoc(ctx, protocols, assoc_name, parent, root)
            {
                return Some(found);
            }
        }
    }

    // Also check direct conformances on the TypeAlias itself if it declares bounds
    // (e.g. `type Iter: Iterator` in protocol body — conformances on the TypeAlias)
    if let Some(conformances) = ctx.get::<Conformances>(assoc_type) {
        let proto_types: Vec<kestrel_ast::AstType> = conformances
            .0
            .iter()
            .filter_map(|item| {
                let ConformanceItem::Positive(ast_type, _) = item else {
                    return None;
                };
                Some(ast_type.clone())
            })
            .collect();

        if let Some(found) = search_protocols_for_assoc(ctx, &proto_types, assoc_name, parent, root)
        {
            return Some(found);
        }
    }

    // Walk up context to find inherited bounds (e.g. `where I.Iter: Iterator`)
    let mut ancestor = Some(context);
    while let Some(anc) = ancestor {
        if let Some(where_clause) = ctx.get::<WhereClause>(anc) {
            // Look for bounds on paths ending with our associated type name
            for constraint in &where_clause.0 {
                let WhereConstraint::Bound {
                    subject, protocols, ..
                } = constraint
                else {
                    continue;
                };

                let kestrel_ast::AstType::Named { segments, .. } = subject else {
                    continue;
                };

                // Match if the last segment is our associated type's name
                if segments.len() >= 2 {
                    let assoc_self_name = ctx.get::<Name>(assoc_type);
                    if let Some(name) = assoc_self_name
                        && segments.last().map(|s| &s.name) == Some(&name.0)
                            && let Some(found) =
                                search_protocols_for_assoc(ctx, protocols, assoc_name, anc, root)
                            {
                                return Some(found);
                            }
                }
            }
        }
        ancestor = ctx.parent_of(anc);
    }

    None
}

/// Check if a name corresponds to a type parameter of the given entity.
fn is_type_param_name(ctx: &QueryContext<'_>, entity: Entity, name: &str) -> bool {
    if let Some(tps) = ctx.get::<TypeParams>(entity) {
        tps.0
            .iter()
            .any(|&tp| ctx.get::<Name>(tp).is_some_and(|n| n.0 == name))
    } else {
        false
    }
}

/// Find an associated type (TypeAlias child) in a protocol by name.
fn find_assoc_type(ctx: &QueryContext<'_>, protocol: Entity, name: &str) -> Option<Entity> {
    ctx.children_of(protocol)
        .iter()
        .find(|&&child| {
            ctx.get::<NodeKind>(child) == Some(&NodeKind::TypeAlias)
                && ctx.get::<Name>(child).is_some_and(|n| n.0 == name)
        })
        .copied()
}

/// Find an associated type in a protocol's inherited (parent) protocols.
fn find_inherited_assoc_type(
    ctx: &QueryContext<'_>,
    protocol: Entity,
    name: &str,
    scope: Entity,
    root: Entity,
) -> Option<Entity> {
    let conformances = ctx.get::<Conformances>(protocol)?;

    for item in &conformances.0 {
        let ConformanceItem::Positive(ast_type, _) = item else {
            continue;
        };

        let kestrel_ast::AstType::Named { segments, .. } = ast_type else {
            continue;
        };
        if segments.is_empty() {
            continue;
        }

        let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
        // Use scope's parent to avoid cycles when scope is a protocol
        let resolve_ctx = ctx.parent_of(scope).unwrap_or(scope);
        let result = ctx.query(ResolveTypePath {
            segments: seg_names,
            context: resolve_ctx,
            root,
        });

        let TypeResolution::Found(parent_proto) = result else {
            continue;
        };

        if ctx.get::<NodeKind>(parent_proto) != Some(&NodeKind::Protocol) {
            continue;
        }

        if let Some(found) = find_assoc_type(ctx, parent_proto, name) {
            return Some(found);
        }

        // Recursively check grandparent protocols
        if let Some(found) = find_inherited_assoc_type(ctx, parent_proto, name, resolve_ctx, root) {
            return Some(found);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast_builder::Vis;
    use kestrel_hecs::World;

    fn setup() -> (World, Entity) {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        let std = world.spawn();
        world.set(std, NodeKind::Module);
        world.set(std, Name("std".into()));
        world.set_parent(std, root);

        let core = world.spawn();
        world.set(core, NodeKind::Module);
        world.set(core, Name("core".into()));
        world.set_parent(core, std);

        let int64 = world.spawn();
        world.set(int64, NodeKind::Struct);
        world.set(int64, Name("Int64".into()));
        world.set(int64, Vis::Public);
        world.set(int64, Typed);
        world.set_parent(int64, core);

        let bool_t = world.spawn();
        world.set(bool_t, NodeKind::Enum);
        world.set(bool_t, Name("Bool".into()));
        world.set(bool_t, Vis::Public);
        world.set(bool_t, Typed);
        world.set_parent(bool_t, core);

        // User module
        let myapp = world.spawn();
        world.set(myapp, NodeKind::Module);
        world.set(myapp, Name("MyApp".into()));
        world.set_parent(myapp, root);

        (world, root)
    }

    #[test]
    fn resolve_self_type() {
        let (world, root) = setup();
        let ctx = world.query_context();

        let result = ctx.query(ResolveTypePath {
            segments: vec!["Self".into()],
            context: root,
            root,
        });
        assert!(matches!(result, TypeResolution::SelfType));
    }

    #[test]
    fn resolve_lang_int() {
        let (mut world, root) = setup();

        // Seed the lang module so lang.i64 resolves as a real entity
        kestrel_ast_builder::seed_lang_module(&mut world, root);

        let ctx = world.query_context();

        let result = ctx.query(ResolveTypePath {
            segments: vec!["lang".into(), "i64".into()],
            context: root,
            root,
        });
        match result {
            TypeResolution::Found(entity) => {
                assert_eq!(ctx.get::<Name>(entity).unwrap().0, "i64");
            },
            other => panic!("expected Found, got {:?}", other),
        }
    }

    #[test]
    fn resolve_named_type() {
        let (world, root) = setup();
        let ctx = world.query_context();

        // From MyApp, Int64 should be findable via auto-import
        let myapp = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        let result = ctx.query(ResolveTypePath {
            segments: vec!["Int64".into()],
            context: myapp,
            root,
        });
        match result {
            TypeResolution::Found(entity) => {
                assert_eq!(ctx.get::<Name>(entity).unwrap().0, "Int64");
            },
            other => panic!("expected Found, got {:?}", other),
        }
    }

    #[test]
    fn resolve_not_found() {
        let (world, root) = setup();
        let ctx = world.query_context();

        let result = ctx.query(ResolveTypePath {
            segments: vec!["Nonexistent".into()],
            context: root,
            root,
        });
        assert!(matches!(result, TypeResolution::NotFound(_)));
    }

    #[test]
    fn resolve_not_a_type() {
        let (mut world, root) = setup();

        // Add a non-type entity (function) to MyApp
        let myapp = world
            .children_of(root)
            .iter()
            .find(|&&e| world.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        let func = world.spawn();
        world.set(func, NodeKind::Function);
        world.set(func, Name("notAType".into()));
        world.set_parent(func, myapp);

        let ctx = world.query_context();
        let result = ctx.query(ResolveTypePath {
            segments: vec!["notAType".into()],
            context: myapp,
            root,
        });
        assert!(matches!(result, TypeResolution::NotAType(_)));
    }

    /// Verifies that resolving a name from a TypeAlias's own scope finds a
    /// sibling associated type on the parent protocol via scope walking.
    ///
    /// Models `protocol Iterable { type Iter: Iterator where Iter.Item = Item;
    /// type Item }` — when we resolve the RHS `Item` of the alias's where
    /// clause with `context = Iter`, scope walking should walk Iter → Iterable
    /// and find the sibling `Item` alias.
    ///
    /// This is the empirical evidence that `WhereClausesOf { entity }` needs
    /// no separate `context` parameter: the entity's own scope is sufficient.
    #[test]
    fn resolve_sibling_assoc_type_from_alias_scope() {
        let (mut world, root) = setup();

        let myapp = world
            .children_of(root)
            .iter()
            .find(|&&e| world.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        // protocol Iterable { type Iter; type Item }
        let iterable = world.spawn();
        world.set(iterable, NodeKind::Protocol);
        world.set(iterable, Name("Iterable".into()));
        world.set(iterable, Typed);
        world.set_parent(iterable, myapp);

        let iter_alias = world.spawn();
        world.set(iter_alias, NodeKind::TypeAlias);
        world.set(iter_alias, Name("TargetIterator".into()));
        world.set(iter_alias, Typed);
        world.set_parent(iter_alias, iterable);

        let item_alias = world.spawn();
        world.set(item_alias, NodeKind::TypeAlias);
        world.set(item_alias, Name("Item".into()));
        world.set(item_alias, Typed);
        world.set_parent(item_alias, iterable);

        let ctx = world.query_context();
        // Resolve `Item` from Iter's own scope — scope walk goes Iter →
        // Iterable and must find the sibling Item alias.
        let result = ctx.query(ResolveTypePath {
            segments: vec!["Item".into()],
            context: iter_alias,
            root,
        });
        match result {
            TypeResolution::Found(entity) => {
                assert_eq!(
                    entity, item_alias,
                    "expected sibling Item, got different entity"
                );
            },
            other => panic!("expected Found(Item), got {:?}", other),
        }
    }
}
