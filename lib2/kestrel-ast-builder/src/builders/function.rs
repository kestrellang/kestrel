//! Function, initializer, and deinit declaration builders.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};
use kestrel_syntax_tree2::utils::{extract_name, find_child, get_node_span};

use crate::ast_type::ast_type_from_cst;
use crate::components::*;
use super::helpers::*;
use super::params::extract_params;
use super::type_param::build_type_parameters;

/// Build a function declaration entity from CST.
///
/// Components: NodeKind::Function, Name, FileId, Vis, Callable,
/// [TypeAnnotation (return)], [Valued (body)], [Static],
/// [TypeParams], [WhereClause], [Attributes], [Documentation]
pub fn build_function(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let entity = world.spawn();

    world.set(entity, NodeKind::Function);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_node_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set_parent(entity, parent);

    if let Some(name) = extract_name(node) {
        world.set(entity, Name(name));
    }

    // Receiver from mutating/consuming/borrowing keyword
    let receiver = extract_receiver(node);

    // Parameters
    let params = extract_params(node, file_id);
    world.set(entity, Callable { params, receiver });

    // Return type from ReturnType child
    if let Some(return_node) = find_child(node, SyntaxKind::ReturnType) {
        if let Some(ty) = return_node
            .children()
            .find(|c| is_type_kind(c.kind()))
            .and_then(|c| ast_type_from_cst(&c, file_id))
        {
            world.set(entity, TypeAnnotation(ty));
        }
    }

    // Body — CST wraps it in FunctionBody > CodeBlock
    if let Some(fn_body) = find_child(node, SyntaxKind::FunctionBody) {
        if let Some(code_block) = find_child(&fn_body, SyntaxKind::CodeBlock) {
            world.set(entity, Valued(code_block));
        }
    }

    if has_static_modifier(node) {
        world.set(entity, Static);
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node);
    set_documentation(world, entity, node);
    set_where_clause(world, entity, node, file_id);
    build_type_parameters(world, entity, node, file_entity, file_id);
}

/// Build an initializer declaration entity from CST.
///
/// Components: NodeKind::Initializer, FileId, Vis, Callable,
/// [Valued (body)], [TypeParams], [WhereClause], [Attributes]
pub fn build_initializer(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let entity = world.spawn();

    world.set(entity, NodeKind::Initializer);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_node_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set_parent(entity, parent);

    let params = extract_params(node, file_id);
    world.set(entity, Callable { params, receiver: None });

    // Body — CST wraps it in FunctionBody > CodeBlock
    if let Some(fn_body) = find_child(node, SyntaxKind::FunctionBody) {
        if let Some(code_block) = find_child(&fn_body, SyntaxKind::CodeBlock) {
            world.set(entity, Valued(code_block));
        }
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node);
    set_where_clause(world, entity, node, file_id);
    build_type_parameters(world, entity, node, file_entity, file_id);
}

/// Build a deinit declaration entity from CST.
///
/// Components: NodeKind::Deinit, FileId, [Valued (body)]
pub fn build_deinit(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let entity = world.spawn();

    world.set(entity, NodeKind::Deinit);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_node_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set_parent(entity, parent);

    // Body — CST wraps it in FunctionBody > CodeBlock
    if let Some(fn_body) = find_child(node, SyntaxKind::FunctionBody) {
        if let Some(code_block) = find_child(&fn_body, SyntaxKind::CodeBlock) {
            world.set(entity, Valued(code_block));
        }
    }
}

/// Extract receiver kind from function modifier keywords.
fn extract_receiver(node: &SyntaxNode) -> Option<ReceiverKind> {
    for elem in node.children_with_tokens() {
        if let Some(token) = elem.as_token() {
            match token.kind() {
                SyntaxKind::Mutating => return Some(ReceiverKind::Mutating),
                SyntaxKind::Consuming => return Some(ReceiverKind::Consuming),
                // Borrowing is the default for methods — only set if explicit
                _ => {}
            }
        }
    }
    None
}

fn is_type_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Ty
            | SyntaxKind::TyPath
            | SyntaxKind::TyTuple
            | SyntaxKind::TyFunction
            | SyntaxKind::TyArray
            | SyntaxKind::TyDictionary
            | SyntaxKind::TyOptional
            | SyntaxKind::TyResult
            | SyntaxKind::TyUnit
            | SyntaxKind::TyNever
            | SyntaxKind::TyInferred
    )
}
