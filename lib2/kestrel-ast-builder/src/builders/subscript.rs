//! Subscript declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};
use kestrel_syntax_tree2::utils::{find_child, get_node_span};

use crate::ast_type::ast_type_from_cst;
use crate::components::*;
use crate::lower;
use super::helpers::*;
use super::params::extract_params;
use super::type_param::build_type_parameters;

/// Build a subscript declaration entity from CST.
///
/// Components: NodeKind::Subscript, FileId, Vis, Callable, TypeAnnotation,
/// Subscript, Gettable, [Settable], [Static], [TypeParams],
/// [WhereClause], [Attributes]
pub fn build_subscript(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let entity = world.spawn();

    world.set(entity, NodeKind::Subscript);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_node_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set(entity, Subscript);
    world.set(entity, Gettable);
    world.set_parent(entity, parent);

    // Parameters — subscripts inside types have a borrowing receiver
    let params = extract_params(world, node, entity, file_entity, file_id);
    let is_static = has_static_modifier(node);
    let parent_is_type = matches!(
        world.get::<NodeKind>(parent),
        Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol | NodeKind::Extension)
    );
    let receiver = if is_static || !parent_is_type {
        None
    } else {
        Some(ReceiverKind::Borrowing)
    };
    world.set(entity, Callable { params, receiver });

    // Return type
    if let Some(return_node) = find_child(node, SyntaxKind::ReturnType) {
        if let Some(ty) = return_node
            .children()
            .find(|c| is_type_kind(c.kind()))
            .and_then(|c| ast_type_from_cst(&c, file_id))
        {
            world.set(entity, TypeAnnotation(ty));
        }
    }

    // Check for getter/setter in SubscriptBody > PropertyAccessors
    if let Some(body) = find_child(node, SyntaxKind::SubscriptBody) {
        if let Some(acc) = find_child(&body, SyntaxKind::PropertyAccessors) {
            if find_child(&acc, SyntaxKind::SetterClause).is_some() {
                world.set(entity, Settable);
            }
            // Lower getter body if present
            if let Some(getter) = find_child(&acc, SyntaxKind::GetterClause) {
                if let Some(code_block) = find_child(&getter, SyntaxKind::CodeBlock) {
                    world.set(entity, Body(lower::lower_body(&code_block, file_id)));
                    world.set(entity, Valued(code_block));
                }
            }
        }
    }

    if has_static_modifier(node) {
        world.set(entity, Static);
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node);
    set_where_clause(world, entity, node, file_id);
    build_type_parameters(world, entity, node, file_entity, file_id);
}
