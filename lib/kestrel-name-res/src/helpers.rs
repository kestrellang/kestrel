//! Helper functions for hierarchy walking and entity lookups.
//!
//! These are used by multiple query implementations. All functions
//! take a `QueryContext` and record dependencies automatically.

use kestrel_ast_builder::{Name, NodeKind, Subscript as SubscriptMarker};
use kestrel_hecs::{Entity, QueryContext};

use crate::extensions::ExtensionsFor;
use crate::visibility::{IsVisibleFrom, VisibleChildrenByName};

/// Walk parent_of chain to find the nearest ancestor with NodeKind::Module.
/// Returns None if no module ancestor exists (e.g. entity is the root).
pub fn ancestor_module(ctx: &QueryContext<'_>, entity: Entity) -> Option<Entity> {
    let mut current = entity;
    loop {
        if ctx.get::<NodeKind>(current) == Some(&NodeKind::Module) {
            return Some(current);
        }
        current = ctx.parent_of(current)?;
    }
}

/// Check if `ancestor` is an ancestor of `descendant` (or is the same entity).
/// Walks the parent chain from descendant upward.
pub fn is_ancestor_of(ctx: &QueryContext<'_>, ancestor: Entity, descendant: Entity) -> bool {
    let mut current = descendant;
    loop {
        if current == ancestor {
            return true;
        }
        match ctx.parent_of(current) {
            Some(parent) => current = parent,
            None => return false,
        }
    }
}

/// Find direct children of `parent` that have a matching Name component.
/// Returns all matching children (may be multiple for overloaded functions).
pub fn find_children_by_name(ctx: &QueryContext<'_>, parent: Entity, name: &str) -> Vec<Entity> {
    ctx.children_of(parent)
        .iter()
        .filter(|&&child| ctx.get::<Name>(child).is_some_and(|n| n.0 == name))
        .copied()
        .collect()
}

/// Check if an entity is inside a `std.*` module.
/// Walks ancestors looking for a module named "std".
pub fn is_in_std_module(ctx: &QueryContext<'_>, entity: Entity) -> bool {
    let mut current = entity;
    loop {
        if ctx.get::<NodeKind>(current) == Some(&NodeKind::Module)
            && ctx.get::<Name>(current).is_some_and(|n| n.0 == "std")
        {
            return true;
        }
        match ctx.parent_of(current) {
            Some(parent) => current = parent,
            None => return false,
        }
    }
}

/// The name `entity` answers to in member lookups, or `None` for members
/// that can't be addressed by name.
///
/// The entity's `Name` component wins; nameless callables fall back to the
/// keyword sentinels `"init"` (Initializer NodeKind) and `"subscript"`
/// (Subscript marker). Both sentinels are reserved keywords, so they can't
/// collide with a user-declared member name. Single source of truth for
/// member-name matching — `member_name_matches` and the build-time
/// `MemberMap` name index both derive from it.
pub(crate) fn member_lookup_name<'a>(
    ctx: &'a QueryContext<'_>,
    entity: Entity,
) -> Option<&'a str> {
    if let Some(n) = ctx.get::<Name>(entity) {
        return Some(n.0.as_str());
    }
    if ctx.get::<NodeKind>(entity) == Some(&NodeKind::Initializer) {
        return Some("init");
    }
    if ctx.get::<SubscriptMarker>(entity).is_some() {
        return Some("subscript");
    }
    None
}

/// Does `entity` answer to the query name? See `member_lookup_name` for
/// the matching rule (literal `Name` or init/subscript keyword sentinel).
pub(crate) fn member_name_matches(ctx: &QueryContext<'_>, entity: Entity, query: &str) -> bool {
    member_lookup_name(ctx, entity) == Some(query)
}

/// Search extensions of `target` for visible members named `member_name`,
/// keeping only those that pass `filter`. Returns the matches from the
/// first extension that has any (extensions are not merged across), or
/// empty if none do. Shared by value-path resolution for extension static
/// methods and associated-type static members.
pub(crate) fn find_in_extensions(
    ctx: &QueryContext<'_>,
    target: Entity,
    member_name: &str,
    context: Entity,
    root: Entity,
    filter: impl Fn(&QueryContext<'_>, Entity) -> bool,
) -> Vec<Entity> {
    let extensions = ctx.query(ExtensionsFor { target, root });
    for &ext in &extensions {
        let matches: Vec<Entity> = ctx
            .query(VisibleChildrenByName {
                parent: ext,
                name: member_name.to_string(),
                context,
            })
            .into_iter()
            .filter(|&e| filter(ctx, e))
            .collect();
        if !matches.is_empty() {
            return matches;
        }
    }
    Vec::new()
}

/// Filter discovered members down to those answering to `name` (including
/// the init/subscript sentinels) that are visible from `context`. Shared by
/// `TypeMembersByName` and `ProtocolMembersByName`; `entity_of` projects the
/// member entity out of each provenance-carrying member record.
pub(crate) fn filter_members_by_name<M>(
    ctx: &QueryContext<'_>,
    members: Vec<M>,
    name: &str,
    context: Entity,
    entity_of: impl Fn(&M) -> Entity,
) -> Vec<M> {
    members
        .into_iter()
        .filter(|m| {
            let target = entity_of(m);
            if !member_name_matches(ctx, target, name) {
                return false;
            }
            ctx.query(IsVisibleFrom { target, context })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_hecs::World;

    fn setup_module_tree() -> (World, Entity, Entity, Entity, Entity) {
        // root > std > core > SomeStruct
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

        let s = world.spawn();
        world.set(s, NodeKind::Struct);
        world.set(s, Name("Int64".into()));
        world.set_parent(s, core);

        (world, root, std, core, s)
    }

    #[test]
    fn ancestor_module_finds_nearest() {
        let (world, _, _, core, s) = setup_module_tree();
        let ctx = world.query_context();
        assert_eq!(ancestor_module(&ctx, s), Some(core));
    }

    #[test]
    fn ancestor_module_returns_self_if_module() {
        let (world, _, std, _, _) = setup_module_tree();
        let ctx = world.query_context();
        assert_eq!(ancestor_module(&ctx, std), Some(std));
    }

    #[test]
    fn is_ancestor_of_self() {
        let (world, root, _, _, _) = setup_module_tree();
        let ctx = world.query_context();
        assert!(is_ancestor_of(&ctx, root, root));
    }

    #[test]
    fn is_ancestor_of_deep() {
        let (world, root, _, _, s) = setup_module_tree();
        let ctx = world.query_context();
        assert!(is_ancestor_of(&ctx, root, s));
    }

    #[test]
    fn is_ancestor_of_false() {
        let (world, _, _, core, s) = setup_module_tree();
        let ctx = world.query_context();
        assert!(!is_ancestor_of(&ctx, s, core));
    }

    #[test]
    fn find_children_by_name_works() {
        let (world, _, _, core, _) = setup_module_tree();
        let ctx = world.query_context();
        let found = find_children_by_name(&ctx, core, "Int64");
        assert_eq!(found.len(), 1);

        let not_found = find_children_by_name(&ctx, core, "String");
        assert!(not_found.is_empty());
    }

    #[test]
    fn is_in_std_module_positive() {
        let (world, _, _, _, s) = setup_module_tree();
        let ctx = world.query_context();
        assert!(is_in_std_module(&ctx, s));
    }

    #[test]
    fn is_in_std_module_negative() {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        let user_mod = world.spawn();
        world.set(user_mod, NodeKind::Module);
        world.set(user_mod, Name("MyApp".into()));
        world.set_parent(user_mod, root);

        let ctx = world.query_context();
        assert!(!is_in_std_module(&ctx, user_mod));
    }
}
