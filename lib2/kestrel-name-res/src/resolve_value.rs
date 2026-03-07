//! Value path resolution.
//!
//! Resolves value names (variables, functions, enum cases, etc.) to
//! entities. Used by HIR lowering for expression paths.

use kestrel_ast_builder::{Callable, Conformances, ConformanceItem, Gettable, NodeKind, Static, Typed, WhereClause, Name};
use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::extensions::ExtensionsFor;
use crate::resolve_name::{NameResolution, ResolveName};
use crate::resolve_type::{ResolveTypePath, TypeResolution};
use crate::visibility::VisibleChildrenByName;

// ===== ValueResolution =====

/// Result of resolving a value path.
#[derive(Clone, Debug, Hash)]
pub enum ValueResolution {
    /// Single definition
    Def(Entity),
    /// Multiple overloaded functions with the same name
    Overloaded(Vec<Entity>),
    /// Multiple conflicting non-function matches
    Ambiguous(Vec<Entity>),
    /// Type parameter (e.g. `T` used as a value for static access)
    TypeParameter(Entity),
    /// Associated type (e.g. in protocol context)
    AssociatedType { entity: Entity, container: Option<Entity> },
    /// Enum case used as intermediate value (e.g. `MyEnum.caseA.method()`)
    EnumCaseValue { entity: Entity, resolved_index: usize },
    /// Field/getter used as intermediate value (e.g. `obj.field.method()`)
    FieldValue { entity: Entity, resolved_index: usize },
    /// Static member accessed through associated type (e.g., `Item.zero` where `Item: Addable`)
    /// Preserves the associated type context for Self-substitution in type inference.
    AssociatedTypeStaticMember { entity: Entity, assoc_type: Entity },
    /// Not found
    NotFound(String),
}

// ===== ResolveValuePath =====

/// Query: resolve a value path to an entity.
///
/// Handles:
/// - Single-segment: function, enum case, field, etc. via ResolveName
/// - Multi-segment: first segment resolved, then walk children
/// - Type alias: resolves through to underlying type
/// - Extension static methods: if no direct match, search extensions
/// - Function overloads: multiple functions → Overloaded
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ResolveValuePath {
    pub segments: Vec<String>,
    pub context: Entity,
    pub root: Entity,
}

impl QueryFn for ResolveValuePath {
    type Output = ValueResolution;

    fn execute(&self, ctx: &QueryContext<'_>) -> ValueResolution {
        if self.segments.is_empty() {
            return ValueResolution::NotFound("<empty>".into());
        }

        // Single segment: simple name lookup
        if self.segments.len() == 1 {
            return resolve_single_segment(ctx, &self.segments[0], self.context, self.root);
        }

        // Multi-segment: resolve first segment, then walk
        resolve_multi_segment(ctx, &self.segments, self.context, self.root)
    }
}

/// Resolve a single-segment value name.
fn resolve_single_segment(
    ctx: &QueryContext<'_>,
    name: &str,
    context: Entity,
    root: Entity,
) -> ValueResolution {
    let result = ctx.query(ResolveName {
        name: name.to_string(),
        context,
        root,
    });

    match result {
        NameResolution::Found(entities) => {
            // Check for special single-entity kinds
            if entities.len() == 1 {
                let e = entities[0];
                match ctx.get::<NodeKind>(e) {
                    Some(&NodeKind::TypeParameter) => {
                        return ValueResolution::TypeParameter(e);
                    }
                    Some(&NodeKind::TypeAlias) => {
                        // If inside a protocol, this is an associated type
                        if let Some(parent) = ctx.parent_of(e) {
                            if ctx.get::<NodeKind>(parent) == Some(&NodeKind::Protocol) {
                                return ValueResolution::AssociatedType {
                                    entity: e,
                                    container: None,
                                };
                            }
                        }
                    }
                    _ => {}
                }
            }
            classify_value_results(ctx, entities)
        }
        NameResolution::Ambiguous(entities) => ValueResolution::Ambiguous(entities),
        NameResolution::NotFound => ValueResolution::NotFound(name.to_string()),
    }
}

/// Resolve a multi-segment value path (e.g. `MyEnum.case` or `Module.func`).
fn resolve_multi_segment(
    ctx: &QueryContext<'_>,
    segments: &[String],
    context: Entity,
    root: Entity,
) -> ValueResolution {
    // Resolve first segment
    let first = &segments[0];
    let first_result = ctx.query(ResolveName {
        name: first.clone(),
        context,
        root,
    });

    let first_entity = match first_result {
        NameResolution::Found(entities) => {
            if entities.len() == 1 {
                entities[0]
            } else {
                // Multiple matches: allow if all functions, otherwise
                // try to disambiguate to a single type for qualified access
                let all_fns = entities
                    .iter()
                    .all(|&e| ctx.get::<NodeKind>(e) == Some(&NodeKind::Function));
                if all_fns {
                    // Function overloads can't be used as multi-segment base
                    return ValueResolution::Ambiguous(entities);
                }
                // Try to find a unique type
                let types: Vec<Entity> = entities
                    .iter()
                    .filter(|&&e| ctx.has::<Typed>(e))
                    .copied()
                    .collect();
                if types.len() == 1 {
                    types[0]
                } else {
                    return ValueResolution::Ambiguous(entities);
                }
            }
        }
        NameResolution::Ambiguous(entities) => {
            // Try to find a unique type among ambiguous results
            let types: Vec<Entity> = entities
                .iter()
                .filter(|&&e| ctx.has::<Typed>(e))
                .copied()
                .collect();
            if types.len() == 1 {
                types[0]
            } else {
                return ValueResolution::Ambiguous(entities);
            }
        }
        NameResolution::NotFound => {
            return ValueResolution::NotFound(first.clone());
        }
    };

    // Walk remaining segments
    let mut current = first_entity;
    for (i, segment) in segments[1..].iter().enumerate() {
        let is_last = i == segments.len() - 2;

        // Check if current is a type alias → resolve through
        if ctx.get::<NodeKind>(current) == Some(&NodeKind::TypeAlias) {
            if let Some(resolved) = resolve_type_alias_target(ctx, current, context, root) {
                current = resolved;
            }
        }

        // Try direct children first
        let children = ctx.query(VisibleChildrenByName {
            parent: current,
            name: segment.clone(),
            context,
        });

        if !children.is_empty() {
            if is_last {
                return classify_value_results(ctx, children);
            }
            // For intermediate segments, prefer types
            current = children
                .iter()
                .find(|&&e| ctx.has::<Typed>(e))
                .copied()
                .unwrap_or(children[0]);
            continue;
        }

        // No direct children — try extension static methods (only for last segment)
        if is_last && ctx.has::<Typed>(current) {
            let extensions = ctx.query(ExtensionsFor {
                target: current,
                root,
            });
            for &ext in &extensions {
                let ext_children = ctx.query(VisibleChildrenByName {
                    parent: ext,
                    name: segment.clone(),
                    context,
                });
                // Filter to static methods only
                let static_methods: Vec<Entity> = ext_children
                    .into_iter()
                    .filter(|&e| {
                        ctx.get::<NodeKind>(e) == Some(&NodeKind::Function)
                            && (ctx.has::<Static>(e)
                                || ctx
                                    .get::<Callable>(e)
                                    .map_or(false, |c| c.receiver.is_none()))
                    })
                    .collect();
                if !static_methods.is_empty() {
                    return classify_value_results(ctx, static_methods);
                }
            }
        }

        // For associated types (abstract TypeAlias), search protocol bounds
        // for static members (e.g. Item.zero where Item: Addable).
        // Return AssociatedTypeStaticMember to preserve the associated type
        // context for Self-substitution in type inference.
        if ctx.get::<NodeKind>(current) == Some(&NodeKind::TypeAlias) {
            if let Some(found) = resolve_assoc_type_static_member(ctx, current, segment, context, root) {
                if is_last {
                    return ValueResolution::AssociatedTypeStaticMember {
                        entity: found,
                        assoc_type: current,
                    };
                }
                current = found;
                continue;
            }
        }

        // Check for enum case or field/getter used as intermediate value
        // (e.g. MyEnum.caseA where caseA has no children to walk into)
        let segment_index = i + 1; // index in original segments
        match ctx.get::<NodeKind>(current) {
            Some(&NodeKind::EnumCase) => {
                return ValueResolution::EnumCaseValue {
                    entity: current,
                    resolved_index: segment_index - 1,
                };
            }
            Some(&NodeKind::Field) if ctx.has::<Gettable>(current) => {
                return ValueResolution::FieldValue {
                    entity: current,
                    resolved_index: segment_index - 1,
                };
            }
            _ => {}
        }

        return ValueResolution::NotFound(segment.clone());
    }

    // Reached the end with a non-terminal entity
    ValueResolution::Def(current)
}

/// Classify resolved entities as Def or Overloaded.
fn classify_value_results(ctx: &QueryContext<'_>, entities: Vec<Entity>) -> ValueResolution {
    if entities.len() == 1 {
        return ValueResolution::Def(entities[0]);
    }

    // Multiple results: if all functions, it's overloading
    let all_functions = entities
        .iter()
        .all(|&e| ctx.get::<NodeKind>(e) == Some(&NodeKind::Function));

    if all_functions {
        ValueResolution::Overloaded(entities)
    } else {
        // Mixed results — ambiguous
        ValueResolution::Ambiguous(entities)
    }
}

/// Try to resolve a type alias to its underlying type entity.
fn resolve_type_alias_target(
    ctx: &QueryContext<'_>,
    alias: Entity,
    context: Entity,
    root: Entity,
) -> Option<Entity> {
    let type_ann = ctx.get::<kestrel_ast_builder::TypeAnnotation>(alias)?;
    let kestrel_ast::AstType::Named { segments, .. } = &type_ann.0 else {
        return None;
    };
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();

    let result = ctx.query(ResolveTypePath {
        segments: seg_names,
        context,
        root,
    });

    match result {
        TypeResolution::Found(entity) => Some(entity),
        _ => None,
    }
}

/// Search protocol bounds on an associated type for a static member.
///
/// For `Item.zero` where `Item: Addable`, finds the `zero` static member
/// inside the `Addable` protocol. Checks both direct conformances on the
/// TypeAlias and where-clause bounds in ancestor entities.
fn resolve_assoc_type_static_member(
    ctx: &QueryContext<'_>,
    assoc_type: Entity,
    member_name: &str,
    context: Entity,
    root: Entity,
) -> Option<Entity> {
    let assoc_name = ctx.get::<Name>(assoc_type)?;

    // Collect all protocol entities that bound this associated type
    let mut bound_protocols = Vec::new();

    // 1. Direct conformances on the TypeAlias (e.g. `type Item: Addable`)
    if let Some(conformances) = ctx.get::<Conformances>(assoc_type) {
        for item in &conformances.0 {
            let ConformanceItem::Positive(ast_type, _) = item else { continue };
            if let Some(proto) = resolve_protocol_from_ast(ctx, ast_type, context, root) {
                bound_protocols.push(proto);
            }
        }
    }

    // 2. Where-clause bounds in ancestor chain (e.g. `where Item: Addable`)
    let mut ancestor = Some(context);
    while let Some(anc) = ancestor {
        if let Some(where_clause) = ctx.get::<WhereClause>(anc) {
            for constraint in &where_clause.0 {
                let kestrel_ast_builder::WhereConstraint::Bound {
                    subject, protocols, ..
                } = constraint
                else {
                    continue;
                };
                let kestrel_ast::AstType::Named { segments, .. } = subject else {
                    continue;
                };
                // Match single-segment "Item" or two-segment "Self.Item"
                let matches = match segments.len() {
                    1 => segments[0].name == assoc_name.0,
                    2 => segments[0].name == "Self" && segments[1].name == assoc_name.0,
                    _ => false,
                };
                if matches {
                    for proto_ty in protocols {
                        if let Some(proto) = resolve_protocol_from_ast(ctx, proto_ty, anc, root) {
                            bound_protocols.push(proto);
                        }
                    }
                }
            }
        }
        ancestor = ctx.parent_of(anc);
    }

    // Search each protocol for a static member with the requested name
    for proto in &bound_protocols {
        let children = ctx.query(VisibleChildrenByName {
            parent: *proto,
            name: member_name.to_string(),
            context,
        });
        if !children.is_empty() {
            return Some(children[0]);
        }

        // Also check extensions of the protocol
        let extensions = ctx.query(ExtensionsFor {
            target: *proto,
            root,
        });
        for &ext in &extensions {
            let ext_children = ctx.query(VisibleChildrenByName {
                parent: ext,
                name: member_name.to_string(),
                context,
            });
            if !ext_children.is_empty() {
                return Some(ext_children[0]);
            }
        }
    }

    None
}

/// Resolve an AstType to a protocol entity.
fn resolve_protocol_from_ast(
    ctx: &QueryContext<'_>,
    ast_type: &kestrel_ast::AstType,
    scope: Entity,
    root: Entity,
) -> Option<Entity> {
    let kestrel_ast::AstType::Named { segments, .. } = ast_type else {
        return None;
    };
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    let result = ctx.query(ResolveTypePath {
        segments: seg_names,
        context: scope,
        root,
    });
    match result {
        TypeResolution::Found(entity) if ctx.get::<NodeKind>(entity) == Some(&NodeKind::Protocol) => {
            Some(entity)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast_builder::{Callable, ModulePath, Name, Vis};
    use kestrel_hecs::World;

    /// Build:
    ///   root > std > core > [Int64(pub, Typed)]
    ///        > MyApp > [Foo(Struct, Typed), bar(Function), baz(Function)]
    ///                > Foo > [case1(EnumCase)]  (simulated as children)
    fn setup() -> (World, Entity) {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        // std.core
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

        // MyApp
        let myapp = world.spawn();
        world.set(myapp, NodeKind::Module);
        world.set(myapp, Name("MyApp".into()));
        world.set_parent(myapp, root);

        // Import std.core
        let imp = world.spawn();
        world.set(imp, NodeKind::Import);
        world.set(imp, ModulePath(vec!["std".into(), "core".into()]));
        world.set_parent(imp, myapp);

        // Enum with cases
        let my_enum = world.spawn();
        world.set(my_enum, NodeKind::Enum);
        world.set(my_enum, Name("MyEnum".into()));
        world.set(my_enum, Typed);
        world.set(my_enum, Vis::Public);
        world.set_parent(my_enum, myapp);

        let case_a = world.spawn();
        world.set(case_a, NodeKind::EnumCase);
        world.set(case_a, Name("caseA".into()));
        world.set_parent(case_a, my_enum);

        let case_b = world.spawn();
        world.set(case_b, NodeKind::EnumCase);
        world.set(case_b, Name("caseB".into()));
        world.set_parent(case_b, my_enum);

        // Two overloaded functions
        let bar1 = world.spawn();
        world.set(bar1, NodeKind::Function);
        world.set(bar1, Name("bar".into()));
        world.set(
            bar1,
            Callable {
                params: vec![],
                receiver: None,
            },
        );
        world.set_parent(bar1, myapp);

        let bar2 = world.spawn();
        world.set(bar2, NodeKind::Function);
        world.set(bar2, Name("bar".into()));
        world.set(
            bar2,
            Callable {
                params: vec![kestrel_ast_builder::AstParam {
                    label: None,
                    name: "x".into(),
                    ty: None,
                    has_default: false,
                }],
                receiver: None,
            },
        );
        world.set_parent(bar2, myapp);

        (world, root)
    }

    #[test]
    fn resolve_single_function() {
        let (world, root) = setup();
        let ctx = world.query_context();

        let myapp = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        // "bar" is overloaded
        let result = ctx.query(ResolveValuePath {
            segments: vec!["bar".into()],
            context: myapp,
            root,
        });
        assert!(matches!(result, ValueResolution::Overloaded(_)));
    }

    #[test]
    fn resolve_enum_case() {
        let (world, root) = setup();
        let ctx = world.query_context();

        let myapp = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        // MyEnum.caseA
        let result = ctx.query(ResolveValuePath {
            segments: vec!["MyEnum".into(), "caseA".into()],
            context: myapp,
            root,
        });
        match result {
            ValueResolution::Def(entity) => {
                assert_eq!(ctx.get::<Name>(entity).unwrap().0, "caseA");
                assert_eq!(
                    ctx.get::<NodeKind>(entity),
                    Some(&NodeKind::EnumCase)
                );
            }
            other => panic!("expected Def, got {:?}", other),
        }
    }

    #[test]
    fn resolve_not_found() {
        let (world, root) = setup();
        let ctx = world.query_context();

        let result = ctx.query(ResolveValuePath {
            segments: vec!["nonexistent".into()],
            context: root,
            root,
        });
        assert!(matches!(result, ValueResolution::NotFound(_)));
    }

    #[test]
    fn resolve_multi_segment_not_found() {
        let (world, root) = setup();
        let ctx = world.query_context();

        let myapp = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        let result = ctx.query(ResolveValuePath {
            segments: vec!["MyEnum".into(), "nonexistent".into()],
            context: myapp,
            root,
        });
        assert!(matches!(result, ValueResolution::NotFound(_)));
    }
}
