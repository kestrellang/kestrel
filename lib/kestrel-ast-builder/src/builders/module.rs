//! Module hierarchy find-or-create.
//!
//! Walks a dotted module path left-to-right. For each segment, scans
//! `children_of(parent)` for an existing `NodeKind::Module` + `Name` match.
//! Creates if not found. Module entities have NO `FileId`.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::ast;

use crate::components::{Name, NodeKind};

/// Find or create a module entity for the given path segment under parent.
fn find_or_create_module(world: &mut World, parent: Entity, segment: &str) -> Entity {
    for &child in world.children_of(parent) {
        if world.get::<NodeKind>(child) == Some(&NodeKind::Module)
            && world.get::<Name>(child).is_some_and(|n| n.0 == segment)
        {
            return child;
        }
    }

    let entity = world.spawn();
    world.set(entity, NodeKind::Module);
    world.set(entity, Name(segment.to_string()));
    world.set_parent(entity, parent);
    entity
}

/// Find-or-create the module hierarchy a `module A.B.C` declaration names.
/// Returns the deepest module entity.
pub fn resolve_module_path(
    world: &mut World,
    root: Entity,
    decl: &ast::ModuleDeclaration,
) -> Entity {
    let Some(path) = decl.module_path() else {
        return root;
    };
    let mut current = root;
    for segment in path.segment_tokens() {
        current = find_or_create_module(world, current, segment.text());
    }
    current
}
