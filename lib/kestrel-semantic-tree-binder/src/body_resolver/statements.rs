//! Statement resolution.
//!
//! This module handles resolving statements from syntax nodes into semantic
//! Statement representations, including variable declarations and expression statements.

use super::context::resolve_code_block;
use kestrel_semantic_tree::behavior::executable::CodeBlock;
use kestrel_semantic_tree::expr::ExprKind;
use kestrel_semantic_tree::pattern::{Mutability, Pattern};
use kestrel_semantic_tree::stmt::Statement;
use kestrel_semantic_tree::ty::Ty;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use kestrel_syntax_tree::utils::get_node_span;

use super::context::BodyResolutionContext;
use super::expressions::resolve_expression;
use super::patterns::{is_pattern_kind, resolve_pattern_with_mutability};
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
            },
            SyntaxKind::ExpressionStatement => {
                return resolve_expression_statement(&child, ctx);
            },
            SyntaxKind::GuardLetStatement => {
                return resolve_guard_let_statement(&child, ctx);
            },
            SyntaxKind::DeinitStatement => {
                return resolve_deinit_statement(&child, ctx);
            },
            SyntaxKind::Expression => {
                let expr = resolve_expression(&child, ctx);
                let span = get_node_span(&child, ctx.file_id);
                return Some(Statement::expr(expr, span));
            },
            _ => {},
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
        let span = get_node_span(stmt_node, ctx.file_id);
        return Some(Statement::expr(expr, span));
    }

    // Also check for bare expression kinds
    for child in stmt_node.children() {
        if is_expression_kind(child.kind()) {
            let expr = resolve_expression(&child, ctx);
            let span = get_node_span(stmt_node, ctx.file_id);
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
    let span = get_node_span(decl_node, ctx.file_id);

    // Determine if let or var (affects mutability of all bindings in pattern)
    let is_mutable = decl_node
        .children_with_tokens()
        .any(|elem| elem.kind() == SyntaxKind::Var);

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

    // Track moves for non-copyable types
    // If the value is a LocalRef to a non-copyable type, mark the source as moved
    if let Some(ref value_expr) = value
        && let ExprKind::LocalRef(source_local_id) = &value_expr.kind
    {
        // Use context-aware check that considers `T: not Copyable` bounds
        if !value_expr.ty.is_copyable_in_context(ctx.where_clause()) {
            ctx.move_tracker
                .mark_moved(*source_local_id, value_expr.span.clone());
        }
    }

    // Determine the expected type from annotation or initializer
    let expected_ty = ty.or_else(|| value.as_ref().map(|e| e.ty.clone()));

    // Find and resolve the pattern
    // Look for Pattern node first (new syntax), then fall back to Name (old syntax) or BindingPattern
    let pattern = if let Some(pattern_node) = decl_node
        .children()
        .find(|c| c.kind() == SyntaxKind::Pattern || is_pattern_kind(c.kind()))
    {
        // Pass the mutability from the statement level (var vs let)
        resolve_pattern_with_mutability(&pattern_node, ctx, expected_ty.as_ref(), is_mutable)
    } else if let Some(name_node) = decl_node.children().find(|c| c.kind() == SyntaxKind::Name) {
        // Old syntax: Name node with identifier
        let name = name_node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)
            .map(|t| t.text().to_string())?;

        let name_span = get_node_span(&name_node, ctx.file_id);
        let mutability = if is_mutable {
            Mutability::Mutable
        } else {
            Mutability::Immutable
        };

        let resolved_ty = expected_ty.unwrap_or_else(|| Ty::infer(span.clone()));
        let local_id = ctx.local_scope.bind(
            name.clone(),
            resolved_ty.clone(),
            is_mutable,
            name_span.clone(),
        );

        Pattern::local(local_id, mutability, name, resolved_ty, name_span)
    } else {
        // No pattern found - return error pattern
        Pattern::error(span.clone())
    };

    Some(Statement::binding(pattern, value, span))
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
                ctx.source,
                ctx.file_id,
                ctx.function_id,
            );
            resolver.resolve(&ty_node)
        })
}

/// Resolve a guard-let statement with chain support:
/// - Single: `guard let pattern = expr else { block }`
/// - Chain: `guard let .Some(x) = a, let .Some(y) = b, x > 0 else { block }`
///
/// Guard-let has special scoping rules:
/// - Pattern bindings are NOT visible in the else block
/// - Pattern bindings ARE visible after the guard statement (in subsequent statements)
/// - In chains, pattern bindings from earlier conditions are visible in later conditions
pub fn resolve_guard_let_statement(
    guard_node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<Statement> {
    use kestrel_semantic_tree::expr::IfCondition;

    let span = get_node_span(guard_node, ctx.file_id);

    // Resolve else block BEFORE the pattern bindings, so pattern bindings are not visible
    let else_block = guard_node
        .children()
        .find(|c| c.kind() == SyntaxKind::CodeBlock)
        .map(|block_node| {
            // Resolve the else block before adding pattern bindings
            // Pattern bindings from THIS guard are not visible, but earlier bindings are
            resolve_code_block(&block_node, ctx)
        })
        .unwrap_or_else(CodeBlock::empty);

    // Collect all conditions (GuardLetCondition or Expression children before CodeBlock)
    let mut conditions: Vec<IfCondition> = Vec::new();
    for child in guard_node.children() {
        if child.kind() == SyntaxKind::CodeBlock {
            break;
        }
        if child.kind() == SyntaxKind::Expression || is_expression_kind(child.kind()) {
            // Boolean condition
            let cond_expr = resolve_expression(&child, ctx);
            conditions.push(IfCondition::Expr(cond_expr));
        } else if child.kind() == SyntaxKind::GuardLetCondition {
            // Guard-let condition: let pattern = expr
            let cond = resolve_guard_let_condition(&child, ctx);
            conditions.push(cond);
        }
    }

    // Ensure we have at least one condition
    if conditions.is_empty() {
        conditions.push(IfCondition::Expr(
            kestrel_semantic_tree::expr::Expression::error(span.clone()),
        ));
    }

    Some(Statement::guard_let(conditions, else_block, span))
}

/// Resolve a single guard-let condition: let pattern = expr
fn resolve_guard_let_condition(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> kestrel_semantic_tree::expr::IfCondition {
    use kestrel_semantic_tree::expr::IfCondition;

    let span = get_node_span(node, ctx.file_id);

    // Find the value expression (the scrutinee)
    let value_node = node
        .children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()));

    let value = value_node
        .map(|n| resolve_expression(&n, ctx))
        .unwrap_or_else(|| kestrel_semantic_tree::expr::Expression::error(span.clone()));

    // Find and resolve the pattern
    // Pattern bindings will be added to the current scope for subsequent conditions
    let pattern_node = node.children().find(|c| is_pattern_kind(c.kind()));

    let pattern = pattern_node
        .map(|n| resolve_pattern_with_mutability(&n, ctx, Some(&value.ty), false))
        .unwrap_or_else(|| Pattern::error(span.clone()));

    IfCondition::Let {
        pattern,
        value,
        span,
    }
}

/// Resolve a deinit statement: `deinit identifier;`
///
/// This statement explicitly runs the destructor for a variable and marks it as moved.
/// The variable cannot be used after this point.
pub fn resolve_deinit_statement(
    deinit_node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<Statement> {
    use crate::diagnostics::{DeinitAlreadyMovedError, DeinitUndeclaredError};
    use kestrel_reporting::IntoDiagnostic;

    let span = get_node_span(deinit_node, ctx.file_id);

    // Extract the identifier name from the DeinitStatement node
    let name = deinit_node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string())?;

    // Look up the variable in the local scope
    let local_id = match ctx.local_scope.lookup(&name) {
        Some(id) => id,
        None => {
            // Variable not found
            let error = DeinitUndeclaredError {
                span: span.clone(),
                name: name.clone(),
            };
            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            return None;
        },
    };

    // Check if the variable has already been moved
    if let Some(moved_at) = ctx.move_tracker.get_move_span(local_id) {
        let error = DeinitAlreadyMovedError {
            span: span.clone(),
            name: name.clone(),
            moved_at,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return None;
    }

    // Also check for maybe-moved
    if let Some(moved_at) = ctx.move_tracker.get_maybe_move_span(local_id) {
        let error = DeinitAlreadyMovedError {
            span: span.clone(),
            name: name.clone(),
            moved_at,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return None;
    }

    // Mark the variable as moved (consumed by deinit)
    ctx.move_tracker.mark_moved(local_id, span.clone());

    Some(Statement::deinit(local_id, name, span))
}
