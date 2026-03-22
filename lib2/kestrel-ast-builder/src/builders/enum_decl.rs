//! Enum and EnumCase declaration builders.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};
use kestrel_syntax_tree2::utils::{extract_name, find_child, get_node_span};

use crate::ast_type::ast_type_from_cst;
use crate::components::*;
use super::helpers::*;
use super::type_param::build_type_parameters;

/// Build an enum declaration entity from CST.
///
/// Components: NodeKind::Enum, Name, FileId, Vis, Typed,
/// [IsIndirect], [Conformances], [TypeParams], [WhereClause],
/// [Attributes], [Documentation]
pub fn build_enum(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) -> (Entity, Option<SyntaxNode>) {
    let entity = world.spawn();

    world.set(entity, NodeKind::Enum);
    world.set(entity, FileId(file_entity));
    world.set(entity, Typed);
    world.set(entity, DeclSpan(get_node_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set_parent(entity, parent);

    if let Some(name) = extract_name(node) {
        world.set(entity, Name(name));
    }

    // Check for top-level indirect modifier
    if find_child(node, SyntaxKind::IndirectModifier).is_some() {
        world.set(entity, IsIndirect);
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node);
    set_documentation(world, entity, node);
    set_conformances(world, entity, node, file_id);
    set_where_clause(world, entity, node, file_id);
    build_type_parameters(world, entity, node, file_entity, file_id);

    let body = node.children().find(|c| c.kind() == SyntaxKind::EnumBody);
    (entity, body)
}

/// Build an enum case declaration entity from CST.
///
/// Components: NodeKind::EnumCase, Name, FileId, Vis, [Callable]
pub fn build_enum_case(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let entity = world.spawn();

    world.set(entity, NodeKind::EnumCase);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_node_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set_parent(entity, parent);

    if let Some(name) = extract_name(node) {
        world.set(entity, Name(name));
    }

    set_visibility(world, entity, node);

    // Enum cases with associated values get Callable (from EnumCaseParameterList)
    if let Some(param_list) = find_child(node, SyntaxKind::EnumCaseParameterList) {
        let params: Vec<AstParam> = param_list
            .children()
            .filter(|c| c.kind() == SyntaxKind::EnumCaseParameter)
            .filter_map(|param_node| {
                // Enum case params: label from Name > Identifier, type from Ty
                let label = extract_name(&param_node);

                let ty = param_node
                    .children()
                    .find(|c| is_type_kind(c.kind()))
                    .and_then(|c| ast_type_from_cst(&c, file_id));

                // For enum case params, the label IS the name
                let name = label.clone().unwrap_or_default();
                Some(AstParam {
                    label,
                    name,
                    ty,
                    default_entity: None,
                })
            })
            .collect();

        if !params.is_empty() {
            world.set(entity, Callable { params, receiver: None });
        }
    }
}
