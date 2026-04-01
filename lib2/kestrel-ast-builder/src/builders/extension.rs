//! Extension declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};
use kestrel_syntax_tree2::utils::get_decl_span;

use crate::ast_type::ast_type_from_cst;
use crate::components::*;
use super::helpers::*;

/// Build an extension declaration entity from CST.
///
/// Components: NodeKind::Extension, FileId, ExtensionTarget,
/// [Conformances], [WhereClause], [Attributes]
///
/// Extensions have no Name — they extend an existing type.
pub fn build_extension(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) -> (Entity, Option<SyntaxNode>) {
    let entity = world.spawn();

    world.set(entity, NodeKind::Extension);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_decl_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set_parent(entity, parent);

    // ExtensionTarget — the type being extended is the first type node
    if let Some(target_ty) = node
        .children()
        .find(|c| is_type_kind(c.kind()))
        .and_then(|c| ast_type_from_cst(&c, file_id))
    {
        world.set(entity, ExtensionTarget(target_ty));
    }

    set_attributes(world, entity, node);
    set_conformances(world, entity, node, file_id);
    set_where_clause(world, entity, node, file_id);

    let body = node
        .children()
        .find(|c| c.kind() == SyntaxKind::ExtensionBody);
    (entity, body)
}
