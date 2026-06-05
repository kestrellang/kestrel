//! Extension resolution queries.
//!
//! Finds extensions for a given type entity and resolves extension
//! target types from AstType to entities.

use kestrel_ast_builder::{ExtensionTarget, Name, NodeKind};
use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::resolve_type::{ResolveTypePath, TypeResolution};

// ===== ResolvedExtensionTarget =====

/// Component: resolved extension target entity.
///
/// Can be set during a mutation-phase pass after AST building, or
/// computed lazily by ExtensionTargetEntity query.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResolvedExtensionTarget(pub Entity);

// ===== ExtensionTargetEntity =====

/// Query: resolve an extension's target AstType to a type entity.
///
/// Reads the ExtensionTarget(AstType) component and resolves it
/// via ResolveTypePath. No cycle risk because type resolution never
/// looks through extensions (extensions lack the Typed marker).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExtensionTargetEntity {
    pub extension: Entity,
    pub root: Entity,
}

impl QueryFn for ExtensionTargetEntity {
    type Output = Option<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Entity> {
        // Check for pre-resolved target first
        if let Some(resolved) = ctx.get::<ResolvedExtensionTarget>(self.extension) {
            return Some(resolved.0);
        }

        // Get the unresolved AstType from the ExtensionTarget component
        let target = ctx.get::<ExtensionTarget>(self.extension)?;
        let ast_type = &target.0;

        // Structural singletons `()` and `!` resolve to synthetic `lang` entities
        // (named "()" / "!") so they can be extension targets.
        match ast_type {
            kestrel_ast::AstType::Unit(..) => return resolve_lang_child(ctx, self.root, "()"),
            kestrel_ast::AstType::Never(..) => return resolve_lang_child(ctx, self.root, "!"),
            _ => {},
        }

        // Extract path segments from the AstType
        let kestrel_ast::AstType::Named { segments, .. } = ast_type else {
            return None;
        };

        let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();

        // Resolve using the extension's parent as context (the module it's in)
        let context = ctx.parent_of(self.extension).unwrap_or(self.root);

        let result = ctx.query(ResolveTypePath {
            segments: seg_names,
            context,
            root: self.root,
        });

        match result {
            TypeResolution::Found(entity) => Some(entity),
            _ => None,
        }
    }
}

/// Resolve a synthetic child of the `lang` module by its (possibly
/// non-identifier) name — used for the structural singletons `()` / `!`.
pub fn resolve_lang_child(ctx: &QueryContext<'_>, root: Entity, name: &str) -> Option<Entity> {
    let lang = ctx.children_of(root).iter().copied().find(|&c| {
        ctx.get::<NodeKind>(c) == Some(&NodeKind::Module)
            && ctx.get::<Name>(c).map(|n| n.0 == "lang").unwrap_or(false)
    })?;
    ctx.children_of(lang)
        .iter()
        .copied()
        .find(|&c| ctx.get::<Name>(c).map(|n| n.0 == name).unwrap_or(false))
}

// ===== ExtensionsFor =====

/// Query: find all extensions targeting a given type entity.
///
/// Walks the entire module hierarchy from root, collecting all
/// Extension entities whose resolved target matches the given type.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExtensionsFor {
    pub target: Entity,
    pub root: Entity,
}

impl QueryFn for ExtensionsFor {
    type Output = Vec<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<Entity> {
        let mut extensions = Vec::new();
        collect_extensions(ctx, self.root, self.target, self.root, &mut extensions);
        extensions
    }
}

/// Recursively walk the hierarchy to find Extension entities targeting `target`.
fn collect_extensions(
    ctx: &QueryContext<'_>,
    current: Entity,
    target: Entity,
    root: Entity,
    out: &mut Vec<Entity>,
) {
    for &child in ctx.children_of(current) {
        let kind = ctx.get::<NodeKind>(child);
        match kind {
            Some(&NodeKind::Extension) => {
                // Resolve extension target and check if it matches
                let resolved = ctx.query(ExtensionTargetEntity {
                    extension: child,
                    root,
                });
                if resolved == Some(target) {
                    out.push(child);
                }
            },
            Some(&NodeKind::Module) => {
                // Recurse into modules
                collect_extensions(ctx, child, target, root, out);
            },
            _ => {},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast::{AstType, PathSegment};
    use kestrel_ast_builder::ExtensionTarget;
    use kestrel_ast_builder::{Name, Typed, Vis};
    use kestrel_hecs::World;
    use kestrel_span::Span;

    fn test_span() -> Span {
        Span::synthetic(0)
    }

    /// Build: root > std > core > [Int64(pub, Typed)]
    ///                           > [extend Int64 { ... }]
    fn setup() -> (World, Entity, Entity) {
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

        // Extension targeting Int64
        let ext = world.spawn();
        world.set(ext, NodeKind::Extension);
        world.set(
            ext,
            ExtensionTarget(AstType::Named {
                segments: vec![PathSegment {
                    name: "Int64".into(),
                    type_args: vec![],
                    span: test_span(),
                }],
                span: test_span(),
            }),
        );
        world.set_parent(ext, core);

        (world, root, int64)
    }

    #[test]
    fn extension_target_resolves() {
        let (world, root, int64) = setup();
        let ctx = world.query_context();

        // Find the extension entity
        let std = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "std"))
            .copied()
            .unwrap();
        let core = ctx
            .children_of(std)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "core"))
            .copied()
            .unwrap();
        let ext = ctx
            .children_of(core)
            .iter()
            .find(|&&e| ctx.get::<NodeKind>(e) == Some(&NodeKind::Extension))
            .copied()
            .unwrap();

        let resolved = ctx.query(ExtensionTargetEntity {
            extension: ext,
            root,
        });
        assert_eq!(resolved, Some(int64));
    }

    #[test]
    fn extensions_for_finds_matching() {
        let (world, root, int64) = setup();
        let ctx = world.query_context();

        let exts = ctx.query(ExtensionsFor {
            target: int64,
            root,
        });
        assert_eq!(exts.len(), 1);
    }

    #[test]
    fn extensions_for_no_match() {
        let (mut world, root, _) = setup();
        // Spawn a random struct — no extensions target it
        let other = world.spawn();
        world.set(other, NodeKind::Struct);
        world.set(other, Name("Other".into()));
        world.set(other, Typed);
        world.set_parent(other, root);

        let ctx = world.query_context();
        let exts = ctx.query(ExtensionsFor {
            target: other,
            root,
        });
        assert!(exts.is_empty());
    }
}
