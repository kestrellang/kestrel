//! Type parameter extraction and entity creation.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};
use kestrel_syntax_tree2::utils::{extract_name, find_child, get_node_span};

use crate::ast_type::ast_type_from_cst;
use crate::components::*;
use super::helpers::is_type_kind;

/// Extract type parameters from a TypeParameterList child and create entities.
///
/// Returns the list of type parameter entities and sets the TypeParams
/// component on the parent entity.
pub fn build_type_parameters(
    world: &mut World,
    parent: Entity,
    node: &SyntaxNode,
    file_entity: Entity,
    file_id: usize,
) {
    let tp_list = match find_child(node, SyntaxKind::TypeParameterList) {
        Some(list) => list,
        None => return,
    };

    let mut param_entities = Vec::new();

    for child in tp_list.children() {
        if child.kind() != SyntaxKind::TypeParameter {
            continue;
        }

        // TypeParameter has Name > Identifier in CST
        let name = match extract_name(&child) {
            Some(n) => n,
            None => continue,
        };

        let entity = world.spawn();
        world.set(entity, NodeKind::TypeParameter);
        world.set(entity, Name(name));
        world.set(entity, FileId(file_entity));
        world.set(entity, DeclSpan(get_node_span(&child, file_id)));
        world.set(entity, CstNode(child.clone()));
        world.set_parent(entity, parent);

        // Default type from DefaultType child
        if let Some(default_node) = find_child(&child, SyntaxKind::DefaultType) {
            if let Some(ty) = default_node
                .children()
                .find(|c| is_type_kind(c.kind()))
                .and_then(|c| ast_type_from_cst(&c, file_id))
            {
                world.set(entity, TypeAnnotation(ty));
            }
        }

        param_entities.push(entity);
    }

    if !param_entities.is_empty() {
        world.set(parent, TypeParams(param_entities));
    }
}
