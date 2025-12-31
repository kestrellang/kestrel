//! Core expression resolution.
//!
//! This module handles resolving expression syntax nodes into semantic Expression
//! representations. It dispatches to specialized modules for complex expressions
//! like calls, operators, and paths.

use kestrel_reporting::IntoDiagnostic;
use kestrel_semantic_tree::expr::{ElseBranch, Expression, IfCondition, LabelInfo};
use kestrel_semantic_tree::stmt::Statement;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::diagnostics::{
    BreakOutsideLoopError, ContinueOutsideLoopError, TupleIndexOnNonTupleError,
    TupleIndexOutOfBoundsError, UndeclaredLabelError,
};
use kestrel_syntax_tree::utils::get_node_span;

use super::calls::{resolve_argument_list, resolve_call_expression};
use super::context::BodyResolutionContext;
use super::operators::{
    resolve_binary_expression, resolve_postfix_expression, resolve_unary_expression,
};
use super::paths::resolve_path_expression;
use super::statements::resolve_statement;
use super::utils::{is_expression_kind, validate_not_standalone_type_param};

/// Resolve an expression syntax node into a semantic Expression
pub fn resolve_expression(expr_node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(expr_node, ctx.file_id);

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

        SyntaxKind::ExprArray => resolve_array_expression(expr_node, ctx),

        SyntaxKind::ExprTuple => resolve_tuple_expression(expr_node, ctx),

        SyntaxKind::ExprGrouping => resolve_grouping_expression(expr_node, ctx),

        SyntaxKind::ExprPath => resolve_path_expression(expr_node, ctx),

        SyntaxKind::ExprUnary => resolve_unary_expression(expr_node, ctx),

        SyntaxKind::ExprPostfix => resolve_postfix_expression(expr_node, ctx),

        SyntaxKind::ExprBinary => resolve_binary_expression(expr_node, ctx),

        SyntaxKind::ExprNull => {
            // TODO: Handle null properly with optional types
            Expression::error(span)
        }

        SyntaxKind::ExprCall => resolve_call_expression(expr_node, ctx),

        SyntaxKind::ExprAssignment => resolve_assignment_expression(expr_node, ctx),

        SyntaxKind::ExprIf => resolve_if_expression(expr_node, ctx),

        SyntaxKind::ExprWhile => resolve_while_expression(expr_node, ctx),

        SyntaxKind::ExprLoop => resolve_loop_expression(expr_node, ctx),

        SyntaxKind::ExprBreak => resolve_break_expression(expr_node, ctx),

        SyntaxKind::ExprContinue => resolve_continue_expression(expr_node, ctx),

        SyntaxKind::ExprReturn => resolve_return_expression(expr_node, ctx),

        SyntaxKind::ExprTupleIndex => resolve_tuple_index_expression(expr_node, ctx),

        SyntaxKind::ExprClosure => resolve_closure_expression(expr_node, ctx),

        SyntaxKind::ExprImplicitMemberAccess => resolve_implicit_member_access(expr_node, ctx),

        SyntaxKind::ExprMatch => resolve_match_expression(expr_node, ctx),

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
                text[1..text.len() - 1].to_string()
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
fn resolve_array_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    let elements: Vec<Expression> = node
        .children()
        .filter(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
        .map(|c| resolve_expression(&c, ctx))
        .collect();

    // Infer element type from first element, or use infer if empty
    let element_ty = elements
        .first()
        .map(|e| e.ty.clone())
        .unwrap_or_else(|| Ty::infer(span.clone()));

    Expression::array(elements, element_ty, span)
}

/// Resolve a tuple expression: (1, 2, 3)
fn resolve_tuple_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    let elements: Vec<Expression> = node
        .children()
        .filter(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
        .map(|c| resolve_expression(&c, ctx))
        .collect();

    Expression::tuple(elements, span)
}

/// Resolve a grouping expression: (expr)
fn resolve_grouping_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Find the inner expression
    if let Some(inner_node) = node
        .children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
    {
        let inner = resolve_expression(&inner_node, ctx);
        return Expression::grouping(inner, span);
    }

    Expression::error(span)
}

/// Resolve an assignment expression: target = value
fn resolve_assignment_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Find the LHS and RHS expressions
    // ExprAssignment contains: Expression, Equals token, Expression
    let mut expr_children = node
        .children()
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
/// Also handles if-let: if let pattern = expr { then } else { else }
/// And if-let chains: if let .Some(x) = a, let .Some(y) = b { ... }
fn resolve_if_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // ExprIf structure:
    // - If token
    // - One or more conditions:
    //   - Expression (bool condition), or
    //   - IfLetCondition (let pattern = expr)
    // - CodeBlock (then branch)
    // - Optional: ElseClause
    //   - Else token
    //   - Either CodeBlock or Expression (for else-if)

    // Snapshot move state before the if (for branching)
    let pre_if_moves = ctx.move_tracker.snapshot();

    // Push a new scope for pattern bindings from if-let conditions.
    // Bindings in if-let are visible in subsequent conditions and the then branch,
    // but NOT in the else branch.
    ctx.local_scope.push_scope();

    // Collect all conditions (Expression or IfLetCondition children before CodeBlock)
    let mut conditions: Vec<IfCondition> = Vec::new();
    for child in node.children() {
        if child.kind() == SyntaxKind::CodeBlock {
            break;
        }
        if child.kind() == SyntaxKind::Expression || is_expression_kind(child.kind()) {
            // Boolean condition
            let cond_expr = resolve_expression(&child, ctx);
            conditions.push(IfCondition::Expr(cond_expr));
        } else if child.kind() == SyntaxKind::IfLetCondition {
            // If-let condition: let pattern = expr
            let cond = resolve_if_let_condition(&child, ctx);
            conditions.push(cond);
        }
    }

    // Ensure we have at least one condition
    if conditions.is_empty() {
        conditions.push(IfCondition::Expr(Expression::error(span.clone())));
    }

    // Find then block (first CodeBlock child)
    // The then block is resolved with the pattern bindings in scope
    let (then_statements, then_value) = node
        .children()
        .find(|c| c.kind() == SyntaxKind::CodeBlock)
        .map(|c| resolve_if_then_block(&c, ctx))
        .unwrap_or_else(|| (vec![], None));

    // Snapshot move state after then branch
    let after_then_moves = ctx.move_tracker.snapshot();

    // Pop the scope before resolving the else branch
    // Pattern bindings are NOT visible in the else branch
    ctx.local_scope.pop_scope();

    // Restore move state before resolving else branch
    ctx.move_tracker.restore(pre_if_moves.clone());

    // Find optional else clause (resolved without the if-let bindings)
    let else_clause_node = node
        .children()
        .find(|c| c.kind() == SyntaxKind::ElseClause);

    let else_branch = else_clause_node
        .as_ref()
        .and_then(|else_clause| resolve_else_clause(else_clause, ctx));

    // Merge move states from both branches
    if else_branch.is_some() {
        // Both branches exist - merge then and else states
        // After resolving else, ctx.move_tracker has the else state
        let after_else_moves = ctx.move_tracker.snapshot();
        ctx.move_tracker.restore(after_then_moves);
        ctx.move_tracker.merge(&after_else_moves);
    } else {
        // No else branch - the if might not execute, so merge then with pre-if
        // (moved in then but not else = maybe-moved)
        ctx.move_tracker.restore(after_then_moves);
        ctx.move_tracker.merge(&pre_if_moves);
    }

    Expression::if_expr_with_conditions(conditions, then_statements, then_value, else_branch, span)
}

/// Resolve a single if-let condition: let pattern = expr
fn resolve_if_let_condition(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> IfCondition {
    use super::patterns::resolve_pattern;
    use kestrel_semantic_tree::pattern::Pattern;

    let span = get_node_span(node, ctx.file_id);

    // Find the pattern
    let pattern_node = node
        .children()
        .find(|c| super::patterns::is_pattern_kind(c.kind()));

    // Find the value expression (the scrutinee)
    let value_node = node
        .children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()));

    let value = value_node
        .map(|n| resolve_expression(&n, ctx))
        .unwrap_or_else(|| Expression::error(span.clone()));

    // Resolve the pattern with the value type as expected type
    let pattern = pattern_node
        .map(|n| resolve_pattern(&n, ctx, Some(&value.ty)))
        .unwrap_or_else(|| Pattern::error(span.clone()));

    IfCondition::Let {
        pattern,
        value,
        span,
    }
}

/// Resolve the then-block of an if expression (without pushing a new scope).
/// The pattern bindings from if-let conditions are already in scope.
fn resolve_if_then_block(
    block_node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> (Vec<Statement>, Option<Expression>) {
    // Note: We do NOT push a new scope here because the if-let bindings
    // need to be visible. The scope was already pushed in resolve_if_expression.
    // However, we do want local variables declared in the then block to be scoped,
    // so we push a nested scope.
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
                    let stmt_span = get_node_span(child, ctx.file_id);
                    statements.push(Statement::expr(expr, stmt_span));
                }
            }
            _ if is_expression_kind(child.kind()) => {
                // Handle bare expression kinds (not wrapped in Expression)
                if is_last {
                    trailing_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let stmt_span = get_node_span(child, ctx.file_id);
                    statements.push(Statement::expr(expr, stmt_span));
                }
            }
            // Skip tokens like braces
            _ => {}
        }
    }

    // Pop the block scope
    ctx.local_scope.pop_scope();

    (statements, trailing_expr)
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
                    let stmt_span = get_node_span(child, ctx.file_id);
                    statements.push(Statement::expr(expr, stmt_span));
                }
            }
            _ if is_expression_kind(child.kind()) => {
                // Handle bare expression kinds (not wrapped in Expression)
                if is_last {
                    trailing_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let stmt_span = get_node_span(child, ctx.file_id);
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
/// Also handles while-let: label: while let pattern = expr { body }
fn resolve_while_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Parse optional label
    let label_info = extract_loop_label(node);

    // Check if this is a while-let expression (has WhileLetCondition child)
    let while_let_condition = node
        .children()
        .find(|c| c.kind() == SyntaxKind::WhileLetCondition);

    if let Some(condition_node) = while_let_condition {
        // This is a while-let expression
        return resolve_while_let_expression(node, &condition_node, label_info, ctx);
    }

    // Snapshot move state before the loop
    let pre_loop_moves = ctx.move_tracker.snapshot();

    // Regular while expression
    // Find condition expression
    let condition = node
        .children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
        .map(|c| resolve_expression(&c, ctx))
        .unwrap_or_else(|| Expression::error(span.clone()));

    // Enter the loop context with the label
    let label_name = label_info.as_ref().map(|l| l.name.clone());
    let label_span = label_info.as_ref().map(|l| l.span.clone());
    let loop_id = ctx.enter_loop(label_name, label_span);

    // Resolve the body
    let body = node
        .children()
        .find(|c| c.kind() == SyntaxKind::CodeBlock)
        .map(|c| resolve_loop_body(&c, ctx))
        .unwrap_or_default();

    // Exit the loop context
    ctx.exit_loop();

    // For while loops: the body might not execute (condition false on first check).
    // So any move inside the body is "maybe moved" after the loop.
    // Merge the body's move state with the pre-loop state.
    let after_body_moves = ctx.move_tracker.snapshot();
    ctx.move_tracker.restore(after_body_moves);
    ctx.move_tracker.merge(&pre_loop_moves);

    Expression::while_loop(loop_id, label_info, condition, body, span)
}

/// Resolve a while-let expression with chain support:
/// - Single: `label: while let pattern = expr { body }`
/// - Chain: `label: while let .Some(x) = a, let .Some(y) = b, x > 0 { body }`
fn resolve_while_let_expression(
    node: &SyntaxNode,
    _first_condition_node: &SyntaxNode,
    label_info: Option<LabelInfo>,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Snapshot move state before the loop
    let pre_loop_moves = ctx.move_tracker.snapshot();

    // Enter the loop context with the label
    let label_name = label_info.as_ref().map(|l| l.name.clone());
    let label_span = label_info.as_ref().map(|l| l.span.clone());
    let loop_id = ctx.enter_loop(label_name.clone(), label_span.clone());

    // Push a new scope for pattern bindings (visible in subsequent conditions and loop body)
    ctx.local_scope.push_scope();

    // Collect all conditions (WhileLetCondition or Expression children before CodeBlock)
    let mut conditions: Vec<IfCondition> = Vec::new();
    for child in node.children() {
        if child.kind() == SyntaxKind::CodeBlock {
            break;
        }
        if child.kind() == SyntaxKind::Expression || is_expression_kind(child.kind()) {
            // Boolean condition
            let cond_expr = resolve_expression(&child, ctx);
            conditions.push(IfCondition::Expr(cond_expr));
        } else if child.kind() == SyntaxKind::WhileLetCondition {
            // While-let condition: let pattern = expr
            let cond = resolve_while_let_condition(&child, ctx);
            conditions.push(cond);
        }
    }

    // Ensure we have at least one condition
    if conditions.is_empty() {
        conditions.push(IfCondition::Expr(Expression::error(span.clone())));
    }

    // Resolve the body (pattern bindings are now in scope)
    let body = node
        .children()
        .find(|c| c.kind() == SyntaxKind::CodeBlock)
        .map(|c| resolve_while_let_body(&c, ctx))
        .unwrap_or_default();

    // Pop the pattern scope
    ctx.local_scope.pop_scope();

    // Exit the loop context
    ctx.exit_loop();

    // For while-let loops: the body might not execute (pattern might not match).
    // So any move inside the body is "maybe moved" after the loop.
    let after_body_moves = ctx.move_tracker.snapshot();
    ctx.move_tracker.restore(after_body_moves);
    ctx.move_tracker.merge(&pre_loop_moves);

    Expression::while_let(loop_id, label_info, conditions, body, span)
}

/// Resolve a single while-let condition: let pattern = expr
fn resolve_while_let_condition(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> IfCondition {
    use super::patterns::resolve_pattern;
    use kestrel_semantic_tree::pattern::Pattern;

    let span = get_node_span(node, ctx.file_id);

    // Find the pattern
    let pattern_node = node
        .children()
        .find(|c| super::patterns::is_pattern_kind(c.kind()));

    // Find the value expression (the scrutinee)
    let value_node = node
        .children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()));

    let value = value_node
        .map(|n| resolve_expression(&n, ctx))
        .unwrap_or_else(|| Expression::error(span.clone()));

    // Resolve the pattern with the value type as expected type
    let pattern = pattern_node
        .map(|n| resolve_pattern(&n, ctx, Some(&value.ty)))
        .unwrap_or_else(|| Pattern::error(span.clone()));

    IfCondition::Let {
        pattern,
        value,
        span,
    }
}

/// Resolve the body of a while-let loop.
/// This creates a nested scope for the loop body while keeping pattern bindings visible.
fn resolve_while_let_body(block_node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Vec<Statement> {
    // Push a nested scope for local variables declared in the body
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
                let stmt_span = get_node_span(&child, ctx.file_id);
                statements.push(Statement::expr(expr, stmt_span));
            }
            _ if is_expression_kind(child.kind()) => {
                // Handle bare expression kinds
                let expr = resolve_expression(&child, ctx);
                let stmt_span = get_node_span(&child, ctx.file_id);
                statements.push(Statement::expr(expr, stmt_span));
            }
            _ => {}
        }
    }

    ctx.local_scope.pop_scope();
    statements
}

/// Resolve a loop expression: label: loop { body }
fn resolve_loop_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Parse optional label
    let label_info = extract_loop_label(node);

    // Enter the loop context with the label
    let label_name = label_info.as_ref().map(|l| l.name.clone());
    let label_span = label_info.as_ref().map(|l| l.span.clone());
    let loop_id = ctx.enter_loop(label_name, label_span);

    // Resolve the body
    let body = node
        .children()
        .find(|c| c.kind() == SyntaxKind::CodeBlock)
        .map(|c| resolve_loop_body(&c, ctx))
        .unwrap_or_default();

    // Exit the loop context
    ctx.exit_loop();

    // For infinite `loop`: the body always executes at least once.
    // Any move inside the body means the variable is definitely moved after the loop.
    // We also need to promote any maybe-moved to moved (for second iteration).
    ctx.move_tracker.promote_maybe_to_moved();

    Expression::loop_expr(loop_id, label_info, body, span)
}

/// Resolve a break expression: break or break label
fn resolve_break_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Check if we're in a loop
    if !ctx.in_loop() {
        let error = BreakOutsideLoopError { span: span.clone() };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
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
                ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            }
            return Expression::error(span);
        }
    };

    Expression::break_expr(loop_id, label_info, span)
}

/// Resolve a continue expression: continue or continue label
fn resolve_continue_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Check if we're in a loop
    if !ctx.in_loop() {
        let error = ContinueOutsideLoopError { span: span.clone() };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
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
                ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            }
            return Expression::error(span);
        }
    };

    Expression::continue_expr(loop_id, label_info, span)
}

/// Resolve a return expression: return or return expr
fn resolve_return_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Find the optional value expression
    // The ExprReturn contains: Return keyword, optional Expression child
    // Also validate that it's not a standalone type parameter reference
    let value = node
        .children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
        .map(|expr_node| {
            let expr = resolve_expression(&expr_node, ctx);
            validate_not_standalone_type_param(expr, ctx)
        });

    Expression::return_expr(value, span)
}

/// Resolve the body of a loop, returning statements.
/// This creates a new scope for the loop body.
fn resolve_loop_body(block_node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Vec<Statement> {
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
                let stmt_span = get_node_span(&child, ctx.file_id);
                statements.push(Statement::expr(expr, stmt_span));
            }
            _ if is_expression_kind(child.kind()) => {
                // Handle bare expression kinds
                let expr = resolve_expression(&child, ctx);
                let stmt_span = get_node_span(&child, ctx.file_id);
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
            label_node
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| t.kind() == SyntaxKind::Identifier)
                .map(|token| {
                    let text_range = token.text_range();
                    let start = text_range.start().into();
                    let end = text_range.end().into();
                    LabelInfo {
                        name: token.text().to_string(),
                        span: Span::from(start..end),
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
                span: Span::from(start..end),
            }
        })
}

/// Resolve a tuple index expression: tuple.0, tuple.1
fn resolve_tuple_index_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.file_id);

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
            let idx_span = Span::from(text_range.start().into()..text_range.end().into());
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
                    tuple_type: base_ty.to_string(),
                };
                ctx.diagnostics.add_diagnostic(error.into_diagnostic());
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
                base_type: base_ty.to_string(),
            };
            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            Expression::error(span)
        }
    }
}

/// Check if `it` was referenced anywhere in the closure body.
/// This walks the resolved statements and expressions to find any LocalRef to `it`.
fn check_it_referenced_in_closure(
    body: &[Statement],
    tail_expr: Option<&Expression>,
    ctx: &BodyResolutionContext,
) -> bool {
    // Look up what LocalId `it` is bound to in the current scope
    let it_local_id = match ctx.local_scope.lookup("it") {
        Some(id) => id,
        None => return false, // `it` not bound, so can't be referenced
    };

    // Check statements
    for stmt in body {
        if statement_references_local(stmt, it_local_id) {
            return true;
        }
    }

    // Check tail expression
    if let Some(expr) = tail_expr {
        if expression_references_local(expr, it_local_id) {
            return true;
        }
    }

    false
}

/// Check if a statement references a specific local.
fn statement_references_local(
    stmt: &Statement,
    local_id: kestrel_semantic_tree::symbol::local::LocalId,
) -> bool {
    use kestrel_semantic_tree::stmt::StatementKind;
    match &stmt.kind {
        StatementKind::Binding { value, .. } => {
            if let Some(v) = value {
                expression_references_local(v, local_id)
            } else {
                false
            }
        }
        StatementKind::Expr(expr) => expression_references_local(expr, local_id),
        StatementKind::GuardLet { conditions, else_block } => {
            // Check all conditions
            for condition in conditions {
                match condition {
                    IfCondition::Expr(expr) => {
                        if expression_references_local(expr, local_id) {
                            return true;
                        }
                    }
                    IfCondition::Let { value, .. } => {
                        if expression_references_local(value, local_id) {
                            return true;
                        }
                    }
                }
            }
            // Check else block statements
            for else_stmt in &else_block.statements {
                if statement_references_local(else_stmt, local_id) {
                    return true;
                }
            }
            if let Some(yield_expr) = &else_block.yield_expr {
                if expression_references_local(yield_expr, local_id) {
                    return true;
                }
            }
            false
        }
    }
}

/// Check if an expression references a specific local (recursively).
fn expression_references_local(
    expr: &Expression,
    local_id: kestrel_semantic_tree::symbol::local::LocalId,
) -> bool {
    use kestrel_semantic_tree::expr::{ExprKind, ElseBranch, IfCondition};

    match &expr.kind {
        // Direct reference to the local
        ExprKind::LocalRef(id) => *id == local_id,

        // Recursively check compound expressions
        ExprKind::Array(elements) | ExprKind::Tuple(elements) => {
            elements.iter().any(|e| expression_references_local(e, local_id))
        }

        ExprKind::Grouping(inner) => expression_references_local(inner, local_id),

        ExprKind::FieldAccess { object, .. } => expression_references_local(object, local_id),

        ExprKind::TupleIndex { tuple, .. } => expression_references_local(tuple, local_id),

        ExprKind::MethodRef { receiver, .. } => expression_references_local(receiver, local_id),

        ExprKind::Call { callee, arguments, .. } => {
            expression_references_local(callee, local_id)
                || arguments.iter().any(|arg| expression_references_local(&arg.value, local_id))
        }

        ExprKind::PrimitiveMethodCall { receiver, arguments, .. } => {
            expression_references_local(receiver, local_id)
                || arguments.iter().any(|arg| expression_references_local(&arg.value, local_id))
        }

        ExprKind::ImplicitStructInit { arguments, .. } => {
            arguments.iter().any(|arg| expression_references_local(&arg.value, local_id))
        }

        ExprKind::Assignment { target, value } => {
            expression_references_local(target, local_id)
                || expression_references_local(value, local_id)
        }

        ExprKind::If { conditions, then_branch, then_value, else_branch } => {
            // Check conditions
            for condition in conditions {
                match condition {
                    IfCondition::Expr(expr) => {
                        if expression_references_local(expr, local_id) {
                            return true;
                        }
                    }
                    IfCondition::Let { value, .. } => {
                        if expression_references_local(value, local_id) {
                            return true;
                        }
                    }
                }
            }

            for stmt in then_branch {
                if statement_references_local(stmt, local_id) {
                    return true;
                }
            }

            if let Some(then_val) = then_value {
                if expression_references_local(then_val, local_id) {
                    return true;
                }
            }

            if let Some(else_br) = else_branch {
                match else_br {
                    ElseBranch::Block { statements, value } => {
                        for stmt in statements {
                            if statement_references_local(stmt, local_id) {
                                return true;
                            }
                        }
                        if let Some(val) = value {
                            if expression_references_local(val, local_id) {
                                return true;
                            }
                        }
                    }
                    ElseBranch::ElseIf(if_expr) => {
                        if expression_references_local(if_expr, local_id) {
                            return true;
                        }
                    }
                }
            }

            false
        }

        ExprKind::While { condition, body, .. } => {
            if expression_references_local(condition, local_id) {
                return true;
            }
            for stmt in body {
                if statement_references_local(stmt, local_id) {
                    return true;
                }
            }
            false
        }

        ExprKind::WhileLet { conditions, body, .. } => {
            // Check all conditions
            for condition in conditions {
                match condition {
                    IfCondition::Expr(expr) => {
                        if expression_references_local(expr, local_id) {
                            return true;
                        }
                    }
                    IfCondition::Let { value, .. } => {
                        if expression_references_local(value, local_id) {
                            return true;
                        }
                    }
                }
            }
            for stmt in body {
                if statement_references_local(stmt, local_id) {
                    return true;
                }
            }
            false
        }

        ExprKind::Loop { body, .. } => {
            for stmt in body {
                if statement_references_local(stmt, local_id) {
                    return true;
                }
            }
            false
        }

        ExprKind::Closure { body, tail_expr, .. } => {
            // Check nested closure body
            for stmt in body {
                if statement_references_local(stmt, local_id) {
                    return true;
                }
            }
            if let Some(tail) = tail_expr {
                if expression_references_local(tail, local_id) {
                    return true;
                }
            }
            false
        }

        ExprKind::Return { value } => {
            if let Some(val) = value {
                expression_references_local(val, local_id)
            } else {
                false
            }
        }

        // Implicit member access - check arguments if present
        ExprKind::ImplicitMemberAccess { arguments, .. } => {
            if let Some(args) = arguments {
                args.iter().any(|arg| expression_references_local(&arg.value, local_id))
            } else {
                false
            }
        }

        ExprKind::Match { scrutinee, arms } => {
            expression_references_local(scrutinee, local_id)
                || arms.iter().any(|arm| {
                    arm.guard
                        .as_ref()
                        .map(|g| expression_references_local(g, local_id))
                        .unwrap_or(false)
                        || expression_references_local(&arm.body, local_id)
                })
        }

        // Leaf expressions - no references
        ExprKind::Literal(_)
        | ExprKind::SymbolRef(_)
        | ExprKind::OverloadedRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::AssociatedTypeRef
        | ExprKind::EnumCase { .. }
        | ExprKind::Break { .. }
        | ExprKind::Continue { .. }
        | ExprKind::Error => false,
    }
}

/// Resolve a closure expression: `{ params in body }` or `{ body }`
fn resolve_closure_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Record the scope depth before entering the closure (for capture analysis)
    let closure_entry_depth = ctx.local_scope.depth();

    // Push a new scope for the closure
    ctx.local_scope.push_scope();

    // Parse parameters from ClosureParams node
    let params = resolve_closure_params(node, ctx);

    // If no explicit parameters, inject `it` as an implicit parameter
    // This allows `it` to be referenced in the body (but only errors if actually used with wrong arity)
    let implicit_param = if params.is_none() {
        // Create an infer type for `it`
        let it_ty = kestrel_semantic_tree::ty::Ty::infer(span.clone());
        // Bind `it` as an immutable local in the closure scope
        let local_id = ctx.local_scope.bind("it".to_string(), it_ty.clone(), false, span.clone());
        Some((local_id, it_ty, span.clone()))
    } else {
        None
    };
    
    let has_it = implicit_param.is_some();

    // Resolve the closure body (statements and trailing expression)
    let (body, tail_expr) = resolve_closure_body(node, ctx);

    // Check if `it` was actually referenced in the body (if we injected it)
    let it_was_used = if has_it {
        check_it_referenced_in_closure(&body, tail_expr.as_ref(), ctx)
    } else {
        false
    };

    // Pop the closure scope
    ctx.local_scope.pop_scope();

    // Determine closure type
    let closure_ty = if let Some(param_list) = &params {
        // Explicit parameters - we know the parameter types
        let param_types: Vec<kestrel_semantic_tree::ty::Ty> =
            param_list.iter().map(|p| p.ty.clone()).collect();

        let return_ty = tail_expr
            .as_ref()
            .map(|e| e.ty.clone())
            .unwrap_or_else(|| kestrel_semantic_tree::ty::Ty::unit(span.clone()));

        kestrel_semantic_tree::ty::Ty::function(param_types, return_ty, span.clone())
    } else {
        // No explicit parameters - create UnresolvedFunction with appropriate ParamInfo
        use kestrel_semantic_tree::ty::ParamInfo;

        let return_ty = tail_expr
            .as_ref()
            .map(|e| e.ty.clone())
            .unwrap_or_else(|| kestrel_semantic_tree::ty::Ty::unit(span.clone()));

        if it_was_used {
            // Uses implicit `it` - exactly 1 param
            let it_ty = implicit_param.as_ref().unwrap().1.clone();
            kestrel_semantic_tree::ty::Ty::unresolved_function(
                ParamInfo::ImplicitIt { it_type: Box::new(it_ty) },
                return_ty,
                span.clone(),
            )
        } else {
            // No params, no `it` - unconstrained arity (could be any)
            kestrel_semantic_tree::ty::Ty::unresolved_function(
                ParamInfo::Unconstrained,
                return_ty,
                span.clone(),
            )
        }
    };

    // Collect captured variables from the closure body
    let captures = collect_captures(&body, tail_expr.as_ref(), closure_entry_depth, &ctx.local_scope);

    Expression::closure(params, body, tail_expr, captures, it_was_used, implicit_param, closure_ty, span)
}

/// Resolve closure parameters from the syntax tree.
fn resolve_closure_params(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<Vec<kestrel_semantic_tree::expr::ClosureParam>> {
    use crate::resolution::type_resolver::TypeResolver;

    // Find ClosureParams node
    let params_node = node
        .children()
        .find(|c| c.kind() == SyntaxKind::ClosureParams)?;

    let mut params = Vec::new();
    for child in params_node.children() {
        if child.kind() == SyntaxKind::ClosureParam {
            let param_span = get_node_span(&child, ctx.file_id);

            // Extract name
            let name = child
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| t.kind() == SyntaxKind::Identifier)
                .map(|t| t.text().to_string())?;

            // Extract optional type annotation
            let ty_node = child.children().find(|c| c.kind() == SyntaxKind::Ty);
            let (ty, is_annotated) = match ty_node {
                Some(tn) => {
                    let mut resolver = TypeResolver::new(
                        ctx.model,
                        ctx.diagnostics,
                        ctx.source,
                        ctx.file_id,
                        ctx.function_id,
                    );
                    let resolved_ty = resolver.resolve(&tn);
                    (resolved_ty, true)
                }
                None => (kestrel_semantic_tree::ty::Ty::infer(param_span.clone()), false),
            };

            // Bind parameter as local
            let local_id = ctx.local_scope.bind(
                name.clone(),
                ty.clone(),
                false, // closure params are immutable
                param_span.clone(),
            );

            params.push(kestrel_semantic_tree::expr::ClosureParam {
                name,
                ty,
                is_type_annotated: is_annotated,
                span: param_span,
                local_id,
            });
        }
    }

    Some(params)
}

/// Resolve the body of a closure.
fn resolve_closure_body(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> (Vec<Statement>, Option<Expression>) {
    let mut statements = Vec::new();
    let mut tail_expr = None;

    // Collect all children (nodes only, not tokens)
    let children: Vec<_> = node.children().collect();

    // Find where the body starts (after ClosureParams if present)
    // The 'in' keyword is a token, not a node, so we skip it automatically
    let mut body_start = 0;
    for (i, child) in children.iter().enumerate() {
        if child.kind() == SyntaxKind::ClosureParams {
            body_start = i + 1;
            break;
        }
    }

    // Process body items
    let body_children = &children[body_start..];
    for (i, child) in body_children.iter().enumerate() {
        let is_last = i == body_children.len() - 1;

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
                    tail_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let stmt_span = get_node_span(child, ctx.file_id);
                    statements.push(Statement::expr(expr, stmt_span));
                }
            }
            _ if is_expression_kind(child.kind()) => {
                // Handle bare expression kinds (not wrapped in Expression)
                if is_last {
                    tail_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let stmt_span = get_node_span(child, ctx.file_id);
                    statements.push(Statement::expr(expr, stmt_span));
                }
            }
            // Skip tokens like braces, 'in' keyword
            _ => {}
        }
    }

    (statements, tail_expr)
}

/// Resolve an implicit member access expression: `.foo` or `.foo(args)`
///
/// This handles Swift-style shorthand for enum cases like `.None` instead of `Option.None`.
/// The actual type resolution happens during type inference when the expected type is known.
fn resolve_implicit_member_access(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Extract member name from Name child
    let member_name = node
        .children()
        .find(|c| c.kind() == SyntaxKind::Name)
        .and_then(|name_node| {
            name_node
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| t.kind() == SyntaxKind::Identifier)
                .map(|t| t.text().to_string())
        })
        .unwrap_or_else(|| "?".to_string());

    // Extract optional arguments from ArgumentList
    let arguments = node
        .children()
        .find(|c| c.kind() == SyntaxKind::ArgumentList)
        .map(|arg_list| resolve_argument_list(&arg_list, ctx));

    // Return with Ty::infer() - type inference will resolve the actual type
    Expression::implicit_member_access(member_name, arguments, span)
}

/// Collect captured variables from a closure body.
///
/// A variable is captured if:
/// 1. It's referenced as a LocalRef in the body or tail expression
/// 2. It was declared before the closure scope (scope_depth < closure_entry_depth)
///
/// Captures are deduplicated by LocalId.
fn collect_captures(
    body: &[Statement],
    tail_expr: Option<&Expression>,
    closure_entry_depth: usize,
    local_scope: &crate::LocalScope,
) -> Vec<kestrel_semantic_tree::expr::Capture> {
    use kestrel_semantic_tree::expr::{Capture, CaptureKind};
    use kestrel_semantic_tree::symbol::local::LocalId;
    use std::collections::HashSet;

    let mut captures = Vec::new();
    let mut seen_ids: HashSet<LocalId> = HashSet::new();

    // Helper to process a single LocalRef
    let mut process_local_ref = |local_id: LocalId, _name: &str, ty: &kestrel_semantic_tree::ty::Ty, span: &kestrel_span::Span| {
        // Check if already captured
        if seen_ids.contains(&local_id) {
            return;
        }

        // Check if this local was declared before the closure scope
        // Variables at closure_entry_depth or below are from outer scopes
        if let Some(local_depth) = local_scope.scope_depth_of(local_id) {
            if local_depth <= closure_entry_depth {
                // This is a capture! Get the name from the local_scope
                let name = local_scope
                    .get_local(local_id)
                    .map(|l| l.name().to_string())
                    .unwrap_or_default();

                seen_ids.insert(local_id);
                captures.push(Capture {
                    local_id,
                    name,
                    ty: ty.clone(),
                    kind: CaptureKind::Value,
                    span: span.clone(),
                });
            }
        }
    };

    // Walk all statements
    for stmt in body {
        collect_captures_from_statement(stmt, &mut process_local_ref);
    }

    // Walk the tail expression
    if let Some(expr) = tail_expr {
        collect_captures_from_expression(expr, &mut process_local_ref);
    }

    captures
}

/// Walk a statement to find LocalRef expressions.
fn collect_captures_from_statement<F>(stmt: &Statement, process: &mut F)
where
    F: FnMut(kestrel_semantic_tree::symbol::local::LocalId, &str, &kestrel_semantic_tree::ty::Ty, &kestrel_span::Span),
{
    use kestrel_semantic_tree::stmt::StatementKind;

    match &stmt.kind {
        StatementKind::Expr(expr) => {
            collect_captures_from_expression(expr, process);
        }
        StatementKind::Binding { value, .. } => {
            // Walk the initializer value if present
            if let Some(expr) = value {
                collect_captures_from_expression(expr, process);
            }
        }
        StatementKind::GuardLet { conditions, else_block } => {
            // Walk all conditions
            for condition in conditions {
                match condition {
                    IfCondition::Expr(expr) => {
                        collect_captures_from_expression(expr, process);
                    }
                    IfCondition::Let { value, .. } => {
                        collect_captures_from_expression(value, process);
                    }
                }
            }
            // Walk else block
            for else_stmt in &else_block.statements {
                collect_captures_from_statement(else_stmt, process);
            }
            if let Some(yield_expr) = &else_block.yield_expr {
                collect_captures_from_expression(yield_expr, process);
            }
        }
    }
}

/// Walk an expression to find LocalRef expressions.
fn collect_captures_from_expression<F>(expr: &Expression, process: &mut F)
where
    F: FnMut(kestrel_semantic_tree::symbol::local::LocalId, &str, &kestrel_semantic_tree::ty::Ty, &kestrel_span::Span),
{
    use kestrel_semantic_tree::expr::ExprKind;

    match &expr.kind {
        // The key case: LocalRef - look up the name from the local scope
        ExprKind::LocalRef(local_id) => {
            // We need to get the name from somewhere - use the local_scope or just use a placeholder
            // Actually, we need to pass the name through. Let's look it up from the context.
            // For now, we'll use the expression span to identify the capture location.
            // The actual name will be retrieved later during capture creation.
            process(*local_id, "", &expr.ty, &expr.span);
        }

        // Recursively walk compound expressions
        ExprKind::Grouping(inner) => {
            collect_captures_from_expression(inner, process);
        }
        ExprKind::Call { callee, arguments, .. } => {
            collect_captures_from_expression(callee, process);
            for arg in arguments {
                collect_captures_from_expression(&arg.value, process);
            }
        }
        ExprKind::PrimitiveMethodCall { receiver, arguments, .. } => {
            collect_captures_from_expression(receiver, process);
            for arg in arguments {
                collect_captures_from_expression(&arg.value, process);
            }
        }
        ExprKind::MethodRef { receiver, .. } => {
            collect_captures_from_expression(receiver, process);
        }
        ExprKind::FieldAccess { object, .. } => {
            collect_captures_from_expression(object, process);
        }
        ExprKind::TupleIndex { tuple, .. } => {
            collect_captures_from_expression(tuple, process);
        }
        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments {
                collect_captures_from_expression(&arg.value, process);
            }
        }
        ExprKind::Assignment { target, value } => {
            collect_captures_from_expression(target, process);
            collect_captures_from_expression(value, process);
        }
        ExprKind::Tuple(elements) => {
            for elem in elements {
                collect_captures_from_expression(elem, process);
            }
        }
        ExprKind::Array(elements) => {
            for elem in elements {
                collect_captures_from_expression(elem, process);
            }
        }
        ExprKind::If { conditions, then_branch, then_value, else_branch } => {
            // Collect from conditions
            for condition in conditions {
                match condition {
                    IfCondition::Expr(expr) => {
                        collect_captures_from_expression(expr, process);
                    }
                    IfCondition::Let { value, .. } => {
                        collect_captures_from_expression(value, process);
                    }
                }
            }
            for stmt in then_branch {
                collect_captures_from_statement(stmt, process);
            }
            if let Some(val) = then_value {
                collect_captures_from_expression(val, process);
            }
            if let Some(else_br) = else_branch {
                collect_captures_from_else_branch(else_br, process);
            }
        }
        ExprKind::While { condition, body: while_body, .. } => {
            collect_captures_from_expression(condition, process);
            for stmt in while_body {
                collect_captures_from_statement(stmt, process);
            }
        }
        ExprKind::WhileLet { conditions, body: while_body, .. } => {
            // Walk all conditions
            for condition in conditions {
                match condition {
                    IfCondition::Expr(expr) => {
                        collect_captures_from_expression(expr, process);
                    }
                    IfCondition::Let { value, .. } => {
                        collect_captures_from_expression(value, process);
                    }
                }
            }
            for stmt in while_body {
                collect_captures_from_statement(stmt, process);
            }
        }
        ExprKind::Loop { body: loop_body, .. } => {
            for stmt in loop_body {
                collect_captures_from_statement(stmt, process);
            }
        }
        ExprKind::Break { .. } => {
            // Break doesn't have a value in this AST
        }
        ExprKind::Continue { .. } => {}
        ExprKind::Return { value } => {
            if let Some(val) = value {
                collect_captures_from_expression(val, process);
            }
        }
        ExprKind::Closure { body, tail_expr, .. } => {
            // For nested closures, we still walk their bodies to find captures
            // These will be captures from the outer closure's perspective
            for stmt in body {
                collect_captures_from_statement(stmt, process);
            }
            if let Some(tail) = tail_expr {
                collect_captures_from_expression(tail, process);
            }
        }

        // Implicit member access - check arguments if present
        ExprKind::ImplicitMemberAccess { arguments, .. } => {
            if let Some(args) = arguments {
                for arg in args {
                    collect_captures_from_expression(&arg.value, process);
                }
            }
        }

        // Match expression - walk scrutinee and all arms
        ExprKind::Match { scrutinee, arms } => {
            collect_captures_from_expression(scrutinee, process);
            for arm in arms {
                // Pattern bindings don't capture, but guard and body expressions do
                if let Some(guard) = &arm.guard {
                    collect_captures_from_expression(guard, process);
                }
                collect_captures_from_expression(&arm.body, process);
            }
        }

        // Leaf nodes - no recursion needed
        ExprKind::Literal(_)
        | ExprKind::SymbolRef(_)
        | ExprKind::OverloadedRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::AssociatedTypeRef
        | ExprKind::EnumCase { .. }
        | ExprKind::Error => {}
    }
}

/// Walk an else branch to find LocalRef expressions.
fn collect_captures_from_else_branch<F>(else_branch: &kestrel_semantic_tree::expr::ElseBranch, process: &mut F)
where
    F: FnMut(kestrel_semantic_tree::symbol::local::LocalId, &str, &kestrel_semantic_tree::ty::Ty, &kestrel_span::Span),
{
    match else_branch {
        kestrel_semantic_tree::expr::ElseBranch::Block { statements, value } => {
            for stmt in statements {
                collect_captures_from_statement(stmt, process);
            }
            if let Some(val) = value {
                collect_captures_from_expression(val, process);
            }
        }
        kestrel_semantic_tree::expr::ElseBranch::ElseIf(expr) => {
            collect_captures_from_expression(expr, process);
        }
    }
}

/// Resolve a match expression: `match scrutinee { pattern => body, ... }`
fn resolve_match_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Find the scrutinee expression (first Expression child)
    let scrutinee_node = match node
        .children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
    {
        Some(n) => n,
        None => return Expression::error(span),
    };

    let scrutinee = resolve_expression(&scrutinee_node, ctx);
    let scrutinee_ty = &scrutinee.ty;

    // Snapshot move state before match arms (for branching)
    let pre_match_moves = ctx.move_tracker.snapshot();

    // Collect all match arms and their move states
    let mut arms = Vec::new();
    let mut arm_move_states = Vec::new();

    for child in node.children() {
        if child.kind() == SyntaxKind::MatchArm {
            // Restore to pre-match state before each arm
            ctx.move_tracker.restore(pre_match_moves.clone());

            if let Some(arm) = resolve_match_arm(&child, scrutinee_ty, ctx) {
                arms.push(arm);
                // Capture the move state after this arm
                arm_move_states.push(ctx.move_tracker.snapshot());
            }
        }
    }

    // Merge all arm move states
    // Match is exhaustive, so all arms are valid paths
    if !arm_move_states.is_empty() {
        ctx.move_tracker.merge_all(&arm_move_states);
    } else {
        // No arms - restore pre-match state
        ctx.move_tracker.restore(pre_match_moves);
    }

    Expression::match_expr(scrutinee, arms, span)
}

/// Resolve a single match arm: `pattern [if guard] => body`
fn resolve_match_arm(
    node: &SyntaxNode,
    scrutinee_ty: &kestrel_semantic_tree::ty::Ty,
    ctx: &mut BodyResolutionContext,
) -> Option<kestrel_semantic_tree::expr::MatchArm> {
    use kestrel_semantic_tree::expr::MatchArm;
    use super::patterns::resolve_pattern;

    let span = get_node_span(node, ctx.file_id);

    // Push a new scope for the arm (pattern bindings are local to the arm)
    ctx.local_scope.push_scope();

    // Find and resolve the pattern with the scrutinee type as expected type
    let pattern_node = node.children().find(|c| super::patterns::is_pattern_kind(c.kind()))?;
    let pattern = resolve_pattern(&pattern_node, ctx, Some(scrutinee_ty));

    // Find optional guard (MatchArmGuard node containing an expression)
    let guard = node
        .children()
        .find(|c| c.kind() == SyntaxKind::MatchArmGuard)
        .and_then(|guard_node| {
            guard_node
                .children()
                .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
                .map(|expr_node| resolve_expression(&expr_node, ctx))
        });

    // Find the body expression (after the fat arrow =>)
    // The body is the Expression child that comes after the pattern and optional guard
    let body_node = node
        .children()
        .filter(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
        .last()?;

    // Special handling for closure bodies in match arms:
    // If the body is a closure without explicit parameters, we treat it as an inline block
    // rather than a closure. This ensures outer variables are accessible without being
    // treated as captures (which would prevent assignment).
    let body = resolve_match_arm_body(&body_node, ctx);

    // Pop the arm scope
    ctx.local_scope.pop_scope();

    Some(if guard.is_some() {
        MatchArm::with_guard(pattern, guard.unwrap(), body, span)
    } else {
        MatchArm::new(pattern, body, span)
    })
}

/// Resolve a match arm body, handling closures specially.
///
/// When the body is a closure expression without explicit parameters (i.e., just `{ ... }`),
/// we resolve it as an inline block rather than a closure. This prevents outer variables
/// from being treated as captures (which would disallow assignment).
fn resolve_match_arm_body(
    body_node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    // Unwrap Expression wrapper if present
    let inner_node = if body_node.kind() == SyntaxKind::Expression {
        body_node.children().next().unwrap_or(body_node.clone())
    } else {
        body_node.clone()
    };

    // Check if it's a closure without explicit parameters
    if inner_node.kind() == SyntaxKind::ExprClosure {
        // Check if it has ClosureParams - if so, it's a real closure with explicit params
        let has_explicit_params = inner_node
            .children()
            .any(|c| c.kind() == SyntaxKind::ClosureParams);

        if !has_explicit_params {
            // It's a block-like closure (no params). Resolve the body inline.
            // Don't push a new scope - we're already in the match arm's scope.
            let (statements, tail_expr) = resolve_closure_body(&inner_node, ctx);
            let body_span = get_node_span(&inner_node, ctx.file_id);

            // If there are no statements and just a tail expression, return it directly
            if statements.is_empty() {
                if let Some(expr) = tail_expr {
                    return expr;
                }
                // Empty closure body - return unit
                return Expression::unit(body_span);
            }

            // If there are statements, wrap them in a closure without captures.
            // This maintains the semantic structure while avoiding capture issues.
            let result_ty = tail_expr.as_ref()
                .map(|e| e.ty.clone())
                .unwrap_or_else(|| Ty::unit(body_span.clone()));
            return Expression::closure(
                None,           // no params
                statements,
                tail_expr,
                Vec::new(),     // no captures - this is the key fix!
                false,          // doesn't use `it`
                None,           // no implicit param
                result_ty,
                body_span,
            );
        }
    }

    // For all other cases (non-closure, or closure with explicit params),
    // resolve normally using the exact same code path as before
    resolve_expression(body_node, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_span::Span;

    #[test]
    fn test_parse_integer_literal() {
        assert_eq!(parse_integer_literal("42"), Some(42));
        assert_eq!(parse_integer_literal("0xFF"), Some(255));
        assert_eq!(parse_integer_literal("0b1010"), Some(10));
        assert_eq!(parse_integer_literal("0o17"), Some(15));
        assert_eq!(parse_integer_literal("1_000_000"), Some(1000000));
    }
}
