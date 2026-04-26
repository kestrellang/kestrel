//! Qualified name generation from entity hierarchy.

use kestrel_ast::AstType;
use kestrel_ast_builder::{ExtensionTarget, Name, NodeKind};
use kestrel_hecs::{Entity, World};

/// Build a qualified name by walking the entity's parent chain.
/// Produces names like "std.core.Bool.init".
/// Stops at root (entity with no parent or no Name component).
///
/// Extensions have no `Name`, but members in different `extend` blocks in the
/// same module must produce distinct symbols. When an Extension is in the
/// parent chain, inject the extended type's path segments so
/// `extend Bool: Deserialize { fromValue }` becomes
/// `...deserialize.Bool.fromValue` and `extend Value: Deserialize { fromValue }`
/// becomes `...deserialize.Value.fromValue` instead of colliding.
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
            match world.get::<NodeKind>(e) {
                Some(NodeKind::Initializer) => parts.push("init".to_string()),
                Some(NodeKind::Subscript) => parts.push("subscript".to_string()),
                Some(NodeKind::Deinit) => parts.push("deinit".to_string()),
                Some(NodeKind::Extension) => {
                    if let Some(target) = world.get::<ExtensionTarget>(e) {
                        for seg in extension_target_segments(&target.0).into_iter().rev() {
                            parts.push(seg);
                        }
                    }
                },
                _ => {},
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

/// Extract dotted path segments from an extension target's AST type.
/// Type arguments are dropped — only the name path is kept.
fn extension_target_segments(ty: &AstType) -> Vec<String> {
    match ty {
        AstType::Named { segments, .. } => segments.iter().map(|s| s.name.clone()).collect(),
        _ => Vec::new(),
    }
}
