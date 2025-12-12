//! Statement resolution.
//!
//! This module handles resolving statements from syntax nodes into semantic
//! Statement representations, including variable declarations and expression statements.

use kestrel_semantic_tree::pattern::{Mutability, Pattern};
use kestrel_semantic_tree::stmt::Statement;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::syntax::get_node_span;

use super::context::BodyResolutionContext;
use super::expressions::resolve_expression;
use super::utils::{is_expression_kind, validate_not_standalone_type_param};

/// Resolve a statement syntax node
pub fn resolve_statement(
    stmt_node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<Statement> {
    // Statement wrapper - look inside for actual content
    for child in stmt_node.children() {
        match child.kind() {
            SyntaxKind::VariableDeclaration => {
                return resolve_variable_declaration(&child, ctx);
            }
            SyntaxKind::ExpressionStatement => {
                return resolve_expression_statement(&child, ctx);
            }
            SyntaxKind::Expression => {
                let expr = resolve_expression(&child, ctx);
                let span = get_node_span(&child, ctx.source);
                return Some(Statement::expr(expr, span));
            }
            _ => {}
        }
    }
    None
}

/// Resolve an expression statement (expression with semicolon)
pub fn resolve_expression_statement(
    stmt_node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<Statement> {
    // Find the expression child
    if let Some(expr_node) = stmt_node
        .children()
        .find(|c| c.kind() == SyntaxKind::Expression)
    {
        let expr = resolve_expression(&expr_node, ctx);
        let span = get_node_span(stmt_node, ctx.source);
        return Some(Statement::expr(expr, span));
    }

    // Also check for bare expression kinds
    for child in stmt_node.children() {
        if is_expression_kind(child.kind()) {
            let expr = resolve_expression(&child, ctx);
            let span = get_node_span(stmt_node, ctx.source);
            return Some(Statement::expr(expr, span));
        }
    }

    None
}

/// Resolve a variable declaration (let/var)
pub fn resolve_variable_declaration(
    decl_node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<Statement> {
    let span = get_node_span(decl_node, ctx.source);

    // Determine if let or var
    let is_mutable = decl_node
        .children_with_tokens()
        .any(|elem| elem.kind() == SyntaxKind::Var);
    let mutability = if is_mutable {
        Mutability::Mutable
    } else {
        Mutability::Immutable
    };

    // Extract name
    let name = extract_var_name(decl_node)?;

    // Extract type annotation (if any)
    let ty = extract_var_type(decl_node, ctx);

    // Extract initializer (if any)
    // Also validate that it's not a standalone type parameter reference
    let value = decl_node
        .children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
        .map(|expr_node| {
            let expr = resolve_expression(&expr_node, ctx);
            validate_not_standalone_type_param(expr, ctx)
        });

    // Determine the type from annotation or initializer
    let resolved_ty = ty.unwrap_or_else(|| {
        value
            .as_ref()
            .map(|e| e.ty.clone())
            .unwrap_or_else(|| Ty::type_var(span.clone()))
    });

    // Bind the local variable
    let name_span = get_name_span(decl_node, ctx.source).unwrap_or(span.clone());
    let local_id = ctx.local_scope.bind(
        name.clone(),
        resolved_ty.clone(),
        is_mutable,
        name_span.clone(),
    );

    // Create the pattern
    let pattern = Pattern::local(local_id, mutability, name, resolved_ty, name_span);

    Some(Statement::binding(pattern, value, span))
}

/// Extract the variable name from a VariableDeclaration node
fn extract_var_name(decl_node: &SyntaxNode) -> Option<String> {
    // Look for Name node
    if let Some(name_node) = decl_node.children().find(|c| c.kind() == SyntaxKind::Name) {
        // Get identifier token from Name
        return name_node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)
            .map(|t| t.text().to_string());
    }

    // Fallback: look for bare Identifier
    decl_node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string())
}

/// Get the span of the name in a variable declaration
fn get_name_span(decl_node: &SyntaxNode, source: &str) -> Option<Span> {
    if let Some(name_node) = decl_node.children().find(|c| c.kind() == SyntaxKind::Name) {
        return Some(get_node_span(&name_node, source));
    }
    None
}

/// Extract type annotation from a variable declaration
fn extract_var_type(decl_node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Option<Ty> {
    use crate::resolution::TypeResolver;

    // Look for Ty node
    decl_node
        .children()
        .find(|c| c.kind() == SyntaxKind::Ty)
        .map(|ty_node| {
            // Resolve the type using the database
            let mut resolver = TypeResolver::new(
                ctx.model,
                ctx.diagnostics,
                ctx.file_id,
                ctx.source,
                ctx.function_id,
            );
            resolver.resolve(&ty_node)
        })
}
