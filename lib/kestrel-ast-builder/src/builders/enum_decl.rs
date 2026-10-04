//! Enum and EnumCase declaration builders.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::ast::{self, AstNode, HasName};
use kestrel_syntax_tree::utils::get_decl_span;

use super::helpers::*;
use super::type_param::build_type_parameters;
use crate::ast_type::lower_opt_type;
use crate::components::*;

/// Build an enum declaration entity from CST.
///
/// Components: NodeKind::Enum, Name, FileId, Vis, Typed,
/// [IsIndirect], [Conformances], [TypeParams], [WhereClause],
/// [Attributes], [Documentation]
pub fn build_enum(
    world: &mut World,
    node: &ast::EnumDeclaration,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) -> (Entity, Vec<ast::Item>) {
    let syntax = node.syntax();
    let entity = world.spawn();

    world.set(entity, NodeKind::Enum);
    world.set(entity, FileId(file_entity));
    world.set(entity, Typed);
    world.set(entity, DeclSpan(get_decl_span(syntax, file_id)));
    world.set(entity, CstNode(syntax.clone()));
    world.set_parent(entity, parent);

    if let Some(name) = node.name_text() {
        world.set(entity, Name(name));
    }
    if node.indirect_modifier().is_some() {
        world.set(entity, IsIndirect);
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node, file_id);
    set_documentation(world, entity, syntax);
    set_conformances(world, entity, node, file_id);
    set_where_clause(world, entity, node, file_id);
    build_type_parameters(world, entity, node, file_entity, file_id);

    let members = node
        .enum_body()
        .map(|b| b.items().collect())
        .unwrap_or_default();
    (entity, members)
}

/// Build an enum case declaration entity from CST.
///
/// Components: NodeKind::EnumCase, Name, FileId, [Callable], [Documentation]
pub fn build_enum_case(
    world: &mut World,
    node: &ast::EnumCaseDeclaration,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let syntax = node.syntax();
    let entity = world.spawn();

    world.set(entity, NodeKind::EnumCase);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_decl_span(syntax, file_id)));
    world.set(entity, CstNode(syntax.clone()));
    world.set_parent(entity, parent);

    if let Some(name) = node.name_text() {
        world.set(entity, Name(name));
    }
    set_documentation(world, entity, syntax);

    // Associated values: `case some(value: T)`. The label is the name.
    let params: Vec<AstParam> = node
        .enum_case_parameter_list()
        .into_iter()
        .flat_map(|list| list.enum_case_parameters())
        .map(|param| {
            let label = param.name_text();
            AstParam {
                name: label.clone().unwrap_or_default(),
                label,
                ty: lower_opt_type(param.ty(), file_id),
                default_entity: None,
                pattern: None,
                is_mut: false,
                is_consuming: false,
            }
        })
        .collect();
    if !params.is_empty() {
        world.set(
            entity,
            Callable {
                params,
                receiver: None,
            },
        );
    }
}
