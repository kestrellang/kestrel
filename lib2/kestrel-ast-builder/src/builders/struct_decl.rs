//! Struct declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree2::utils::get_decl_span;
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};

use super::helpers::*;
use super::type_param::build_type_parameters;
use crate::components::*;

/// Build a struct declaration entity from CST.
///
/// Components: NodeKind::Struct, Name, FileId, Vis, Typed,
/// [Conformances], [TypeParams], [WhereClause], [Attributes], [Documentation]
///
/// Returns the entity and pushes body children onto the stack for processing.
pub fn build_struct(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) -> (Entity, Option<SyntaxNode>) {
    let entity = world.spawn();

    world.set(entity, NodeKind::Struct);
    world.set(entity, FileId(file_entity));
    world.set(entity, Typed);
    world.set(entity, DeclSpan(get_decl_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set_parent(entity, parent);

    // Name
    if let Some(name) = kestrel_syntax_tree2::utils::extract_name(node) {
        world.set(entity, Name(name));
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node);
    set_documentation(world, entity, node);
    set_conformances(world, entity, node, file_id);
    set_where_clause(world, entity, node, file_id);
    build_type_parameters(world, entity, node, file_entity, file_id);

    // Find body for child processing
    let body = node.children().find(|c| c.kind() == SyntaxKind::StructBody);

    (entity, body)
}
