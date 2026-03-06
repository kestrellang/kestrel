//! Name resolution via scope chain walk.
//!
//! The core query: given a name and a context entity, walk the scope
//! chain upward to find matching declarations, imports, or wildcard
//! import members.

use kestrel_ast_builder::{Conformances, ConformanceItem, Name, NodeKind, TypeParams};
use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::extensions::ExtensionTargetEntity;
use crate::scope::ScopeFor;
use crate::visibility::VisibleChildrenByName;

// ===== NameResolution =====

/// Result of resolving a name.
#[derive(Clone, Debug)]
pub enum NameResolution {
    /// One or more matching entities (overloaded functions share a name)
    Found(Vec<Entity>),
    /// Multiple conflicting non-function matches from different sources
    Ambiguous(Vec<Entity>),
    /// No match found
    NotFound,
}

// ===== ResolveName =====

/// Query: resolve a simple name by walking the scope chain.
///
/// Checks in order at each scope level:
/// 1. Selective imports
/// 2. Local declarations
/// 3. Wildcard imports (visible members only)
/// 4. Extension type params (if in an extension)
/// 5. Protocol extension associated types
/// 6. Inherited protocol members (if in a protocol)
///
/// If no match at this level, walks up to parent scope.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ResolveName {
    pub name: String,
    pub context: Entity,
    pub root: Entity,
}

impl QueryFn for ResolveName {
    type Output = NameResolution;

    fn execute(&self, ctx: &QueryContext<'_>) -> NameResolution {
        let mut current = self.context;

        loop {
            let scope = ctx.query(ScopeFor {
                entity: current,
                root: self.root,
            });

            // 1. Selective imports (highest priority at this scope)
            if let Some(entities) = scope.selective_imports.get(&self.name) {
                if !entities.is_empty() {
                    return check_ambiguity(ctx, entities.clone());
                }
            }

            // 2. Local declarations
            if let Some(entities) = scope.declarations.get(&self.name) {
                if !entities.is_empty() {
                    return check_ambiguity(ctx, entities.clone());
                }
            }

            // 3. Wildcard imports: check each wildcard source module
            let mut wildcard_matches = Vec::new();
            for &wc_module in &scope.wildcard_imports {
                let visible = ctx.query(VisibleChildrenByName {
                    parent: wc_module,
                    name: self.name.clone(),
                    context: self.context,
                });
                wildcard_matches.extend(visible);
            }
            // Deduplicate (same entity from multiple wildcard sources)
            wildcard_matches.sort_by_key(|e| e.index());
            wildcard_matches.dedup();

            if !wildcard_matches.is_empty() {
                // If all matches are functions, it's overloading. Otherwise
                // multiple non-function matches = ambiguity.
                let all_functions = wildcard_matches.iter().all(|&e| {
                    ctx.get::<NodeKind>(e) == Some(&NodeKind::Function)
                });
                if wildcard_matches.len() > 1 && !all_functions {
                    return NameResolution::Ambiguous(wildcard_matches);
                }
                return NameResolution::Found(wildcard_matches);
            }

            // 4. Extension type params: if current is an extension,
            //    check type params of the extension's target type
            if ctx.get::<NodeKind>(current) == Some(&NodeKind::Extension) {
                if let Some(entity) = resolve_extension_type_param(ctx, current, &self.name, self.root) {
                    return NameResolution::Found(vec![entity]);
                }
            }

            // 5. Protocol extension associated types: if current is an extension
            //    targeting a protocol, check the protocol's associated types
            if ctx.get::<NodeKind>(current) == Some(&NodeKind::Extension) {
                if let Some(entity) = resolve_protocol_extension_assoc(ctx, current, &self.name, self.root) {
                    return NameResolution::Found(vec![entity]);
                }
            }

            // 6. Inherited protocol members: if current is a protocol,
            //    walk conformance hierarchy for matching associated types
            if ctx.get::<NodeKind>(current) == Some(&NodeKind::Protocol) {
                if let Some(entity) = resolve_inherited_protocol_member(ctx, current, &self.name, self.root) {
                    return NameResolution::Found(vec![entity]);
                }
            }

            // Walk up to parent
            match scope.parent {
                Some(parent) => current = parent,
                None => return NameResolution::NotFound,
            }
        }
    }
}

/// Check for ambiguity: multiple non-function matches are ambiguous.
fn check_ambiguity(ctx: &QueryContext<'_>, entities: Vec<Entity>) -> NameResolution {
    if entities.len() > 1 {
        let all_fns = entities
            .iter()
            .all(|&e| ctx.get::<NodeKind>(e) == Some(&NodeKind::Function));
        if !all_fns {
            return NameResolution::Ambiguous(entities);
        }
    }
    NameResolution::Found(entities)
}

/// Check if an extension's target type has type params matching the name.
///
/// For `extend Array[T]`, when looking up `T` inside the extension body,
/// we find Array's type parameter `T`.
fn resolve_extension_type_param(
    ctx: &QueryContext<'_>,
    extension: Entity,
    name: &str,
    root: Entity,
) -> Option<Entity> {
    // Resolve the extension's target to a type entity
    let target_entity = ctx.query(ExtensionTargetEntity {
        extension,
        root,
    })?;

    // Check the target type's type parameters
    let type_params = ctx.get::<TypeParams>(target_entity)?;
    for &tp in &type_params.0 {
        if ctx.get::<Name>(tp).is_some_and(|n| n.0 == name) {
            return Some(tp);
        }
    }
    None
}

/// Check if an extension targets a protocol, and if so, look up the
/// protocol's associated types by name.
fn resolve_protocol_extension_assoc(
    ctx: &QueryContext<'_>,
    extension: Entity,
    name: &str,
    root: Entity,
) -> Option<Entity> {
    let target_entity = ctx.query(ExtensionTargetEntity {
        extension,
        root,
    })?;

    // Only applies if the target is a protocol
    if ctx.get::<NodeKind>(target_entity) != Some(&NodeKind::Protocol) {
        return None;
    }

    // Search the protocol's children for a matching TypeAlias (associated type)
    find_assoc_type(ctx, target_entity, name)
}

/// Walk a protocol's conformance hierarchy to find an associated type by name.
///
/// For `protocol Foo: Bar`, if we're inside Foo and look up an associated
/// type defined in Bar, this finds it.
fn resolve_inherited_protocol_member(
    ctx: &QueryContext<'_>,
    protocol: Entity,
    name: &str,
    root: Entity,
) -> Option<Entity> {
    let conformances = ctx.get::<Conformances>(protocol)?;

    for item in &conformances.0 {
        let ConformanceItem::Positive(ast_type, _) = item else {
            continue;
        };

        // Resolve the conformance target (parent protocol) via type path
        let kestrel_ast::AstType::Named { segments, .. } = ast_type else {
            continue;
        };
        if segments.is_empty() {
            continue;
        }

        // Resolve full path (supports multi-segment like std.core.Equatable)
        let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
        let type_result = ctx.query(crate::resolve_type::ResolveTypePath {
            segments: seg_names,
            context: protocol,
            root,
        });

        let crate::resolve_type::TypeResolution::Found(proto_entity) = type_result else {
            continue;
        };

        if ctx.get::<NodeKind>(proto_entity) != Some(&NodeKind::Protocol) {
            continue;
        }

        // Check this parent protocol's associated types
        if let Some(found) = find_assoc_type(ctx, proto_entity, name) {
            return Some(found);
        }

        // Recursively check inherited protocols
        if let Some(found) =
            resolve_inherited_protocol_member(ctx, proto_entity, name, root)
        {
            return Some(found);
        }
    }
    None
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

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast_builder::{ModulePath, Typed, Vis};
    use kestrel_hecs::World;

    /// Build a small world:
    ///   root > std > core > [Int64(pub), Bool(pub)]
    ///        > MyApp > [Foo, Bar, import std.core]
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

        let bool_t = world.spawn();
        world.set(bool_t, NodeKind::Struct);
        world.set(bool_t, Name("Bool".into()));
        world.set(bool_t, Vis::Public);
        world.set(bool_t, Typed);
        world.set_parent(bool_t, core);

        // MyApp
        let myapp = world.spawn();
        world.set(myapp, NodeKind::Module);
        world.set(myapp, Name("MyApp".into()));
        world.set_parent(myapp, root);

        // Wildcard import
        let imp = world.spawn();
        world.set(imp, NodeKind::Import);
        world.set(imp, ModulePath(vec!["std".into(), "core".into()]));
        world.set_parent(imp, myapp);

        let foo = world.spawn();
        world.set(foo, NodeKind::Struct);
        world.set(foo, Name("Foo".into()));
        world.set(foo, Typed);
        world.set_parent(foo, myapp);

        let bar = world.spawn();
        world.set(bar, NodeKind::Function);
        world.set(bar, Name("bar".into()));
        world.set_parent(bar, myapp);

        (world, root)
    }

    #[test]
    fn resolve_local_declaration() {
        let (world, root) = setup();
        let ctx = world.query_context();

        let myapp = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        let result = ctx.query(ResolveName {
            name: "Foo".into(),
            context: myapp,
            root,
        });

        match result {
            NameResolution::Found(entities) => {
                assert_eq!(entities.len(), 1);
                assert_eq!(ctx.get::<Name>(entities[0]).unwrap().0, "Foo");
            }
            other => panic!("expected Found, got {:?}", other),
        }
    }

    #[test]
    fn resolve_via_wildcard_import() {
        let (world, root) = setup();
        let ctx = world.query_context();

        let myapp = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        // Int64 comes from wildcard import of std.core
        let result = ctx.query(ResolveName {
            name: "Int64".into(),
            context: myapp,
            root,
        });

        match result {
            NameResolution::Found(entities) => {
                assert_eq!(entities.len(), 1);
                assert_eq!(ctx.get::<Name>(entities[0]).unwrap().0, "Int64");
            }
            other => panic!("expected Found, got {:?}", other),
        }
    }

    #[test]
    fn resolve_not_found() {
        let (world, root) = setup();
        let ctx = world.query_context();

        let myapp = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        let result = ctx.query(ResolveName {
            name: "Nonexistent".into(),
            context: myapp,
            root,
        });

        assert!(matches!(result, NameResolution::NotFound));
    }

    #[test]
    fn resolve_local_shadows_import() {
        let (mut world, root) = setup();

        // Add a local "Int64" to MyApp (shadows the import)
        let myapp = world
            .children_of(root)
            .iter()
            .find(|&&e| world.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        let local_int = world.spawn();
        world.set(local_int, NodeKind::Struct);
        world.set(local_int, Name("Int64".into()));
        world.set(local_int, Typed);
        world.set_parent(local_int, myapp);

        let ctx = world.query_context();

        let result = ctx.query(ResolveName {
            name: "Int64".into(),
            context: myapp,
            root,
        });

        match result {
            NameResolution::Found(entities) => {
                // Should find the local one, not the import
                assert_eq!(entities.len(), 1);
                assert_eq!(entities[0], local_int);
            }
            other => panic!("expected Found, got {:?}", other),
        }
    }

    #[test]
    fn resolve_via_auto_import() {
        // Non-std module should auto-import std modules
        let (world, root) = setup();
        let ctx = world.query_context();

        let myapp = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        // Bool should be findable via auto-import
        let result = ctx.query(ResolveName {
            name: "Bool".into(),
            context: myapp,
            root,
        });

        assert!(matches!(result, NameResolution::Found(_)));
    }
}
