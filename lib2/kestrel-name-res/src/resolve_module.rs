//! Module path resolution queries.
//!
//! Resolves dotted module paths (e.g. `std.collections`) to their
//! corresponding module entities in the ECS hierarchy.

use kestrel_ast_builder::{Name, NodeKind};
use kestrel_hecs::{Entity, QueryContext, QueryFn};

// ===== ResolveModulePath =====

/// Query: resolve a dotted module path to a module entity.
///
/// Walks the module hierarchy from root, matching each path segment
/// against module children by name. Returns None if any segment
/// fails to match.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ResolveModulePath {
    pub path: Vec<String>,
    pub root: Entity,
}

impl QueryFn for ResolveModulePath {
    type Output = Option<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Entity> {
        if self.path.is_empty() {
            return None;
        }

        // First segment: search the entire module tree from root
        let first = find_module_by_name(ctx, self.root, &self.path[0])?;

        // Subsequent segments: walk children of current module
        let mut current = first;
        for segment in &self.path[1..] {
            current = find_child_module(ctx, current, segment)?;
        }

        Some(current)
    }
}

/// Search the module tree from `root` for a module with the given name.
/// Does a BFS through all module children.
fn find_module_by_name(ctx: &QueryContext<'_>, root: Entity, name: &str) -> Option<Entity> {
    let mut queue = vec![root];
    while let Some(current) = queue.pop() {
        for &child in ctx.children_of(current) {
            if ctx.get::<NodeKind>(child) == Some(&NodeKind::Module) {
                if ctx.get::<Name>(child).is_some_and(|n| n.0 == name) {
                    return Some(child);
                }
                // Continue searching deeper
                queue.push(child);
            }
        }
    }
    None
}

/// Find a direct child module with the given name.
fn find_child_module(ctx: &QueryContext<'_>, parent: Entity, name: &str) -> Option<Entity> {
    ctx.children_of(parent)
        .iter()
        .find(|&&child| {
            ctx.get::<NodeKind>(child) == Some(&NodeKind::Module)
                && ctx.get::<Name>(child).is_some_and(|n| n.0 == name)
        })
        .copied()
}

// ===== StdModules =====

/// Query: collect all leaf submodules of the `std` module.
///
/// A leaf module is one with no child modules. These are used for
/// auto-importing stdlib declarations into user code.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StdModules {
    pub root: Entity,
}

impl QueryFn for StdModules {
    type Output = Vec<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<Entity> {
        // Find the "std" module under root
        let Some(std_mod) = find_child_module(ctx, self.root, "std") else {
            return Vec::new();
        };

        // Collect all leaf submodules
        let mut leaves = Vec::new();
        collect_leaf_modules(ctx, std_mod, &mut leaves);
        leaves
    }
}

/// Recursively collect leaf modules (modules with no child modules).
fn collect_leaf_modules(ctx: &QueryContext<'_>, module: Entity, out: &mut Vec<Entity>) {
    let child_modules: Vec<Entity> = ctx
        .children_of(module)
        .iter()
        .filter(|&&child| ctx.get::<NodeKind>(child) == Some(&NodeKind::Module))
        .copied()
        .collect();

    if child_modules.is_empty() {
        // This is a leaf module
        out.push(module);
    } else {
        // Recurse into child modules
        for child in child_modules {
            collect_leaf_modules(ctx, child, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_hecs::World;

    /// Build: root > std > [core, collections > [array, dictionary]]
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

        let collections = world.spawn();
        world.set(collections, NodeKind::Module);
        world.set(collections, Name("collections".into()));
        world.set_parent(collections, std);

        let array = world.spawn();
        world.set(array, NodeKind::Module);
        world.set(array, Name("array".into()));
        world.set_parent(array, collections);

        let dict = world.spawn();
        world.set(dict, NodeKind::Module);
        world.set(dict, Name("dictionary".into()));
        world.set_parent(dict, collections);

        (world, root)
    }

    #[test]
    fn resolve_single_segment() {
        let (world, root) = setup();
        let ctx = world.query_context();
        let result = ctx.query(ResolveModulePath {
            path: vec!["std".into()],
            root,
        });
        assert!(result.is_some());
        let std = result.unwrap();
        assert_eq!(ctx.get::<Name>(std).unwrap().0, "std");
    }

    #[test]
    fn resolve_multi_segment() {
        let (world, root) = setup();
        let ctx = world.query_context();
        let result = ctx.query(ResolveModulePath {
            path: vec!["std".into(), "collections".into(), "array".into()],
            root,
        });
        assert!(result.is_some());
        assert_eq!(ctx.get::<Name>(result.unwrap()).unwrap().0, "array");
    }

    #[test]
    fn resolve_missing_module() {
        let (world, root) = setup();
        let ctx = world.query_context();
        let result = ctx.query(ResolveModulePath {
            path: vec!["std".into(), "nonexistent".into()],
            root,
        });
        assert!(result.is_none());
    }

    #[test]
    fn resolve_empty_path() {
        let (world, root) = setup();
        let ctx = world.query_context();
        let result = ctx.query(ResolveModulePath {
            path: vec![],
            root,
        });
        assert!(result.is_none());
    }

    #[test]
    fn std_modules_collects_leaves() {
        let (world, root) = setup();
        let ctx = world.query_context();
        let modules = ctx.query(StdModules { root });

        // Leaves are: core, array, dictionary (not std, not collections)
        assert_eq!(modules.len(), 3);

        let names: Vec<String> = modules
            .iter()
            .map(|&e| ctx.get::<Name>(e).unwrap().0.clone())
            .collect();
        assert!(names.contains(&"core".to_string()));
        assert!(names.contains(&"array".to_string()));
        assert!(names.contains(&"dictionary".to_string()));
    }

    #[test]
    fn std_modules_empty_when_no_std() {
        let mut world = World::new();
        world.begin_revision();
        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        let ctx = world.query_context();
        let modules = ctx.query(StdModules { root });
        assert!(modules.is_empty());
    }
}
