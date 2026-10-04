//! Struct declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::SyntaxNodePtr;
use kestrel_syntax_tree::ast::{self, AstNode, HasName};
use kestrel_syntax_tree::utils::get_decl_span;

use super::helpers::*;
use super::type_param::build_type_parameters;
use crate::components::*;

/// Build a struct declaration entity from CST.
///
/// Components: NodeKind::Struct, Name, FileId, Vis, Typed,
/// [Conformances], [TypeParams], [WhereClause], [Attributes], [Documentation]
///
/// Returns the entity and its member items for the caller to build.
pub fn build_struct(
    world: &mut World,
    node: &ast::StructDeclaration,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) -> (Entity, Vec<ast::Item>) {
    let syntax = node.syntax();
    let entity = world.spawn();

    world.set(entity, NodeKind::Struct);
    world.set(entity, FileId(file_entity));
    world.set(entity, Typed);
    world.set(entity, DeclSpan(get_decl_span(syntax, file_id)));
    world.set(entity, CstNode(SyntaxNodePtr::new(&syntax)));
    world.set_parent(entity, parent);

    if let Some(name) = node.name_text() {
        world.set(entity, Name(name));
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node, file_id);
    set_documentation(world, entity, syntax);
    set_conformances(world, entity, node, file_id);
    set_where_clause(world, entity, node, file_id);
    build_type_parameters(world, entity, node, file_entity, file_id);

    let members = node
        .struct_body()
        .map(|b| b.items().collect())
        .unwrap_or_default();
    (entity, members)
}
