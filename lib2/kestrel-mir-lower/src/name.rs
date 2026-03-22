//! Qualified name generation from entity hierarchy.

use kestrel_ast_builder::{Name, NodeKind};
use kestrel_hecs::{Entity, World};

/// Build a qualified name by walking the entity's parent chain.
/// Produces names like "std.core.Bool.init".
/// Stops at root (entity with no parent or no Name component).
pub fn qualified_name(world: &World, entity: Entity) -> String {
    let mut parts = Vec::new();
    let mut current = Some(entity);
    while let Some(e) = current {
        if let Some(name) = world.get::<Name>(e) {
            // Skip the root module "<root>"
            if name.0 != "<root>" {
                parts.push(name.0.clone());
            }
        } else {
            // Entities without Name components (e.g., initializers, subscripts)
            // get a synthetic name based on their NodeKind
            match world.get::<NodeKind>(e) {
                Some(NodeKind::Initializer) => parts.push("init".to_string()),
                Some(NodeKind::Subscript) => parts.push("subscript".to_string()),
                Some(NodeKind::Deinit) => parts.push("deinit".to_string()),
                _ => {}
            }
        }
        current = world.parent_of(e);
    }
    parts.reverse();
    if parts.is_empty() {
        format!("<entity:{:?}>", entity)
    } else {
        parts.join(".")
    }
}
