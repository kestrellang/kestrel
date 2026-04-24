//! Module hierarchy find-or-create.
//!
//! Walks a dotted module path left-to-right. For each segment, scans
//! `children_of(parent)` for an existing `NodeKind::Module` + `Name` match.
//! Creates if not found. Module entities have NO `FileId`.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::components::{Name, NodeKind};

/// Find or create a module entity for the given path segment under parent.
fn find_or_create_module(world: &mut World, parent: Entity, segment: &str) -> Entity {
    // Check existing children for a matching module
    for &child in world.children_of(parent) {
        if world.get::<NodeKind>(child) == Some(&NodeKind::Module)
            && world.get::<Name>(child).is_some_and(|n| n.0 == segment)
        {
            return child;
        }
    }

    // Create new module entity
    let entity = world.spawn();
    world.set(entity, NodeKind::Module);
    world.set(entity, Name(segment.to_string()));
    world.set_parent(entity, parent);
    entity
}

/// Extract module path from a ModuleDeclaration node and find-or-create
/// the module hierarchy. Returns the deepest module entity.
pub fn resolve_module_path(world: &mut World, root: Entity, module_node: &SyntaxNode) -> Entity {
    // Extract path segments from the ModulePath child
    let path_node = match module_node
        .children()
        .find(|c| c.kind() == SyntaxKind::ModulePath)
    {
        Some(p) => p,
        None => return root,
    };

    let segments: Vec<String> = path_node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string())
        .collect();

    // Walk the path, creating modules as needed
    let mut current = root;
    for segment in &segments {
        current = find_or_create_module(world, current, segment);
    }
    current
}
