//! Function, initializer, and deinit declaration builders.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::utils::{extract_name, find_child, get_decl_span};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use super::helpers::*;
use super::params::extract_params;
use super::type_param::build_type_parameters;
use crate::ast_type::ast_type_from_cst;
use crate::components::*;
use crate::lower;

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
    world.set(entity, DeclSpan(get_decl_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set_parent(entity, parent);

    if let Some(name) = extract_name(node) {
        world.set(entity, Name(name));
    }

    // Determine receiver: non-static functions inside type declarations are methods.
    // Explicit keyword (mutating/consuming) overrides, otherwise defaults to Borrowing.
    let is_static = has_static_modifier(node);
    let parent_is_type = matches!(
        world.get::<NodeKind>(parent),
        Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol | NodeKind::Extension)
    );
    let receiver = if is_static || !parent_is_type {
        None
    } else {
        Some(extract_receiver_kind(node))
    };

    // Parameters (creates child entities for default value expressions)
    let params = extract_params(world, node, entity, file_entity, file_id);
    world.set(entity, Callable { params, receiver });

    // Return type from ReturnType child
    if let Some(return_node) = find_child(node, SyntaxKind::ReturnType)
        && let Some(ty) = return_node
            .children()
            .find(|c| is_type_kind(c.kind()))
            .and_then(|c| ast_type_from_cst(&c, file_id))
        {
            world.set(entity, TypeAnnotation(ty));
        }

    // Body — CST wraps it in FunctionBody > CodeBlock (block body)
    // or FunctionBody > Expression (expression body: `= expr`)
    if let Some(fn_body) = find_child(node, SyntaxKind::FunctionBody) {
        if let Some(code_block) = find_child(&fn_body, SyntaxKind::CodeBlock) {
            world.set(entity, Body(lower::lower_body(&code_block, file_id)));
            world.set(entity, Valued(code_block));
        } else {
            // Expression body: `func foo() -> T = expr`
            world.set(entity, Body(lower::lower_default_value(&fn_body, file_id)));
            world.set(entity, Valued(fn_body));
        }
    }

    if has_static_modifier(node) {
        world.set(entity, Static);
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node, file_id);
    set_documentation(world, entity, node);
    set_where_clause(world, entity, node, file_id);
    build_type_parameters(world, entity, node, file_entity, file_id);
}

/// Build an initializer declaration entity from CST.
///
/// Components: NodeKind::Initializer, FileId, Vis, Callable,
/// [InitEffect], [TypeAnnotation (return)],
/// [Valued (body)], [TypeParams], [WhereClause], [Attributes], [Documentation]
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
    world.set(entity, DeclSpan(get_decl_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set_parent(entity, parent);

    let params = extract_params(world, node, entity, file_entity, file_id);
    // Inits always have a `self` receiver (mutating — they're building the instance)
    world.set(
        entity,
        Callable {
            params,
            receiver: Some(ReceiverKind::Mutating),
        },
    );

    // Init effect: ? (failable) or throws E (throwing).
    // Sets TypeAnnotation to ()? or () throws E so the body return type is correct.
    if let Some(effect_node) = find_child(node, SyntaxKind::InitEffect) {
        let effect_span = get_decl_span(&effect_node, file_id);
        let unit_ty = kestrel_ast::AstType::Unit(effect_span.clone());

        let has_question = effect_node.children_with_tokens().any(|c| {
            c.as_token()
                .map(|t| t.kind() == SyntaxKind::Question)
                .unwrap_or(false)
        });

        if has_question {
            world.set(entity, InitEffect::Failable);
            world.set(
                entity,
                TypeAnnotation(kestrel_ast::AstType::Optional(
                    Box::new(unit_ty),
                    effect_span,
                )),
            );
        } else if let Some(err_ty) = effect_node
            .children()
            .find(|c| is_type_kind(c.kind()))
            .and_then(|c| ast_type_from_cst(&c, file_id))
        {
            world.set(entity, InitEffect::Throwing);
            world.set(
                entity,
                TypeAnnotation(kestrel_ast::AstType::Result {
                    ok: Box::new(unit_ty),
                    err: Box::new(err_ty),
                    span: effect_span,
                }),
            );
        }
    }

    // Body — CST wraps it in FunctionBody > CodeBlock
    if let Some(fn_body) = find_child(node, SyntaxKind::FunctionBody)
        && let Some(code_block) = find_child(&fn_body, SyntaxKind::CodeBlock) {
            world.set(entity, Body(lower::lower_body(&code_block, file_id)));
            world.set(entity, Valued(code_block));
        }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node, file_id);
    set_documentation(world, entity, node);
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
    world.set(entity, DeclSpan(get_decl_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set_parent(entity, parent);

    // Deinits always have a `self` receiver (consuming — they're destroying the instance)
    world.set(
        entity,
        Callable {
            params: Vec::new(),
            receiver: Some(ReceiverKind::Consuming),
        },
    );

    // Body — CST wraps it in FunctionBody > CodeBlock
    if let Some(fn_body) = find_child(node, SyntaxKind::FunctionBody)
        && let Some(code_block) = find_child(&fn_body, SyntaxKind::CodeBlock) {
            world.set(entity, Body(lower::lower_body(&code_block, file_id)));
            world.set(entity, Valued(code_block));
        }
}

/// Extract receiver kind from function modifier keywords.
/// Defaults to Borrowing if no explicit keyword.
fn extract_receiver_kind(node: &SyntaxNode) -> ReceiverKind {
    for elem in node.children_with_tokens() {
        if let Some(token) = elem.as_token() {
            match token.kind() {
                SyntaxKind::Mutating => return ReceiverKind::Mutating,
                SyntaxKind::Consuming => return ReceiverKind::Consuming,
                _ => {},
            }
        }
    }
    ReceiverKind::Borrowing
}
