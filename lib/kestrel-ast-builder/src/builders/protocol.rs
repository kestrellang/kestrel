//! Protocol declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree2::utils::{extract_name, get_decl_span};
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};

use super::helpers::*;
use super::type_param::build_type_parameters;
use crate::components::*;

/// Build a protocol declaration entity from CST.
///
/// Components: NodeKind::Protocol, Name, FileId, Vis, Typed,
/// [Conformances], [TypeParams], [WhereClause], [Attributes], [Documentation]
pub fn build_protocol(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) -> (Entity, Option<SyntaxNode>) {
    let entity = world.spawn();

    world.set(entity, NodeKind::Protocol);
    world.set(entity, FileId(file_entity));
    world.set(entity, Typed);
    world.set(entity, DeclSpan(get_decl_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set_parent(entity, parent);

    if let Some(name) = extract_name(node) {
        world.set(entity, Name(name));
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node, file_id);
    set_documentation(world, entity, node);
    set_conformances(world, entity, node, file_id);
    set_where_clause(world, entity, node, file_id);
    build_type_parameters(world, entity, node, file_entity, file_id);

    let body = node
        .children()
        .find(|c| c.kind() == SyntaxKind::ProtocolBody);
    (entity, body)
}
