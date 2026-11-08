//! Qualified names from the entity hierarchy: `std.core.Bool.init`.
//!
//! The one naming used for symbols (mir-lower's mangled names) and for any
//! order that must not depend on entity numbering (the canonical order of
//! `any P and Q` bounds and of existential parent tables). Entity indices are
//! internal and may differ between builds; these names don't.

use kestrel_ast::AstType;
use kestrel_hecs::component::Component;
use kestrel_hecs::{Entity, QueryContext, World};

use crate::{ExtensionTarget, Name, NodeKind};

/// What [`qualified_name`] reads: a component and a parent link. Implemented
/// for `World` and for `QueryContext`, so a query's reads stay tracked.
pub trait EntityTree {
    fn component<T: Component>(&self, entity: Entity) -> Option<&T>;
    fn parent(&self, entity: Entity) -> Option<Entity>;
}

impl EntityTree for World {
    fn component<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.get::<T>(entity)
    }
    fn parent(&self, entity: Entity) -> Option<Entity> {
        self.parent_of(entity)
    }
}

impl EntityTree for QueryContext<'_> {
    fn component<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.get::<T>(entity)
    }
    fn parent(&self, entity: Entity) -> Option<Entity> {
        self.parent_of(entity)
    }
}

/// Build a qualified name by walking the entity's parent chain.
/// Produces names like "std.core.Bool.init".
///
/// Extensions have no `Name`, but members in different `extend` blocks in the
/// same module must produce distinct symbols. When an Extension is in the
/// parent chain, inject the extended type's path segments so
/// `extend Bool: Deserialize { fromValue }` becomes
/// `...deserialize.Bool.fromValue`.
pub fn qualified_name<W: EntityTree + ?Sized>(world: &W, entity: Entity) -> String {
    let mut parts = Vec::new();
    let mut current = Some(entity);
    while let Some(e) = current {
        if let Some(name) = world.component::<Name>(e) {
            if !name.is_root() {
                parts.push(name.0.clone());
            }
        } else {
            match world.component::<NodeKind>(e) {
                Some(NodeKind::Initializer) => parts.push("init".to_string()),
                Some(NodeKind::Subscript) => parts.push("subscript".to_string()),
                Some(NodeKind::Deinit) => parts.push("deinit".to_string()),
                Some(NodeKind::Extension) => {
                    if let Some(target) = world.component::<ExtensionTarget>(e) {
                        for seg in extension_target_segments(&target.0).into_iter().rev() {
                            parts.push(seg);
                        }
                    }
                },
                _ => {},
            }
        }
        current = world.parent(e);
    }
    parts.reverse();
    if parts.is_empty() {
        format!("<entity:{:?}>", entity)
    } else {
        parts.join(".")
    }
}

/// Extract dotted path segments from an extension target's AST type.
fn extension_target_segments(ty: &AstType) -> Vec<String> {
    match ty {
        AstType::Named { segments, .. } => segments.iter().map(|s| s.name.clone()).collect(),
        _ => Vec::new(),
    }
}
