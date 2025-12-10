//! Core expression resolution.
//!
//! This module handles resolving expression syntax nodes into semantic Expression
//! representations. It dispatches to specialized modules for complex expressions
//! like calls, operators, and paths.

use kestrel_semantic_tree::expr::{ElseBranch, Expression, LabelInfo};
use kestrel_semantic_tree::stmt::Statement;
use kestrel_semantic_tree::ty::Ty;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use kestrel_reporting::IntoDiagnostic;

use crate::diagnostics::{
    BreakOutsideLoopError, ContinueOutsideLoopError, TupleIndexOnNonTupleError,
    TupleIndexOutOfBoundsError, UndeclaredLabelError,
};
use crate::syntax::get_node_span;

use super::calls::resolve_call_expression;
use super::context::BodyResolutionContext;
use super::operators::{resolve_binary_expression, resolve_postfix_expression, resolve_unary_expression};
use super::paths::resolve_path_expression;
use super::statements::resolve_statement;
use super::utils::{format_type, is_expression_kind, validate_not_standalone_type_param};

/// Resolve an expression syntax node into a semantic Expression
pub fn resolve_expression(
    expr_node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(expr_node, ctx.source);

    match expr_node.kind() {
        SyntaxKind::Expression => {
            // Wrapper node - resolve the inner expression
            for child in expr_node.children() {
                if is_expression_kind(child.kind()) {
                    return resolve_expression(&child, ctx);
                }
            }
            Expression::error(span)
        }

        SyntaxKind::ExprUnit => Expression::unit(span),

        SyntaxKind::ExprInteger => {
            let value = extract_integer_value(expr_node);
            Expression::integer(value, span)
        }

        SyntaxKind::ExprFloat => {
            let value = extract_float_value(expr_node);
            Expression::float(value, span)
        }

        SyntaxKind::ExprString => {
            let value = extract_string_value(expr_node);
            Expression::string(value, span)
        }

        SyntaxKind::ExprBool => {
            let value = extract_bool_value(expr_node);
            Expression::bool(value, span)
        }

        SyntaxKind::ExprArray => {
            resolve_array_expression(expr_node, ctx)
        }

        SyntaxKind::ExprTuple => {
            resolve_tuple_expression(expr_node, ctx)
        }

        SyntaxKind::ExprGrouping => {
            resolve_grouping_expression(expr_node, ctx)
        }

        SyntaxKind::ExprPath => {
            resolve_path_expression(expr_node, ctx)
        }

        SyntaxKind::ExprUnary => {
            resolve_unary_expression(expr_node, ctx)
        }

        SyntaxKind::ExprPostfix => {
            resolve_postfix_expression(expr_node, ctx)
        }

        SyntaxKind::ExprBinary => {
            resolve_binary_expression(expr_node, ctx)
        }

        SyntaxKind::ExprNull => {
            // TODO: Handle null properly with optional types
            Expression::error(span)
        }

        SyntaxKind::ExprCall => {
            resolve_call_expression(expr_node, ctx)
        }

        SyntaxKind::ExprAssignment => {
            resolve_assignment_expression(expr_node, ctx)
        }

        SyntaxKind::ExprIf => {
            resolve_if_expression(expr_node, ctx)
        }

        SyntaxKind::ExprWhile => {
            resolve_while_expression(expr_node, ctx)
        }

        SyntaxKind::ExprLoop => {
            resolve_loop_expression(expr_node, ctx)
        }

        SyntaxKind::ExprBreak => {
            resolve_break_expression(expr_node, ctx)
        }

        SyntaxKind::ExprContinue => {
            resolve_continue_expression(expr_node, ctx)
        }

        SyntaxKind::ExprReturn => {
            resolve_return_expression(expr_node, ctx)
        }

        SyntaxKind::ExprTupleIndex => {
            resolve_tuple_index_expression(expr_node, ctx)
        }

        _ => Expression::error(span),
    }
}

/// Extract integer value from an ExprInteger node
fn extract_integer_value(node: &SyntaxNode) -> i64 {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Integer)
        .and_then(|t| parse_integer_literal(t.text()))
        .unwrap_or(0)
}

/// Parse an integer literal (handles 0x, 0b, 0o prefixes)
fn parse_integer_literal(text: &str) -> Option<i64> {
    let text = text.replace('_', "");
    if text.starts_with("0x") || text.starts_with("0X") {
        i64::from_str_radix(&text[2..], 16).ok()
    } else if text.starts_with("0b") || text.starts_with("0B") {
        i64::from_str_radix(&text[2..], 2).ok()
    } else if text.starts_with("0o") || text.starts_with("0O") {
        i64::from_str_radix(&text[2..], 8).ok()
    } else {
        text.parse().ok()
    }
}

/// Extract float value from an ExprFloat node
fn extract_float_value(node: &SyntaxNode) -> f64 {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Float)
        .and_then(|t| t.text().replace('_', "").parse().ok())
        .unwrap_or(0.0)
}

/// Extract string value from an ExprString node (strips quotes)
fn extract_string_value(node: &SyntaxNode) -> String {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::String)
        .map(|t| {
            let text = t.text();
            // Strip surrounding quotes
            if text.len() >= 2 {
                text[1..text.len()-1].to_string()
            } else {
                text.to_string()
            }
        })
        .unwrap_or_default()
}

/// Extract boolean value from an ExprBool node
fn extract_bool_value(node: &SyntaxNode) -> bool {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Boolean)
        .map(|t| t.text() == "true")
        .unwrap_or(false)
}

/// Resolve an array expression: [1, 2, 3]
fn resolve_array_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.source);

    let elements: Vec<Expression> = node.children()
        .filter(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
        .map(|c| resolve_expression(&c, ctx))
        .collect();

    // Infer element type from first element, or use inferred if empty
    let element_ty = elements.first()
        .map(|e| e.ty.clone())
        .unwrap_or_else(|| Ty::inferred(span.clone()));

    Expression::array(elements, element_ty, span)
}

/// Resolve a tuple expression: (1, 2, 3)
fn resolve_tuple_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.source);

    let elements: Vec<Expression> = node.children()
        .filter(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
        .map(|c| resolve_expression(&c, ctx))
        .collect();

    Expression::tuple(elements, span)
}

/// Resolve a grouping expression: (expr)
fn resolve_grouping_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.source);

    // Find the inner expression
    if let Some(inner_node) = node.children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
    {
        let inner = resolve_expression(&inner_node, ctx);
        return Expression::grouping(inner, span);
    }

    Expression::error(span)
}

/// Resolve an assignment expression: target = value
fn resolve_assignment_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.source);

    // Find the LHS and RHS expressions
    // ExprAssignment contains: Expression, Equals token, Expression
    let mut expr_children = node.children()
        .filter(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()));

    let lhs_node = match expr_children.next() {
        Some(n) => n,
        None => return Expression::error(span),
    };

    let rhs_node = match expr_children.next() {
        Some(n) => n,
        None => return Expression::error(span),
    };

    // Resolve both sides
    let target = resolve_expression(&lhs_node, ctx);
    let value = resolve_expression(&rhs_node, ctx);

    // TODO: Validate that target is assignable (var, not let; field on mutable receiver)
    // TODO: Type check that value type is compatible with target type

    Expression::assignment(target, value, span)
}

/// Resolve an if expression: if condition { then } else { else }
fn resolve_if_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.source);

    // ExprIf structure:
    // - If token
    // - Expression (condition)
    // - CodeBlock (then branch)
    // - Optional: ElseClause
    //   - Else token
    //   - Either CodeBlock or Expression (for else-if)

    let mut children = node.children().peekable();

    // Find condition expression (first Expression child)
    let condition = children
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
        .map(|c| resolve_expression(&c, ctx))
        .unwrap_or_else(|| Expression::error(span.clone()));

    // Find then block (first CodeBlock child)
    let (then_statements, then_value) = children
        .find(|c| c.kind() == SyntaxKind::CodeBlock)
        .map(|c| resolve_if_block(&c, ctx))
        .unwrap_or_else(|| (vec![], None));

    // Find optional else clause
    let else_branch = node.children()
        .find(|c| c.kind() == SyntaxKind::ElseClause)
        .and_then(|else_clause| resolve_else_clause(&else_clause, ctx));

    Expression::if_expr(condition, then_statements, then_value, else_branch, span)
}

/// Resolve the body of an if/else block, returning statements and optional trailing expression.
/// This creates a new scope for the block.
fn resolve_if_block(
    block_node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> (Vec<Statement>, Option<Expression>) {
    // Push a new scope for this block
    ctx.local_scope.push_scope();

    let mut statements = Vec::new();
    let mut trailing_expr = None;

    let children: Vec<_> = block_node.children().collect();

    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;

        match child.kind() {
            SyntaxKind::Statement | SyntaxKind::ExpressionStatement => {
                if let Some(stmt) = resolve_statement(child, ctx) {
                    statements.push(stmt);
                }
            }
            SyntaxKind::VariableDeclaration => {
                if let Some(stmt) = super::statements::resolve_variable_declaration(child, ctx) {
                    statements.push(stmt);
                }
            }
            SyntaxKind::Expression => {
                // If last child and no semicolon, it's the trailing expression
                if is_last && !has_trailing_semicolon(child) {
                    trailing_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let stmt_span = get_node_span(child, ctx.source);
                    statements.push(Statement::expr(expr, stmt_span));
                }
            }
            _ if is_expression_kind(child.kind()) => {
                // Handle bare expression kinds (not wrapped in Expression)
                if is_last {
                    trailing_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let stmt_span = get_node_span(child, ctx.source);
                    statements.push(Statement::expr(expr, stmt_span));
                }
            }
            // Skip tokens like braces
            _ => {}
        }
    }

    // Pop the scope
    ctx.local_scope.pop_scope();

    (statements, trailing_expr)
}

/// Check if a node has a trailing semicolon
fn has_trailing_semicolon(node: &SyntaxNode) -> bool {
    node.children_with_tokens()
        .any(|elem| elem.kind() == SyntaxKind::Semicolon)
}

/// Resolve an else clause, which can be either a block or an else-if expression
fn resolve_else_clause(
    else_node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<ElseBranch> {
    // ElseClause contains either:
    // - A CodeBlock (else { ... })
    // - An Expression which is an ExprIf (else if ... { ... })

    // Check for else-if (Expression containing ExprIf)
    for child in else_node.children() {
        if child.kind() == SyntaxKind::Expression || child.kind() == SyntaxKind::ExprIf {
            // This is an else-if expression
            let if_expr = resolve_expression(&child, ctx);
            return Some(ElseBranch::ElseIf(Box::new(if_expr)));
        }
    }

    // Check for plain else block
    for child in else_node.children() {
        if child.kind() == SyntaxKind::CodeBlock {
            let (statements, value) = resolve_if_block(&child, ctx);
            return Some(ElseBranch::Block {
                statements,
                value: value.map(Box::new),
            });
        }
    }

    None
}

/// Resolve a while expression: label: while condition { body }
fn resolve_while_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.source);

    // Parse optional label
    let label_info = extract_loop_label(node);

    // Find condition expression
    let condition = node.children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
        .map(|c| resolve_expression(&c, ctx))
        .unwrap_or_else(|| Expression::error(span.clone()));

    // Enter the loop context with the label
    let label_name = label_info.as_ref().map(|l| l.name.clone());
    let label_span = label_info.as_ref().map(|l| l.span.clone());
    let loop_id = ctx.enter_loop(label_name, label_span);

    // Resolve the body
    let body = node.children()
        .find(|c| c.kind() == SyntaxKind::CodeBlock)
        .map(|c| resolve_loop_body(&c, ctx))
        .unwrap_or_default();

    // Exit the loop context
    ctx.exit_loop();

    Expression::while_loop(loop_id, label_info, condition, body, span)
}

/// Resolve a loop expression: label: loop { body }
fn resolve_loop_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.source);

    // Parse optional label
    let label_info = extract_loop_label(node);

    // Enter the loop context with the label
    let label_name = label_info.as_ref().map(|l| l.name.clone());
    let label_span = label_info.as_ref().map(|l| l.span.clone());
    let loop_id = ctx.enter_loop(label_name, label_span);

    // Resolve the body
    let body = node.children()
        .find(|c| c.kind() == SyntaxKind::CodeBlock)
        .map(|c| resolve_loop_body(&c, ctx))
        .unwrap_or_default();

    // Exit the loop context
    ctx.exit_loop();

    Expression::loop_expr(loop_id, label_info, body, span)
}

/// Resolve a break expression: break or break label
fn resolve_break_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.source);

    // Check if we're in a loop
    if !ctx.in_loop() {
        let error = BreakOutsideLoopError { span: span.clone() };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic(ctx.file_id));
        return Expression::error(span);
    }

    // Extract optional label
    let label_info = extract_break_continue_label(node);
    let label_name = label_info.as_ref().map(|l| l.name.as_str());

    // Find the target loop
    let loop_id = match ctx.find_loop(label_name) {
        Some(id) => id,
        None => {
            // Label not found
            if let Some(ref label) = label_info {
                let error = UndeclaredLabelError {
                    span: label.span.clone(),
                    label_name: label.name.clone(),
                };
                ctx.diagnostics.add_diagnostic(error.into_diagnostic(ctx.file_id));
            }
            return Expression::error(span);
        }
    };

    Expression::break_expr(loop_id, label_info, span)
}

/// Resolve a continue expression: continue or continue label
fn resolve_continue_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.source);

    // Check if we're in a loop
    if !ctx.in_loop() {
        let error = ContinueOutsideLoopError { span: span.clone() };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic(ctx.file_id));
        return Expression::error(span);
    }

    // Extract optional label
    let label_info = extract_break_continue_label(node);
    let label_name = label_info.as_ref().map(|l| l.name.as_str());

    // Find the target loop
    let loop_id = match ctx.find_loop(label_name) {
        Some(id) => id,
        None => {
            // Label not found
            if let Some(ref label) = label_info {
                let error = UndeclaredLabelError {
                    span: label.span.clone(),
                    label_name: label.name.clone(),
                };
                ctx.diagnostics.add_diagnostic(error.into_diagnostic(ctx.file_id));
            }
            return Expression::error(span);
        }
    };

    Expression::continue_expr(loop_id, label_info, span)
}

/// Resolve a return expression: return or return expr
fn resolve_return_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.source);

    // Find the optional value expression
    // The ExprReturn contains: Return keyword, optional Expression child
    // Also validate that it's not a standalone type parameter reference
    let value = node.children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
        .map(|expr_node| {
            let expr = resolve_expression(&expr_node, ctx);
            validate_not_standalone_type_param(expr, ctx)
        });

    Expression::return_expr(value, span)
}

/// Resolve the body of a loop, returning statements.
/// This creates a new scope for the loop body.
fn resolve_loop_body(
    block_node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Vec<Statement> {
    ctx.local_scope.push_scope();

    let mut statements = Vec::new();

    for child in block_node.children() {
        match child.kind() {
            SyntaxKind::Statement | SyntaxKind::ExpressionStatement => {
                if let Some(stmt) = resolve_statement(&child, ctx) {
                    statements.push(stmt);
                }
            }
            SyntaxKind::VariableDeclaration => {
                if let Some(stmt) = super::statements::resolve_variable_declaration(&child, ctx) {
                    statements.push(stmt);
                }
            }
            SyntaxKind::Expression => {
                // Expressions in loop body become expression statements
                let expr = resolve_expression(&child, ctx);
                let stmt_span = get_node_span(&child, ctx.source);
                statements.push(Statement::expr(expr, stmt_span));
            }
            _ if is_expression_kind(child.kind()) => {
                // Handle bare expression kinds
                let expr = resolve_expression(&child, ctx);
                let stmt_span = get_node_span(&child, ctx.source);
                statements.push(Statement::expr(expr, stmt_span));
            }
            _ => {}
        }
    }

    ctx.local_scope.pop_scope();
    statements
}

/// Extract label info from a loop expression (while/loop).
/// The label appears as a LoopLabel child before the loop keyword.
fn extract_loop_label(node: &SyntaxNode) -> Option<LabelInfo> {
    node.children()
        .find(|c| c.kind() == SyntaxKind::LoopLabel)
        .and_then(|label_node| {
            // The LoopLabel contains an Identifier token
            label_node.children_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| t.kind() == SyntaxKind::Identifier)
                .map(|token| {
                    let text_range = token.text_range();
                    let start = text_range.start().into();
                    let end = text_range.end().into();
                    LabelInfo {
                        name: token.text().to_string(),
                        span: start..end,
                    }
                })
        })
}

/// Extract label info from a break/continue expression.
/// The label appears as an Identifier token after the keyword.
fn extract_break_continue_label(node: &SyntaxNode) -> Option<LabelInfo> {
    // The ExprBreak/ExprContinue contains: keyword token, optional Identifier token
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|token| {
            let text_range = token.text_range();
            let start = text_range.start().into();
            let end = text_range.end().into();
            LabelInfo {
                name: token.text().to_string(),
                span: start..end,
            }
        })
}

/// Resolve a tuple index expression: tuple.0, tuple.1
fn resolve_tuple_index_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.source);

    // Find the base expression (first Expression child)
    let base_node = match node
        .children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
    {
        Some(n) => n,
        None => return Expression::error(span),
    };

    // Resolve the base expression
    let base = resolve_expression(&base_node, ctx);

    // Extract the index from the Integer token
    let index_token = node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Integer);

    let (index, index_span) = match index_token {
        Some(token) => {
            let text_range = token.text_range();
            let idx_span = text_range.start().into()..text_range.end().into();
            let index_value = token.text().parse::<usize>().unwrap_or(0);
            (index_value, idx_span)
        }
        None => return Expression::error(span),
    };

    // Check if the base type is a tuple
    let base_ty = &base.ty;
    match base_ty.as_tuple() {
        Some(elements) => {
            // Check bounds
            if index >= elements.len() {
                let error = TupleIndexOutOfBoundsError {
                    index_span,
                    index,
                    tuple_length: elements.len(),
                    tuple_type: format_type(base_ty),
                };
                ctx.diagnostics
                    .add_diagnostic(error.into_diagnostic(ctx.file_id));
                return Expression::error(span);
            }

            // Get the element type at the index
            let element_ty = elements[index].clone();
            Expression::tuple_index(base, index, element_ty, span)
        }
        None => {
            // Not a tuple type
            let error = TupleIndexOnNonTupleError {
                span: span.clone(),
                index,
                base_type: format_type(base_ty),
            };
            ctx.diagnostics
                .add_diagnostic(error.into_diagnostic(ctx.file_id));
            Expression::error(span)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_integer_literal() {
        assert_eq!(parse_integer_literal("42"), Some(42));
        assert_eq!(parse_integer_literal("0xFF"), Some(255));
        assert_eq!(parse_integer_literal("0b1010"), Some(10));
        assert_eq!(parse_integer_literal("0o17"), Some(15));
        assert_eq!(parse_integer_literal("1_000_000"), Some(1000000));
    }
}
