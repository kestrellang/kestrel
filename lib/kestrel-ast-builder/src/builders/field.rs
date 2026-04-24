//! Field declaration builder.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree2::utils::{extract_name, find_child, get_decl_span};
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};

use super::helpers::*;
use crate::ast_type::ast_type_from_cst;
use crate::components::*;
use crate::lower;

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
    world.set(entity, DeclSpan(get_decl_span(node, file_id)));
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

    // Determine mutability from var/let keyword and capture it as a component
    // so downstream analyzers don't need to re-scan tokens.
    let is_var = node
        .children_with_tokens()
        .any(|e| e.as_token().is_some_and(|t| t.kind() == SyntaxKind::Var));
    world.set(
        entity,
        if is_var {
            FieldMutability::Var
        } else {
            FieldMutability::Let
        },
    );

    // Check for computed property accessors
    let has_accessors = find_child(node, SyntaxKind::PropertyAccessors).is_some();
    if has_accessors {
        world.set(entity, Computed);
    }

    if has_accessors {
        // Computed property: Gettable/Settable based on get/set clauses
        let accessors = find_child(node, SyntaxKind::PropertyAccessors).unwrap();

        // Clauses wrap an accessor body; bare `Get`/`Set` tokens appear as
        // direct children for protocol requirements (`{ get set }`) where
        // accessors are declared without bodies.
        let has_getter = find_child(&accessors, SyntaxKind::GetterClause).is_some()
            || accessors
                .children_with_tokens()
                .any(|e| e.as_token().is_some_and(|t| t.kind() == SyntaxKind::Get));
        let has_setter = find_child(&accessors, SyntaxKind::SetterClause).is_some()
            || accessors
                .children_with_tokens()
                .any(|e| e.as_token().is_some_and(|t| t.kind() == SyntaxKind::Set));

        if has_getter {
            world.set(entity, Gettable);
        }
        if has_setter {
            world.set(entity, Settable);
        }

        // Store getter body as Valued + Body if present.
        // Instance computed properties access `self` via a borrowing receiver;
        // `static` fields and module-level computed globals have no receiver
        // (the latter have no parent type to bind `self` to).
        let is_static_field = has_static_modifier(node);
        let parent_is_type = matches!(
            world.get::<NodeKind>(parent),
            Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol | NodeKind::Extension)
        );
        let receiver = if is_static_field || !parent_is_type {
            None
        } else {
            Some(ReceiverKind::Borrowing)
        };

        if let Some(getter) = find_child(&accessors, SyntaxKind::GetterClause) {
            if let Some(body) = find_child(&getter, SyntaxKind::CodeBlock) {
                world.set(entity, Body(lower::lower_body(&body, file_id)));
                world.set(entity, Valued(body));
                world.set(
                    entity,
                    Callable {
                        params: Vec::new(),
                        receiver: receiver.clone(),
                    },
                );
            }
        } else if let Some(body) = find_child(&accessors, SyntaxKind::CodeBlock) {
            // Shorthand computed property: `var foo: Type { expr }`
            // The parser emits PropertyAccessors > CodeBlock without GetterClause.
            // Treat as an implicit getter.
            world.set(entity, Gettable);
            world.set(entity, Body(lower::lower_body(&body, file_id)));
            world.set(entity, Valued(body));
            world.set(
                entity,
                Callable {
                    params: Vec::new(),
                    receiver: receiver.clone(),
                },
            );
        }

        // Setter accessor: spawn a child entity with its own Callable + Body.
        // `newValue` is an implicit parameter typed as the field's type.
        // Instance setters are Mutating (they write self's backing storage);
        // static/global setters have no receiver.
        if has_setter {
            if let Some(setter_clause) = find_child(&accessors, SyntaxKind::SetterClause) {
                if let Some(setter_body) = find_child(&setter_clause, SyntaxKind::CodeBlock) {
                    let new_value_ty = world.get::<TypeAnnotation>(entity).map(|t| t.0.clone());
                    let setter_receiver = if is_static_field || !parent_is_type {
                        None
                    } else {
                        Some(ReceiverKind::Mutating)
                    };
                    let params = vec![AstParam {
                        label: None,
                        name: "newValue".into(),
                        ty: new_value_ty,
                        default_entity: None,
                        pattern: None,
                        is_mut: false,
                        is_consuming: false,
                    }];
                    spawn_setter(
                        world,
                        entity,
                        &setter_clause,
                        &setter_body,
                        params,
                        setter_receiver,
                        file_entity,
                        file_id,
                        is_static_field,
                    );
                }
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
            if child
                .as_token()
                .is_some_and(|t| t.kind() == SyntaxKind::Equals)
            {
                found_equals = true;
            } else if found_equals {
                if let Some(expr_node) = child.into_node() {
                    // The parser wraps initializer exprs in Expression nodes
                    world.set(
                        entity,
                        Body(lower::lower_default_value_expr(&expr_node, file_id)),
                    );
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
    set_attributes(world, entity, node, file_id);
    set_documentation(world, entity, node);
}
