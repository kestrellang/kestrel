//! Conformance resolution queries.
//!
//! Collects all protocols a concrete type entity transitively conforms to,
//! walking direct conformances, extension conformances, and protocol
//! inheritance.

use std::collections::HashSet;

use kestrel_ast::AstType;
use kestrel_ast_builder::{Conformances, ConformanceItem, NodeKind};
use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::extensions::ExtensionsFor;
use crate::resolve_type::{ResolveTypePath, TypeResolution};

// ===== ConformingProtocols =====

/// Query: collect all protocols a concrete type entity transitively conforms to.
///
/// Walks:
/// 1. Direct conformances on the entity (`Conformances` component)
/// 2. Conformances on extensions of this entity (`ExtensionsFor` query)
/// 3. Protocol inheritance — for each discovered protocol, walk its own
///    conformances and extension conformances recursively
///
/// Result is memoized per `(entity, root)`. Callers check membership
/// with `.contains(&protocol)`.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ConformingProtocols {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for ConformingProtocols {
    type Output = Vec<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<Entity> {
        let mut protocols = Vec::new();
        let mut visited = HashSet::new();

        // Direct conformances on the type itself
        gather_protocol_conformances(ctx, self.entity, self.root, &mut protocols, &mut visited);

        // Conformances declared on extensions of this type
        let extensions = ctx.query(ExtensionsFor {
            target: self.entity,
            root: self.root,
        });
        for ext in &extensions {
            gather_protocol_conformances(ctx, *ext, self.root, &mut protocols, &mut visited);
        }

        // Walk extensions of discovered protocols for additional conformances.
        // E.g., `extend Comparable: Less[Self]` adds Less through a protocol extension.
        let mut i = 0;
        while i < protocols.len() {
            let proto = protocols[i];
            let proto_extensions = ctx.query(ExtensionsFor {
                target: proto,
                root: self.root,
            });
            for ext in &proto_extensions {
                gather_protocol_conformances(ctx, *ext, self.root, &mut protocols, &mut visited);
            }
            i += 1;
        }

        protocols
    }

    fn describe(&self) -> String {
        format!("ConformingProtocols({:?})", self.entity)
    }
}

// ===== Helpers =====

/// Gather protocols from an entity's `Conformances` component, recursively
/// walking inherited protocols (protocol parents).
fn gather_protocol_conformances(
    ctx: &QueryContext<'_>,
    entity: Entity,
    root: Entity,
    protocols: &mut Vec<Entity>,
    visited: &mut HashSet<Entity>,
) {
    let Some(conformances) = ctx.get::<Conformances>(entity) else {
        return;
    };

    for item in &conformances.0 {
        let ConformanceItem::Positive(ast_ty, _) = item else { continue };
        let Some(resolved) = resolve_conformance_entity(ctx, ast_ty, entity, root) else {
            continue;
        };

        // Only collect protocol entities
        if ctx.get::<NodeKind>(resolved) != Some(&NodeKind::Protocol) {
            continue;
        }

        if !visited.insert(resolved) {
            continue;
        }

        protocols.push(resolved);

        // Walk inherited protocols transitively
        gather_protocol_conformances(ctx, resolved, root, protocols, visited);
    }
}

/// Resolve a conformance AstType to an entity via `ResolveTypePath`.
///
/// Uses the conformance-bearing entity as resolution context — protocol
/// names are top-level so they resolve from any scope.
fn resolve_conformance_entity(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    context: Entity,
    root: Entity,
) -> Option<Entity> {
    let AstType::Named { segments, .. } = ast_ty else {
        return None;
    };

    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    match ctx.query(ResolveTypePath {
        segments: seg_names,
        context,
        root,
    }) {
        TypeResolution::Found(entity) => Some(entity),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast_builder::Name;
    use kestrel_hecs::World;

    #[test]
    fn no_conformances() {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        let bare_struct = world.spawn();
        world.set(bare_struct, NodeKind::Struct);
        world.set(bare_struct, Name("Bare".into()));
        world.set_parent(bare_struct, root);

        let ctx = world.query_context();
        let protocols = ctx.query(ConformingProtocols {
            entity: bare_struct,
            root,
        });
        assert!(protocols.is_empty());
    }
}
