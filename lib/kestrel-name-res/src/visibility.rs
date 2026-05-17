//! Visibility checks for name resolution.
//!
//! Determines whether a declaration is visible from a given context,
//! respecting public/private/internal/fileprivate modifiers.

use kestrel_ast_builder::{FileId, Name, NodeKind, Vis};
use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::helpers::{ancestor_module, is_ancestor_of};

// ===== IsVisibleFrom =====

/// Query: is `target` visible from `context`?
///
/// Checks the Vis component on target against the structural relationship
/// between target and context in the entity hierarchy.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct IsVisibleFrom {
    pub target: Entity,
    pub context: Entity,
}

impl QueryFn for IsVisibleFrom {
    type Output = bool;

    fn execute(&self, ctx: &QueryContext<'_>) -> bool {
        match ctx.get::<Vis>(self.target) {
            // No visibility modifier → always visible (default)
            None => true,
            Some(Vis::Public) => true,
            Some(Vis::Private) => {
                // Visible within the parent scope and all its descendants
                let Some(scope) = ctx.parent_of(self.target) else {
                    return true;
                };
                is_ancestor_of(ctx, scope, self.context)
            },
            Some(Vis::Fileprivate) => {
                // Visible within the same file
                let target_file = find_file_id(ctx, self.target);
                let context_file = find_file_id(ctx, self.context);
                match (target_file, context_file) {
                    (Some(t), Some(c)) => t == c,
                    // If no file info, fall back to parent scope check
                    _ => {
                        let Some(scope) = ctx.parent_of(self.target) else {
                            return true;
                        };
                        is_ancestor_of(ctx, scope, self.context)
                    },
                }
            },
            Some(Vis::Internal) => {
                // Visible within the same top-level module subtree
                let target_mod = ancestor_module(ctx, self.target);
                let context_mod = ancestor_module(ctx, self.context);
                match (target_mod, context_mod) {
                    (Some(t), Some(c)) => {
                        // Walk to find the top-level module (child of root)
                        let t_top = top_level_module(ctx, t);
                        let c_top = top_level_module(ctx, c);
                        t_top == c_top
                    },
                    _ => true,
                }
            },
        }
    }
}

/// Walk ancestors to find the FileId component for an entity.
fn find_file_id(ctx: &QueryContext<'_>, entity: Entity) -> Option<Entity> {
    let mut current = entity;
    loop {
        if let Some(file_id) = ctx.get::<FileId>(current) {
            return Some(file_id.0);
        }
        current = ctx.parent_of(current)?;
    }
}

/// Find the top-level module (direct child of root) for a given module.
/// If the module IS the root or a top-level module, returns it.
fn top_level_module(ctx: &QueryContext<'_>, module: Entity) -> Entity {
    let mut current = module;
    loop {
        let Some(parent) = ctx.parent_of(current) else {
            return current;
        };
        // If parent is root (has name "<root>"), current is top-level
        if ctx.get::<Name>(parent).is_some_and(|n| n.0 == "<root>") {
            return current;
        }
        // If parent is not a module, current is as high as we go
        if ctx.get::<NodeKind>(parent) != Some(&NodeKind::Module) {
            return current;
        }
        current = parent;
    }
}

// ===== VisibleChildrenByName =====

/// Query: find children of `parent` with the given `name` that are
/// visible from `context`.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct VisibleChildrenByName {
    pub parent: Entity,
    pub name: String,
    pub context: Entity,
}

impl QueryFn for VisibleChildrenByName {
    type Output = Vec<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<Entity> {
        ctx.children_of(self.parent)
            .iter()
            .filter(|&&child| {
                ctx.get::<Name>(child).is_some_and(|n| n.0 == self.name)
                    && ctx.query(IsVisibleFrom {
                        target: child,
                        context: self.context,
                    })
            })
            .copied()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast_builder::Typed;
    use kestrel_hecs::World;

    /// Build: root > mod_a > [pub_struct, priv_struct, internal_struct]
    fn setup() -> (World, Entity, Entity, Entity, Entity, Entity) {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        let mod_a = world.spawn();
        world.set(mod_a, NodeKind::Module);
        world.set(mod_a, Name("A".into()));
        world.set_parent(mod_a, root);

        let pub_s = world.spawn();
        world.set(pub_s, NodeKind::Struct);
        world.set(pub_s, Name("PubS".into()));
        world.set(pub_s, Vis::Public);
        world.set(pub_s, Typed);
        world.set_parent(pub_s, mod_a);

        let priv_s = world.spawn();
        world.set(priv_s, NodeKind::Struct);
        world.set(priv_s, Name("PrivS".into()));
        world.set(priv_s, Vis::Private);
        world.set(priv_s, Typed);
        world.set_parent(priv_s, mod_a);

        let internal_s = world.spawn();
        world.set(internal_s, NodeKind::Struct);
        world.set(internal_s, Name("InternalS".into()));
        world.set(internal_s, Vis::Internal);
        world.set(internal_s, Typed);
        world.set_parent(internal_s, mod_a);

        (world, root, mod_a, pub_s, priv_s, internal_s)
    }

    #[test]
    fn public_visible_everywhere() {
        let (mut world, _, _, pub_s, _, _) = setup();
        let other = world.spawn();
        let ctx = world.query_context();
        assert!(ctx.query(IsVisibleFrom {
            target: pub_s,
            context: other,
        }));
    }

    #[test]
    fn private_visible_in_same_scope() {
        let (world, _, mod_a, _, priv_s, _) = setup();
        let ctx = world.query_context();
        // Visible from parent module
        assert!(ctx.query(IsVisibleFrom {
            target: priv_s,
            context: mod_a,
        }));
    }

    #[test]
    fn private_not_visible_outside() {
        let (world, root, _, _, priv_s, _) = setup();
        let ctx = world.query_context();
        assert!(!ctx.query(IsVisibleFrom {
            target: priv_s,
            context: root,
        }));
    }

    #[test]
    fn no_vis_always_visible() {
        let (mut world, _, mod_a, _, _, _) = setup();
        // Entity with no Vis component
        let no_vis = world.spawn();
        world.set(no_vis, NodeKind::Function);
        world.set(no_vis, Name("f".into()));
        world.set_parent(no_vis, mod_a);

        let other = world.spawn();
        let ctx = world.query_context();
        assert!(ctx.query(IsVisibleFrom {
            target: no_vis,
            context: other,
        }));
    }

    #[test]
    fn visible_children_by_name_filters() {
        let (world, root, mod_a, pub_s, _, _) = setup();
        let ctx = world.query_context();

        // From root, only public is visible
        let visible = ctx.query(VisibleChildrenByName {
            parent: mod_a,
            name: "PubS".into(),
            context: root,
        });
        assert_eq!(visible, vec![pub_s]);

        // Private not visible from root
        let visible = ctx.query(VisibleChildrenByName {
            parent: mod_a,
            name: "PrivS".into(),
            context: root,
        });
        assert!(visible.is_empty());

        // Private visible from within mod_a
        let visible = ctx.query(VisibleChildrenByName {
            parent: mod_a,
            name: "PrivS".into(),
            context: mod_a,
        });
        assert_eq!(visible.len(), 1);
    }
}
