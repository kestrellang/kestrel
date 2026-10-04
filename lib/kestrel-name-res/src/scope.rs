//! Scope construction for name resolution.
//!
//! Builds a Scope for each declaration entity, containing its local
//! declarations, selective imports, and wildcard import sources.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use kestrel_ast_builder::{
    ImportAlias, ImportItems, ModulePath, Name, NodeKind, QualifiedTarget, Static,
};
use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::extensions::{ExtensionTargetEntity, ExtensionsFor};
use crate::helpers::is_in_std_module;
use crate::resolve_module::{ResolveModulePath, StdModules};
use crate::visibility::VisibleChildrenByName;

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

/// Hash for Scope: HashMap doesn't implement Hash, so we sort
/// entries by key to produce a deterministic hash.
impl Hash for Scope {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity.hash(state);

        // Sort HashMap entries by key for deterministic hashing
        let mut selective: Vec<_> = self.selective_imports.iter().collect();
        selective.sort_by_key(|(k, _)| *k);
        for (k, v) in &selective {
            k.hash(state);
            v.hash(state);
        }

        let mut decls: Vec<_> = self.declarations.iter().collect();
        decls.sort_by_key(|(k, _)| *k);
        for (k, v) in &decls {
            k.hash(state);
            v.hash(state);
        }

        self.wildcard_imports.hash(state);
        self.parent.hash(state);
    }
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

        let is_type_scope = ctx
            .get::<NodeKind>(self.entity)
            .is_some_and(NodeKind::is_type_scope);
        if is_type_scope {
            // A type's lexical names are its member scope: every part of the
            // type (body and all extensions) contributes the same names.
            for child in member_scope_children(ctx, self.entity, self.root) {
                if let Some(name) = ctx.get::<Name>(child) {
                    declarations.entry(name.0.clone()).or_default().push(child);
                }
            }
        } else {
            for &child in ctx.children_of(self.entity) {
                if ctx.get::<NodeKind>(child) == Some(&NodeKind::Import) {
                    process_import(
                        ctx,
                        child,
                        self.root,
                        &mut selective_imports,
                        &mut wildcard_imports,
                    );
                } else if let Some(name) = ctx.get::<Name>(child) {
                    // Non-import child with a name → local declaration
                    declarations.entry(name.0.clone()).or_default().push(child);
                }
            }
        }

        // Auto-imports: if this is a non-std module, add all std leaf modules
        // as wildcards. Only apply at module scope — otherwise auto-imports
        // would shadow local declarations in the enclosing module when name
        // lookup reaches a nested scope (function, struct, etc.) first.
        if ctx.get::<NodeKind>(self.entity) == Some(&NodeKind::Module)
            && !is_in_std_module(ctx, self.entity)
        {
            let std_modules = ctx.query(StdModules { root: self.root });
            wildcard_imports.extend(std_modules);
        }

        // Dedup selective imports (same entity imported from multiple files in same module)
        for entries in selective_imports.values_mut() {
            entries.sort_by_key(|e| e.index());
            entries.dedup();
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

/// The declarations a type scope binds lexically: the scope's own type
/// parameters, plus every non-instance member (nested types, type aliases,
/// enum cases, static members) of every part of the type — its body and all
/// of its extensions — so a type body and its extensions see one member
/// scope. Instance members are never lexical bindings; they are reached only
/// through `self.` (audit H3: a bare field name used to resolve to the field
/// entity and fail in codegen).
fn member_scope_children(ctx: &QueryContext<'_>, scope: Entity, root: Entity) -> Vec<Entity> {
    // Type parameters are per part: an extension's free RHS parameters and a
    // body's `[T]` are not in scope in the other parts (extension LHS
    // parameters resolve through `ExtensionLhsParams`).
    let mut out: Vec<Entity> = ctx
        .children_of(scope)
        .iter()
        .copied()
        .filter(|&c| ctx.get::<NodeKind>(c) == Some(&NodeKind::TypeParameter))
        .collect();
    let nominal = if ctx.get::<NodeKind>(scope) == Some(&NodeKind::Extension) {
        ctx.query(ExtensionTargetEntity {
            extension: scope,
            root,
        })
    } else {
        Some(scope)
    };
    // An extension whose target does not resolve contributes only itself.
    let parts: Vec<Entity> = match nominal {
        Some(nominal) => std::iter::once(nominal)
            .chain(ctx.query(ExtensionsFor {
                target: nominal,
                root,
            }))
            .collect(),
        None => vec![scope],
    };
    for part in parts {
        out.extend(ctx.children_of(part).iter().copied().filter(|&c| {
            // A qualified associated-type binding (`type Iterable.Item = …`)
            // witnesses one conformance; it names `Item` only inside its own
            // extension, or it would collide with the type's own `Item`.
            is_lexical_member(ctx, c) && (part == scope || ctx.get::<QualifiedTarget>(c).is_none())
        }));
    }
    out.dedup();
    out
}

/// Whether a member of a type is a lexical binding in the type's scope:
/// everything but instance members and per-part type parameters.
fn is_lexical_member(ctx: &QueryContext<'_>, member: Entity) -> bool {
    match ctx.get::<NodeKind>(member) {
        Some(
            NodeKind::Field
            | NodeKind::Function
            | NodeKind::Subscript
            | NodeKind::Initializer
            | NodeKind::Deinit
            | NodeKind::Setter
            | NodeKind::RefAccessor,
        ) => ctx.get::<Static>(member).is_some(),
        Some(NodeKind::TypeParameter | NodeKind::Import | NodeKind::ParamDefault) | None => false,
        Some(
            NodeKind::Module
            | NodeKind::Struct
            | NodeKind::Enum
            | NodeKind::EnumCase
            | NodeKind::Protocol
            | NodeKind::Extension
            | NodeKind::TypeAlias,
        ) => true,
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
            // The module's children of that name that the importing file may
            // see — the same visibility rule wildcard imports apply (audit H5:
            // a selective import used to bind private declarations).
            let matches = ctx.query(VisibleChildrenByName {
                parent: module_entity,
                name: item.name.clone(),
                context: import,
            });

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
        world.set(root, Name(Name::ROOT.into()));

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
        world.set(sel_import, ModulePath(vec!["std".into(), "core".into()]));
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

        let scope = ctx.query(ScopeFor { entity: core, root });

        // std.core should NOT have auto-imported wildcard modules
        // (it only has its own local declarations)
        // Wildcard imports should be empty since it's in std
        assert!(scope.wildcard_imports.is_empty());
    }
}
