//! Core expression resolution.
//!
//! This module handles resolving expression syntax nodes into semantic Expression
//! representations. It dispatches to specialized modules for complex expressions
//! like calls, operators, and paths.

use kestrel_reporting::IntoDiagnostic;
use kestrel_semantic_tree::expr::{ElseBranch, Expression, LabelInfo};
use kestrel_semantic_tree::stmt::Statement;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::diagnostics::{
    BreakOutsideLoopError, ContinueOutsideLoopError, TupleIndexOnNonTupleError,
    TupleIndexOutOfBoundsError, UndeclaredLabelError,
};
use kestrel_syntax_tree::utils::get_node_span;

use super::calls::resolve_call_expression;
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
fn resolve_if_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

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
    let else_branch = node
        .children()
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
fn resolve_while_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Parse optional label
    let label_info = extract_loop_label(node);

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

    Expression::while_loop(loop_id, label_info, condition, body, span)
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
    }
}

/// Check if an expression references a specific local (recursively).
fn expression_references_local(
    expr: &Expression,
    local_id: kestrel_semantic_tree::symbol::local::LocalId,
) -> bool {
    use kestrel_semantic_tree::expr::{ExprKind, ElseBranch};

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

        ExprKind::If { condition, then_branch, then_value, else_branch } => {
            if expression_references_local(condition, local_id) {
                return true;
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

        // Leaf expressions - no references
        ExprKind::Literal(_)
        | ExprKind::SymbolRef(_)
        | ExprKind::OverloadedRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
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
            ctx.local_scope.bind(
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
        ExprKind::If { condition, then_branch, then_value, else_branch } => {
            collect_captures_from_expression(condition, process);
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

        // Leaf nodes - no recursion needed
        ExprKind::Literal(_)
        | ExprKind::SymbolRef(_)
        | ExprKind::OverloadedRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
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
