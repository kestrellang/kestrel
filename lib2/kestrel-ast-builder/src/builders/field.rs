//! Field declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};
use kestrel_syntax_tree2::utils::{extract_name, find_child, get_node_span};

use crate::ast_type::ast_type_from_cst;
use crate::components::*;
use crate::lower;
use super::helpers::*;

/// Build a field declaration entity from CST.
///
/// Components: NodeKind::Field, Name, FileId, Vis, TypeAnnotation,
/// Gettable, [Settable], [Valued (init expr)], [Static],
/// [Attributes], [Documentation]
///
/// Fields declared with `var` are Settable. Fields with `let` are read-only.
/// Computed properties (with get/set accessors) set Gettable/Settable
/// based on which accessors are present.
pub fn build_field(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let entity = world.spawn();

    world.set(entity, NodeKind::Field);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_node_span(node, file_id)));
    world.set(entity, CstNode(node.clone()));
    world.set_parent(entity, parent);

    if let Some(name) = extract_name(node) {
        world.set(entity, Name(name));
    }

    // Type annotation
    if let Some(ty) = node
        .children()
        .find(|c| is_type_kind(c.kind()))
        .and_then(|c| ast_type_from_cst(&c, file_id))
    {
        world.set(entity, TypeAnnotation(ty));
    }

    // Determine mutability from var/let keyword
    let is_var = node
        .children_with_tokens()
        .any(|e| e.as_token().is_some_and(|t| t.kind() == SyntaxKind::Var));

    // Check for computed property accessors
    let has_accessors = find_child(node, SyntaxKind::PropertyAccessors).is_some();

    if has_accessors {
        // Computed property: Gettable/Settable based on get/set clauses
        let accessors = find_child(node, SyntaxKind::PropertyAccessors).unwrap();

        let has_getter = find_child(&accessors, SyntaxKind::GetterClause).is_some();
        let has_setter = find_child(&accessors, SyntaxKind::SetterClause).is_some();

        if has_getter {
            world.set(entity, Gettable);
        }
        if has_setter {
            world.set(entity, Settable);
        }

        // Store getter body as Valued + Body if present
        if let Some(getter) = find_child(&accessors, SyntaxKind::GetterClause) {
            if let Some(body) = find_child(&getter, SyntaxKind::CodeBlock) {
                world.set(entity, Body(lower::lower_body(&body, file_id)));
                world.set(entity, Valued(body));
            }
        }
    } else {
        // Stored property: always Gettable
        world.set(entity, Gettable);
        if is_var {
            world.set(entity, Settable);
        }

        // Default value — field initializers are emitted as `= Expression`
        // directly under FieldDeclaration (NOT wrapped in DefaultValue).
        // Find the first Expression child after an Equals token.
        let mut found_equals = false;
        for child in node.children_with_tokens() {
            if child.as_token().is_some_and(|t| t.kind() == SyntaxKind::Equals) {
                found_equals = true;
            } else if found_equals {
                if let Some(expr_node) = child.into_node() {
                    // The parser wraps initializer exprs in Expression nodes
                    world.set(entity, Body(lower::lower_default_value_expr(&expr_node, file_id)));
                    world.set(entity, Valued(expr_node));
                    break;
                }
            }
        }
    }

    if has_static_modifier(node) {
        world.set(entity, Static);
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node);
    set_documentation(world, entity, node);
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
