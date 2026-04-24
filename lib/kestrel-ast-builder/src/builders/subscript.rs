//! Subscript declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::utils::{find_child, get_decl_span};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use super::helpers::*;
use super::params::extract_params;
use super::type_param::build_type_parameters;
use crate::ast_type::ast_type_from_cst;
use crate::components::*;
use crate::lower;

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
    world.set(entity, DeclSpan(get_decl_span(node, file_id)));
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
            // SetterClause wraps a setter with a body; a bare `Set` token
            // appears for protocol requirements (`{ get set }`) without a body.
            let has_setter = find_child(&acc, SyntaxKind::SetterClause).is_some()
                || acc
                    .children_with_tokens()
                    .any(|e| e.as_token().is_some_and(|t| t.kind() == SyntaxKind::Set));
            if has_setter {
                world.set(entity, Settable);
            }
            // Lower getter body if present
            if let Some(getter) = find_child(&acc, SyntaxKind::GetterClause) {
                if let Some(code_block) = find_child(&getter, SyntaxKind::CodeBlock) {
                    world.set(entity, Body(lower::lower_body(&code_block, file_id)));
                    world.set(entity, Valued(code_block));
                }
            }
            // Setter accessor: spawn a child entity whose Callable is
            // `[index_params..., newValue]`. Receiver upgraded to Mutating
            // (setters mutate self's backing storage); None for static.
            if has_setter {
                if let Some(setter_clause) = find_child(&acc, SyntaxKind::SetterClause) {
                    if let Some(setter_body) = find_child(&setter_clause, SyntaxKind::CodeBlock) {
                        let new_value_ty = world.get::<TypeAnnotation>(entity).map(|t| t.0.clone());
                        let mut setter_params = world
                            .get::<Callable>(entity)
                            .map(|c| c.params.clone())
                            .unwrap_or_default();
                        setter_params.push(AstParam {
                            label: None,
                            name: "newValue".into(),
                            ty: new_value_ty,
                            default_entity: None,
                            pattern: None,
                            is_mut: false,
                            is_consuming: false,
                        });
                        let setter_receiver = if is_static || !parent_is_type {
                            None
                        } else {
                            Some(ReceiverKind::Mutating)
                        };
                        spawn_setter(
                            world,
                            entity,
                            &setter_clause,
                            &setter_body,
                            setter_params,
                            setter_receiver,
                            file_entity,
                            file_id,
                            is_static,
                        );
                    }
                }
            }
        } else if let Some(code_block) = find_child(&body, SyntaxKind::CodeBlock) {
            // Shorthand getter-only form: subscript(...) -> T { expr }
            world.set(entity, Body(lower::lower_body(&code_block, file_id)));
            world.set(entity, Valued(code_block));
        }
    }

    if has_static_modifier(node) {
        world.set(entity, Static);
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node, file_id);
    set_where_clause(world, entity, node, file_id);
    build_type_parameters(world, entity, node, file_entity, file_id);
}
