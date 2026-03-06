//! Scope construction for name resolution.
//!
//! Builds a Scope for each declaration entity, containing its local
//! declarations, selective imports, and wildcard import sources.

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_ast_builder::{ImportAlias, ImportItems, ModulePath, Name, NodeKind};
use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::helpers::is_in_std_module;
use crate::resolve_module::{ResolveModulePath, StdModules};

// ===== Scope =====

/// Resolved scope for a declaration entity.
///
/// Contains all names directly available at this scope level:
/// local declarations, selective imports, and wildcard import sources.
#[derive(Clone, Debug)]
pub struct Scope {
    /// The entity this scope belongs to
    pub entity: Entity,
    /// Selective imports: name -> [target entities]
    /// From `import A.B.(Foo)` and `import A.B as X`
    pub selective_imports: HashMap<String, Vec<Entity>>,
    /// Local declarations: name -> [child entities]
    pub declarations: HashMap<String, Vec<Entity>>,
    /// Wildcard import source modules (checked during name lookup)
    pub wildcard_imports: Vec<Entity>,
    /// Parent entity for scope chain walkup
    pub parent: Option<Entity>,
}

// ===== ScopeFor =====

/// Query: build the scope for a declaration entity.
///
/// Processes children to find local declarations and imports,
/// resolves import module paths, and adds auto-imports from std
/// for non-stdlib entities.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ScopeFor {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for ScopeFor {
    type Output = Arc<Scope>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Arc<Scope> {
        let mut selective_imports: HashMap<String, Vec<Entity>> = HashMap::new();
        let mut declarations: HashMap<String, Vec<Entity>> = HashMap::new();
        let mut wildcard_imports: Vec<Entity> = Vec::new();

        // Process children
        for &child in ctx.children_of(self.entity) {
            let kind = ctx.get::<NodeKind>(child);

            if kind == Some(&NodeKind::Import) {
                // Process import declaration
                process_import(
                    ctx,
                    child,
                    self.root,
                    &mut selective_imports,
                    &mut wildcard_imports,
                );
            } else if let Some(name) = ctx.get::<Name>(child) {
                // Non-import child with a name → local declaration
                declarations
                    .entry(name.0.clone())
                    .or_default()
                    .push(child);
            }
        }

        // Auto-imports: if not in std.*, add all std leaf modules as wildcards
        if !is_in_std_module(ctx, self.entity) {
            let std_modules = ctx.query(StdModules { root: self.root });
            wildcard_imports.extend(std_modules);
        }

        let parent = ctx.parent_of(self.entity);

        Arc::new(Scope {
            entity: self.entity,
            selective_imports,
            declarations,
            wildcard_imports,
            parent,
        })
    }
}

/// Process a single import entity, adding to selective or wildcard imports.
fn process_import(
    ctx: &QueryContext<'_>,
    import: Entity,
    root: Entity,
    selective: &mut HashMap<String, Vec<Entity>>,
    wildcards: &mut Vec<Entity>,
) {
    // Get the module path
    let Some(module_path) = ctx.get::<ModulePath>(import) else {
        return;
    };

    // Resolve the module path to an entity
    let resolved = ctx.query(ResolveModulePath {
        path: module_path.0.clone(),
        root,
    });
    let Some(module_entity) = resolved else {
        return;
    };

    // Check what kind of import this is
    if let Some(items) = ctx.get::<ImportItems>(import) {
        // Selective import: `import A.B.(Foo, Bar as Baz)`
        for item in &items.0 {
            // Find the item in the module's children
            let matches: Vec<Entity> = ctx
                .children_of(module_entity)
                .iter()
                .filter(|&&child| {
                    ctx.get::<Name>(child)
                        .is_some_and(|n| n.0 == item.name)
                })
                .copied()
                .collect();

            // Use alias if provided, otherwise original name
            let import_name = item.alias.as_ref().unwrap_or(&item.name).clone();
            selective.entry(import_name).or_default().extend(matches);
        }
    } else if let Some(alias) = ctx.get::<ImportAlias>(import) {
        // Module alias: `import A.B as X`
        selective
            .entry(alias.0.clone())
            .or_default()
            .push(module_entity);
    } else {
        // Wildcard import: `import A.B`
        wildcards.push(module_entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast_builder::{ImportItem, Typed, Vis};
    use kestrel_hecs::World;

    /// Build: root > std > core > [Int64(pub)]
    ///              > MyApp > [import std.core, Foo]
    fn setup() -> (World, Entity) {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        // std.core with Int64
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

        // MyApp module with a wildcard import and a local decl
        let myapp = world.spawn();
        world.set(myapp, NodeKind::Module);
        world.set(myapp, Name("MyApp".into()));
        world.set_parent(myapp, root);

        // Wildcard import of std.core
        let imp = world.spawn();
        world.set(imp, NodeKind::Import);
        world.set(imp, ModulePath(vec!["std".into(), "core".into()]));
        world.set_parent(imp, myapp);

        // Local struct Foo
        let foo = world.spawn();
        world.set(foo, NodeKind::Struct);
        world.set(foo, Name("Foo".into()));
        world.set(foo, Typed);
        world.set_parent(foo, myapp);

        (world, root)
    }

    #[test]
    fn scope_has_local_declarations() {
        let (world, root) = setup();
        let ctx = world.query_context();

        // Find MyApp module
        let myapp = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        let scope = ctx.query(ScopeFor {
            entity: myapp,
            root,
        });

        assert!(scope.declarations.contains_key("Foo"));
        assert_eq!(scope.declarations["Foo"].len(), 1);
    }

    #[test]
    fn scope_has_wildcard_import() {
        let (world, root) = setup();
        let ctx = world.query_context();

        let myapp = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        let scope = ctx.query(ScopeFor {
            entity: myapp,
            root,
        });

        // Has the explicit wildcard import AND auto-imported std modules
        // The explicit import of std.core + auto-import of std.core (leaf) = core appears
        assert!(!scope.wildcard_imports.is_empty());
    }

    #[test]
    fn scope_has_selective_import() {
        let (mut world, root) = setup();

        // Add a selective import to MyApp: import std.core.(Int64)
        let myapp = world
            .children_of(root)
            .iter()
            .find(|&&e| world.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        let sel_import = world.spawn();
        world.set(sel_import, NodeKind::Import);
        world.set(
            sel_import,
            ModulePath(vec!["std".into(), "core".into()]),
        );
        world.set(
            sel_import,
            ImportItems(vec![ImportItem {
                name: "Int64".into(),
                alias: None,
            }]),
        );
        world.set_parent(sel_import, myapp);

        let ctx = world.query_context();
        let scope = ctx.query(ScopeFor {
            entity: myapp,
            root,
        });

        assert!(scope.selective_imports.contains_key("Int64"));
        assert_eq!(scope.selective_imports["Int64"].len(), 1);
    }

    #[test]
    fn scope_has_parent() {
        let (world, root) = setup();
        let ctx = world.query_context();

        let myapp = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "MyApp"))
            .copied()
            .unwrap();

        let scope = ctx.query(ScopeFor {
            entity: myapp,
            root,
        });
        assert_eq!(scope.parent, Some(root));
    }

    #[test]
    fn std_module_no_auto_imports() {
        let (world, root) = setup();
        let ctx = world.query_context();

        // Find std.core
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

        let scope = ctx.query(ScopeFor {
            entity: core,
            root,
        });

        // std.core should NOT have auto-imported wildcard modules
        // (it only has its own local declarations)
        // Wildcard imports should be empty since it's in std
        assert!(scope.wildcard_imports.is_empty());
    }
}
