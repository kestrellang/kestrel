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
    AsciiEscapeOutOfRangeError, BreakOutsideLoopError, ContinueOutsideLoopError,
    EmptyCharacterLiteralError, IncompleteEscapeSequenceError, InvalidEscapeSequenceError,
    InvalidUnicodeEscapeError, MultipleCodepointsInCharLiteralError, TupleIndexOnNonTupleError,
    TupleIndexOutOfBoundsError, UndeclaredLabelError, UnicodeEscapeErrorReason,
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
        },

        SyntaxKind::ExprUnit => Expression::unit(span),

        SyntaxKind::ExprInteger => {
            let value = extract_integer_value(expr_node);
            // Use inference type so literal protocols can be applied
            Expression::integer_infer(value, span)
        },

        SyntaxKind::ExprFloat => {
            let value = extract_float_value(expr_node);
            // Use inference type so literal protocols can be applied
            Expression::float_infer(value, span)
        },

        SyntaxKind::ExprString => {
            let value = extract_string_value(expr_node, ctx);
            // Use inference type so literal protocols can be applied
            Expression::string_infer(value, span)
        },

        SyntaxKind::ExprRawString => {
            let value = extract_raw_string_value(expr_node);
            // Use inference type so literal protocols can be applied
            Expression::string_infer(value, span)
        },

        SyntaxKind::ExprChar => {
            let value = extract_char_value(expr_node, ctx);
            // Use inference type so literal protocols can be applied
            Expression::char_infer(value, span)
        },

        SyntaxKind::ExprBool => {
            let value = extract_bool_value(expr_node);
            // Use inference type so literal protocols can be applied
            Expression::bool_infer(value, span)
        },

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
        },

        SyntaxKind::ExprCall => resolve_call_expression(expr_node, ctx),

        SyntaxKind::ExprAssignment => resolve_assignment_expression(expr_node, ctx),

        SyntaxKind::ExprIf => resolve_if_expression(expr_node, ctx),

        SyntaxKind::ExprWhile => resolve_while_expression(expr_node, ctx),

        SyntaxKind::ExprLoop => resolve_loop_expression(expr_node, ctx),

        SyntaxKind::ExprFor => resolve_for_expression(expr_node, ctx),

        SyntaxKind::ExprBreak => resolve_break_expression(expr_node, ctx),

        SyntaxKind::ExprContinue => resolve_continue_expression(expr_node, ctx),

        SyntaxKind::ExprReturn => resolve_return_expression(expr_node, ctx),

        SyntaxKind::ExprTry => resolve_try_expression(expr_node, ctx),

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

/// Extract string value from an ExprString node (strips quotes and processes escapes)
fn extract_string_value(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> String {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::String)
        .map(|t| {
            let text = t.text();
            let text_range = t.text_range();
            let token_start: usize = text_range.start().into();
            // Strip surrounding quotes
            if text.len() >= 2 {
                let inner = &text[1..text.len() - 1];
                // Process escape sequences, offset by 1 for the opening quote
                unescape_string(inner, ctx.file_id, token_start + 1, ctx)
            } else {
                text.to_string()
            }
        })
        .unwrap_or_default()
}

/// Extract raw string value from an ExprRawString node (strips quotes, no escape processing)
fn extract_raw_string_value(node: &SyntaxNode) -> String {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::RawString)
        .map(|t| {
            let text = t.text();
            // Count opening quotes (minimum 3)
            let quote_count = text.chars().take_while(|&c| c == '"').count();
            // Strip surrounding quotes
            if text.len() >= quote_count * 2 {
                text[quote_count..text.len() - quote_count].to_string()
            } else {
                text.to_string()
            }
        })
        .unwrap_or_default()
}

/// Extract character value from an ExprChar node (strips quotes, processes escapes, validates single codepoint)
fn extract_char_value(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> u32 {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Char)
        .map(|t| {
            let text = t.text();
            let text_range = t.text_range();
            let token_start: usize = text_range.start().into();
            let token_span = Span::new(ctx.file_id, token_start..token_start + text.len());

            // Strip surrounding single quotes
            if text.len() >= 2 {
                let inner = &text[1..text.len() - 1];

                if inner.is_empty() {
                    // Empty character literal ''
                    let error = EmptyCharacterLiteralError { span: token_span };
                    ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                    return 0;
                }

                // Process escape sequences, collecting code points
                let unescaped = unescape_string(inner, ctx.file_id, token_start + 1, ctx);
                let code_points: Vec<char> = unescaped.chars().collect();

                if code_points.is_empty() {
                    // After escape processing, still empty (shouldn't happen normally)
                    let error = EmptyCharacterLiteralError { span: token_span };
                    ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                    0
                } else if code_points.len() > 1 {
                    // Multiple code points
                    let error = MultipleCodepointsInCharLiteralError {
                        span: token_span,
                        count: code_points.len(),
                    };
                    ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                    // Return the first code point as a fallback
                    code_points[0] as u32
                } else {
                    code_points[0] as u32
                }
            } else {
                0
            }
        })
        .unwrap_or(0)
}

/// Process escape sequences in a string literal.
///
/// Supports:
/// - Basic escapes: \n, \r, \t, \\, \", \', \0
/// - Hex ASCII escapes: \xNN (must be 0x00-0x7F)
/// - Unicode escapes: \u{NNNN} (1-6 hex digits, max 0x10FFFF)
/// - Line continuation: \ followed by newline (skips the newline)
pub(crate) fn unescape_string(
    s: &str,
    file_id: usize,
    base_offset: usize,
    ctx: &mut BodyResolutionContext,
) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }

        // We have a backslash - look at the next character
        let escape_start = base_offset + i;
        match chars.next() {
            None => {
                // Backslash at end of string
                let error = IncompleteEscapeSequenceError {
                    span: Span::new(file_id, escape_start..escape_start + 1),
                };
                ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                result.push('\\');
            },
            Some((j, next_char)) => {
                match next_char {
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    '\'' => result.push('\''),
                    '0' => result.push('\0'),
                    // Line continuation: \ followed by newline
                    '\n' => {
                        // Skip the newline (line continuation)
                        // Also skip any leading whitespace on the next line
                        while let Some(&(_, ch)) = chars.peek() {
                            if ch == ' ' || ch == '\t' {
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    },
                    '\r' => {
                        // Handle \r\n as line continuation
                        if let Some(&(_, '\n')) = chars.peek() {
                            chars.next();
                        }
                        // Skip leading whitespace on next line
                        while let Some(&(_, ch)) = chars.peek() {
                            if ch == ' ' || ch == '\t' {
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    },
                    // Hex escape: \xNN
                    'x' => {
                        let hex_start = base_offset + j + 1;
                        let mut hex_str = String::new();
                        for _ in 0..2 {
                            if let Some(&(_, ch)) = chars.peek() {
                                if ch.is_ascii_hexdigit() {
                                    hex_str.push(ch);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                        }
                        if hex_str.len() != 2 {
                            let error = InvalidEscapeSequenceError {
                                span: Span::new(file_id, escape_start..hex_start + hex_str.len()),
                                sequence: format!("\\x{}", hex_str),
                            };
                            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                            result.push_str(&format!("\\x{}", hex_str));
                        } else {
                            let value = u8::from_str_radix(&hex_str, 16).unwrap();
                            if value > 0x7F {
                                let error = AsciiEscapeOutOfRangeError {
                                    span: Span::new(file_id, escape_start..hex_start + 2),
                                    value,
                                };
                                ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                                result.push_str(&format!("\\x{:02X}", value));
                            } else {
                                result.push(value as char);
                            }
                        }
                    },
                    // Unicode escape: \u{NNNN}
                    'u' => {
                        let u_pos = base_offset + j;
                        // Expect opening brace
                        if chars.peek().map(|&(_, c)| c) != Some('{') {
                            let error = InvalidUnicodeEscapeError {
                                span: Span::new(file_id, escape_start..u_pos + 1),
                                value: "\\u".to_string(),
                                reason: UnicodeEscapeErrorReason::MissingOpenBrace,
                            };
                            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                            result.push_str("\\u");
                            continue;
                        }
                        chars.next(); // consume '{'

                        let mut hex_str = String::new();
                        let mut found_close = false;
                        while let Some(&(_, ch)) = chars.peek() {
                            if ch == '}' {
                                chars.next();
                                found_close = true;
                                break;
                            } else if ch.is_ascii_hexdigit() && hex_str.len() < 6 {
                                hex_str.push(ch);
                                chars.next();
                            } else if ch.is_ascii_hexdigit() {
                                // Too many digits
                                hex_str.push(ch);
                                chars.next();
                            } else {
                                break;
                            }
                        }

                        let escape_end =
                            base_offset + j + 2 + hex_str.len() + if found_close { 1 } else { 0 };
                        let escape_seq = format!("\\u{{{}}}", hex_str);

                        if !found_close {
                            let error = InvalidUnicodeEscapeError {
                                span: Span::new(file_id, escape_start..escape_end),
                                value: escape_seq.clone(),
                                reason: UnicodeEscapeErrorReason::MissingCloseBrace,
                            };
                            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                            result.push_str(&escape_seq);
                        } else if hex_str.is_empty() {
                            let error = InvalidUnicodeEscapeError {
                                span: Span::new(file_id, escape_start..escape_end),
                                value: escape_seq.clone(),
                                reason: UnicodeEscapeErrorReason::EmptyBraces,
                            };
                            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                            result.push_str(&escape_seq);
                        } else if hex_str.len() > 6 {
                            let error = InvalidUnicodeEscapeError {
                                span: Span::new(file_id, escape_start..escape_end),
                                value: escape_seq.clone(),
                                reason: UnicodeEscapeErrorReason::TooManyDigits,
                            };
                            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                            result.push_str(&escape_seq);
                        } else {
                            match u32::from_str_radix(&hex_str, 16) {
                                Ok(code_point) if code_point <= 0x10FFFF => {
                                    if let Some(ch) = char::from_u32(code_point) {
                                        result.push(ch);
                                    } else {
                                        // Invalid unicode scalar (e.g., surrogate)
                                        let error = InvalidUnicodeEscapeError {
                                            span: Span::new(file_id, escape_start..escape_end),
                                            value: escape_seq.clone(),
                                            reason: UnicodeEscapeErrorReason::OutOfRange,
                                        };
                                        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                                        result.push_str(&escape_seq);
                                    }
                                },
                                _ => {
                                    let error = InvalidUnicodeEscapeError {
                                        span: Span::new(file_id, escape_start..escape_end),
                                        value: escape_seq.clone(),
                                        reason: UnicodeEscapeErrorReason::OutOfRange,
                                    };
                                    ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                                    result.push_str(&escape_seq);
                                },
                            }
                        }
                    },
                    // Unknown escape sequence
                    other => {
                        let error = InvalidEscapeSequenceError {
                            span: Span::new(
                                file_id,
                                escape_start..base_offset + j + other.len_utf8(),
                            ),
                            sequence: format!("\\{}", other),
                        };
                        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                        result.push('\\');
                        result.push(other);
                    },
                }
            },
        }
    }

    result
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
    let else_clause_node = node.children().find(|c| c.kind() == SyntaxKind::ElseClause);

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
                // Check if this is a trailing expression wrapped in a statement
                if is_last && let Some(expr) = try_extract_trailing_expression(child, ctx) {
                    trailing_expr = Some(expr);
                    continue;
                }
                if let Some(stmt) = resolve_statement(child, ctx) {
                    statements.push(stmt);
                }
            },
            SyntaxKind::VariableDeclaration => {
                if let Some(stmt) = super::statements::resolve_variable_declaration(child, ctx) {
                    statements.push(stmt);
                }
            },
            SyntaxKind::Expression => {
                // If last child and no semicolon, it's the trailing expression
                if is_last && !has_trailing_semicolon(child) {
                    trailing_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let stmt_span = get_node_span(child, ctx.file_id);
                    statements.push(Statement::expr(expr, stmt_span));
                }
            },
            _ if is_expression_kind(child.kind()) => {
                // Handle bare expression kinds (not wrapped in Expression)
                if is_last {
                    trailing_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let stmt_span = get_node_span(child, ctx.file_id);
                    statements.push(Statement::expr(expr, stmt_span));
                }
            },
            // Skip tokens like braces
            _ => {},
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
                // Check if this is a trailing expression wrapped in a statement
                if is_last && let Some(expr) = try_extract_trailing_expression(child, ctx) {
                    trailing_expr = Some(expr);
                    continue;
                }
                if let Some(stmt) = resolve_statement(child, ctx) {
                    statements.push(stmt);
                }
            },
            SyntaxKind::VariableDeclaration => {
                if let Some(stmt) = super::statements::resolve_variable_declaration(child, ctx) {
                    statements.push(stmt);
                }
            },
            SyntaxKind::Expression => {
                // If last child and no semicolon, it's the trailing expression
                if is_last && !has_trailing_semicolon(child) {
                    trailing_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let stmt_span = get_node_span(child, ctx.file_id);
                    statements.push(Statement::expr(expr, stmt_span));
                }
            },
            _ if is_expression_kind(child.kind()) => {
                // Handle bare expression kinds (not wrapped in Expression)
                if is_last {
                    trailing_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let stmt_span = get_node_span(child, ctx.file_id);
                    statements.push(Statement::expr(expr, stmt_span));
                }
            },
            // Skip tokens like braces
            _ => {},
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

/// Try to extract a trailing expression from a Statement or ExpressionStatement node.
/// Returns Some(expression) if this is a trailing expression (no semicolon, value-producing),
/// None otherwise.
fn try_extract_trailing_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<Expression> {
    // Don't extract if this node has a semicolon at its level
    if has_trailing_semicolon(node) {
        return None;
    }

    // Look for the inner content
    for child in node.children() {
        match child.kind() {
            SyntaxKind::ExpressionStatement => {
                // Recurse into ExpressionStatement
                return try_extract_trailing_expression(&child, ctx);
            },
            SyntaxKind::Expression => {
                // Found the expression wrapper - look inside for the actual expression
                if !has_trailing_semicolon(&child) {
                    // Check if the inner expression can produce a value
                    if can_be_trailing_expression(&child) {
                        return Some(resolve_expression(&child, ctx));
                    }
                }
            },
            // Also handle direct expression kinds (ExprIf, ExprMatch without Expression wrapper)
            SyntaxKind::ExprIf => {
                if !has_trailing_semicolon(&child) && has_value_else_branch(&child) {
                    return Some(resolve_expression(&child, ctx));
                }
            },
            SyntaxKind::ExprMatch => {
                if !has_trailing_semicolon(&child) {
                    return Some(resolve_expression(&child, ctx));
                }
            },
            _ => {},
        }
    }

    None
}

/// Check if an expression can be a trailing expression (produces a value).
fn can_be_trailing_expression(expr_node: &SyntaxNode) -> bool {
    // Look for the actual expression type inside the Expression wrapper
    if let Some(child) = expr_node.children().next() {
        match child.kind() {
            SyntaxKind::ExprIf => {
                // If-expression can be a trailing expression only if it has an else branch
                return has_value_else_branch(&child);
            },
            SyntaxKind::ExprMatch => {
                // Match expressions are always exhaustive and can be trailing expressions
                return true;
            },
            SyntaxKind::ExprLoop | SyntaxKind::ExprWhile | SyntaxKind::ExprFor => {
                // Loops cannot be trailing expressions - they return () or Never
                return false;
            },
            _ => {
                // Other expressions can be trailing expressions
                return true;
            },
        }
    }
    // If we found nothing inside, it's probably a simple expression
    true
}

/// Check if an if-expression has a complete else branch (can produce a value).
fn has_value_else_branch(if_node: &SyntaxNode) -> bool {
    // Find the ElseClause
    let else_clause = if_node
        .children()
        .find(|child| child.kind() == SyntaxKind::ElseClause);

    match else_clause {
        None => false, // No else at all
        Some(else_node) => {
            // Check what's inside the else clause
            for child in else_node.children() {
                match child.kind() {
                    SyntaxKind::ExprIf => {
                        // It's an "else if" - recursively check
                        return has_value_else_branch(&child);
                    },
                    SyntaxKind::CodeBlock => {
                        // It's a final "else { ... }" - it exists, so we have a value
                        return true;
                    },
                    SyntaxKind::Expression => {
                        // The else if might be wrapped in an Expression node
                        // Look inside for ExprIf
                        for inner in child.children() {
                            if inner.kind() == SyntaxKind::ExprIf {
                                return has_value_else_branch(&inner);
                            }
                        }
                    },
                    _ => {},
                }
            }
            false
        },
    }
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
    let label_info = extract_loop_label(node, ctx.file_id);

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

/// Resolve a for expression by desugaring to while-let:
/// ```
/// for pattern in iterable { body }
/// ```
/// becomes:
/// ```
/// {
///     var iter = iterable.iter()
///     while let .Some(pattern) = iter.next() {
///         body
///     }
/// }
/// ```
fn resolve_for_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    use kestrel_semantic_tree::expr::IfCondition;
    use kestrel_semantic_tree::pattern::{EnumPatternBinding, Mutability, Pattern};
    use kestrel_semantic_tree::stmt::Statement;

    let span = get_node_span(node, ctx.file_id);

    // Extract label if present (use the same function as while/loop)
    let label_info = extract_loop_label(node, ctx.file_id);

    // Find the ForPattern node
    let pattern_node = node
        .children()
        .find(|c| c.kind() == SyntaxKind::ForPattern)
        .and_then(|fp| {
            fp.children()
                .find(|c| super::patterns::is_pattern_kind(c.kind()))
        });

    // Find the ForIterable node
    let iterable_node = node
        .children()
        .find(|c| c.kind() == SyntaxKind::ForIterable)
        .and_then(|fi| {
            fi.children()
                .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
        });

    // Find the body (CodeBlock)
    let body_node = node
        .children()
        .find(|c| c.kind() == SyntaxKind::CodeBlock);

    // Resolve the iterable expression
    let iterable_expr = iterable_node
        .as_ref()
        .map(|n| resolve_expression(n, ctx))
        .unwrap_or_else(|| Expression::error(span.clone()));

    // Snapshot move state before the loop
    let pre_loop_moves = ctx.move_tracker.snapshot();

    // Push a new scope for the iterator variable and loop body
    ctx.local_scope.push_scope();

    // Create a synthetic iterator variable: var iter = iterable.iter()
    let iter_ty = Ty::infer(span.clone());
    let iter_local_id = ctx.local_scope.bind(
        "$for_iter".to_string(),
        iter_ty.clone(),
        true, // mutable
        span.clone(),
    );

    // Create the .iter() method call on the iterable
    let iter_call = Expression::deferred_method_call(
        iterable_expr,
        "iter".to_string(),
        vec![],
        iter_ty.clone(),
        span.clone(),
    );

    // Create the binding pattern for the iterator
    let iter_pattern = Pattern::local(
        iter_local_id,
        Mutability::Mutable,
        "$for_iter".to_string(),
        iter_ty.clone(),
        span.clone(),
    );

    // Create the binding statement: var iter = iterable.iter()
    let iter_binding = Statement::binding(iter_pattern, Some(iter_call), span.clone());

    // Enter the loop context with the label
    let label_name = label_info.as_ref().map(|l| l.name.clone());
    let label_span = label_info.as_ref().map(|l| l.span.clone());
    let loop_id = ctx.enter_loop(label_name, label_span);

    // Push a new scope for pattern bindings in the loop body
    ctx.local_scope.push_scope();

    // Create the .next() method call: iter.next()
    let item_ty = Ty::infer(span.clone());
    let optional_item_ty = Ty::infer(span.clone()); // Will be Optional[Item]
    let iter_ref = Expression::local_ref(iter_local_id, iter_ty, true, span.clone());
    let next_call = Expression::deferred_method_call(
        iter_ref,
        "next".to_string(),
        vec![],
        optional_item_ty.clone(),
        span.clone(),
    );

    // Resolve the user's pattern with the item type
    let user_pattern = pattern_node
        .as_ref()
        .map(|n| super::patterns::resolve_pattern(n, ctx, Some(&item_ty)))
        .unwrap_or_else(|| Pattern::error(span.clone()));

    // Create the .Some(pattern) enum pattern
    // Use the same type as next_call (optional_item_ty) so type inference connects them
    let some_binding = EnumPatternBinding::unlabeled(user_pattern, span.clone());
    let some_pattern = Pattern::enum_variant(
        None,
        "Some".to_string(),
        vec![some_binding],
        optional_item_ty.clone(),
        span.clone(),
    );

    // Create the while-let condition: let .Some(pattern) = iter.next()
    let condition = IfCondition::Let {
        pattern: some_pattern,
        value: next_call,
        span: span.clone(),
    };

    // Resolve the body
    let body = body_node
        .as_ref()
        .map(|c| resolve_while_let_body(c, ctx))
        .unwrap_or_default();

    // Pop the pattern scope
    ctx.local_scope.pop_scope();

    // Exit the loop context
    ctx.exit_loop();

    // For for loops: the body might not execute (iterator might be empty).
    // So any move inside the body is "maybe moved" after the loop.
    let after_body_moves = ctx.move_tracker.snapshot();
    ctx.move_tracker.restore(after_body_moves);
    ctx.move_tracker.merge(&pre_loop_moves);

    // Create the while-let expression (marked as from_for_loop for pattern checking)
    let while_let = Expression::while_let_from_for(loop_id, label_info, vec![condition], body, span.clone());

    // Pop the iterator scope
    ctx.local_scope.pop_scope();

    // Create the block expression: { var iter = ...; while let ... }
    Expression::block(vec![iter_binding], Some(while_let), Ty::unit(span.clone()), span)
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
fn resolve_while_let_body(
    block_node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Vec<Statement> {
    // Push a nested scope for local variables declared in the body
    ctx.local_scope.push_scope();

    let mut statements = Vec::new();

    for child in block_node.children() {
        match child.kind() {
            SyntaxKind::Statement | SyntaxKind::ExpressionStatement => {
                if let Some(stmt) = resolve_statement(&child, ctx) {
                    statements.push(stmt);
                }
            },
            SyntaxKind::VariableDeclaration => {
                if let Some(stmt) = super::statements::resolve_variable_declaration(&child, ctx) {
                    statements.push(stmt);
                }
            },
            SyntaxKind::Expression => {
                // Expressions in loop body become expression statements
                let expr = resolve_expression(&child, ctx);
                let stmt_span = get_node_span(&child, ctx.file_id);
                statements.push(Statement::expr(expr, stmt_span));
            },
            _ if is_expression_kind(child.kind()) => {
                // Handle bare expression kinds
                let expr = resolve_expression(&child, ctx);
                let stmt_span = get_node_span(&child, ctx.file_id);
                statements.push(Statement::expr(expr, stmt_span));
            },
            _ => {},
        }
    }

    ctx.local_scope.pop_scope();
    statements
}

/// Resolve a loop expression: label: loop { body }
fn resolve_loop_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Parse optional label
    let label_info = extract_loop_label(node, ctx.file_id);

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
    let label_info = extract_break_continue_label(node, ctx.file_id);
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
        },
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
    let label_info = extract_break_continue_label(node, ctx.file_id);
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
        },
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

/// Resolve a try expression: try expr
///
/// Desugars to:
/// ```text
/// match expr.tryExtract() {
///     .Continue(value) => value,
///     .Break(early) => return R.fromResidual(early)
/// }
/// ```
/// where R is the enclosing function's return type.
fn resolve_try_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    use kestrel_semantic_tree::expr::{CallArgument, MatchArm};
    use kestrel_semantic_tree::pattern::{EnumPatternBinding, Mutability, Pattern};

    let span = get_node_span(node, ctx.file_id);

    // Find the operand expression
    let operand_node = match node
        .children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
    {
        Some(n) => n,
        None => return Expression::error(span),
    };

    let operand = resolve_expression(&operand_node, ctx);

    // Create method call: operand.tryExtract()
    // This is a deferred method call that will be resolved during type inference
    let try_extract_call = Expression::deferred_method_call(
        operand,
        "tryExtract".to_string(),
        vec![],
        Ty::infer(span.clone()), // ControlFlow[Output, Early]
        span.clone(),
    );

    // Create locals for the bound variables in each arm
    // Push scope for continue arm
    ctx.local_scope.push_scope();

    // Create a single inference type for the value binding.
    // This type will be shared between the local binding, pattern, and body reference
    // so that type inference can connect them properly.
    let value_ty = Ty::infer(span.clone());

    // Bind 'value' local for .Continue(value) pattern
    let value_local_id = ctx.local_scope.bind(
        "$try_value".to_string(), // Use synthetic name to avoid conflicts
        value_ty.clone(),
        false,
        span.clone(),
    );

    // Create pattern: .Continue(value)
    let value_binding_pattern = Pattern::local(
        value_local_id,
        Mutability::Immutable,
        "$try_value".to_string(),
        value_ty.clone(),
        span.clone(),
    );
    let continue_binding = EnumPatternBinding::unlabeled(value_binding_pattern, span.clone());
    let continue_pattern = Pattern::unresolved_enum_variant(
        "Continue".to_string(),
        vec![continue_binding],
        span.clone(),
    );

    // Body for continue arm: just reference the value
    // Use the same type as the binding pattern so type inference connects them
    let continue_body = Expression::local_ref(value_local_id, value_ty, false, span.clone());
    let continue_arm = MatchArm::new(continue_pattern, continue_body, span.clone());

    ctx.local_scope.pop_scope();

    // Push scope for break arm
    ctx.local_scope.push_scope();

    // Create a single inference type for the early binding.
    // This type will be shared between the local binding, pattern, and body reference
    // so that type inference can connect them properly.
    let early_ty = Ty::infer(span.clone());

    // Bind 'early' local for .Break(early) pattern
    let early_local_id = ctx.local_scope.bind(
        "$try_early".to_string(),
        early_ty.clone(),
        false,
        span.clone(),
    );

    // Create pattern: .Break(early)
    let early_binding_pattern = Pattern::local(
        early_local_id,
        Mutability::Immutable,
        "$try_early".to_string(),
        early_ty.clone(),
        span.clone(),
    );
    let break_binding = EnumPatternBinding::unlabeled(early_binding_pattern, span.clone());
    let break_pattern =
        Pattern::unresolved_enum_variant("Break".to_string(), vec![break_binding], span.clone());

    // Body for break arm: return R.fromResidual(early)
    // R is the enclosing function's return type
    let return_ty = ctx
        .function_return_type()
        .unwrap_or_else(|| Ty::error(span.clone()));

    // Reference to the early local - use the same type as the binding pattern
    let early_ref = Expression::local_ref(early_local_id, early_ty, false, span.clone());

    // Create argument: residual: early
    let from_residual_arg = CallArgument::labeled("residual".to_string(), early_ref, span.clone());

    // Create deferred static call: R.fromResidual(early)
    // Type inference will resolve this to the actual static method
    let from_residual_call = Expression::deferred_static_call(
        return_ty.clone(),
        "fromResidual".to_string(),
        vec![from_residual_arg],
        return_ty, // Result type is also R (Self)
        span.clone(),
    );

    let break_body = Expression::return_expr(Some(from_residual_call), span.clone());
    let break_arm = MatchArm::new(break_pattern, break_body, span.clone());

    ctx.local_scope.pop_scope();

    // Create the match expression
    Expression::match_expr(try_extract_call, vec![continue_arm, break_arm], span)
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
            },
            SyntaxKind::VariableDeclaration => {
                if let Some(stmt) = super::statements::resolve_variable_declaration(&child, ctx) {
                    statements.push(stmt);
                }
            },
            SyntaxKind::Expression => {
                // Expressions in loop body become expression statements
                let expr = resolve_expression(&child, ctx);
                let stmt_span = get_node_span(&child, ctx.file_id);
                statements.push(Statement::expr(expr, stmt_span));
            },
            _ if is_expression_kind(child.kind()) => {
                // Handle bare expression kinds
                let expr = resolve_expression(&child, ctx);
                let stmt_span = get_node_span(&child, ctx.file_id);
                statements.push(Statement::expr(expr, stmt_span));
            },
            _ => {},
        }
    }

    ctx.local_scope.pop_scope();
    statements
}

/// Extract label info from a loop expression (while/loop).
/// The label appears as a LoopLabel child before the loop keyword.
fn extract_loop_label(node: &SyntaxNode, file_id: usize) -> Option<LabelInfo> {
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
                        span: Span::new(file_id, start..end),
                    }
                })
        })
}

/// Extract label info from a break/continue expression.
/// The label appears as an Identifier token after the keyword.
fn extract_break_continue_label(node: &SyntaxNode, file_id: usize) -> Option<LabelInfo> {
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
                span: Span::new(file_id, start..end),
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
            let idx_span = Span::new(
                ctx.file_id,
                text_range.start().into()..text_range.end().into(),
            );
            let index_value = token.text().parse::<usize>().unwrap_or(0);
            (index_value, idx_span)
        },
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
        },
        None => {
            // Not a tuple type
            let error = TupleIndexOnNonTupleError {
                span: span.clone(),
                index,
                base_type: base_ty.to_string(),
            };
            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            Expression::error(span)
        },
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
    if let Some(expr) = tail_expr
        && expression_references_local(expr, it_local_id)
    {
        return true;
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
        },
        StatementKind::Expr(expr) => expression_references_local(expr, local_id),
        StatementKind::GuardLet {
            conditions,
            else_block,
        } => {
            // Check all conditions
            for condition in conditions {
                match condition {
                    IfCondition::Expr(expr) => {
                        if expression_references_local(expr, local_id) {
                            return true;
                        }
                    },
                    IfCondition::Let { value, .. } => {
                        if expression_references_local(value, local_id) {
                            return true;
                        }
                    },
                }
            }
            // Check else block statements
            for else_stmt in &else_block.statements {
                if statement_references_local(else_stmt, local_id) {
                    return true;
                }
            }
            if let Some(yield_expr) = &else_block.yield_expr
                && expression_references_local(yield_expr, local_id)
            {
                return true;
            }
            false
        },
        StatementKind::Deinit {
            local_id: deinit_id,
            ..
        } => {
            // The deinit statement references the variable being deinited
            *deinit_id == local_id
        },
    }
}

/// Check if an expression references a specific local (recursively).
fn expression_references_local(
    expr: &Expression,
    local_id: kestrel_semantic_tree::symbol::local::LocalId,
) -> bool {
    use kestrel_semantic_tree::expr::{ElseBranch, ExprKind, IfCondition};

    match &expr.kind {
        // Direct reference to the local
        ExprKind::LocalRef(id) => *id == local_id,

        // Recursively check compound expressions
        ExprKind::Array(elements) | ExprKind::Tuple(elements) => elements
            .iter()
            .any(|e| expression_references_local(e, local_id)),

        ExprKind::Grouping(inner) => expression_references_local(inner, local_id),

        ExprKind::FieldAccess { object, .. } => expression_references_local(object, local_id),

        ExprKind::TupleIndex { tuple, .. } => expression_references_local(tuple, local_id),

        ExprKind::MethodRef { receiver, .. } => expression_references_local(receiver, local_id),

        ExprKind::PrimitiveMethodRef { receiver, .. } => {
            expression_references_local(receiver, local_id)
        },

        ExprKind::Call {
            callee, arguments, ..
        } => {
            expression_references_local(callee, local_id)
                || arguments
                    .iter()
                    .any(|arg| expression_references_local(&arg.value, local_id))
        },

        ExprKind::PrimitiveMethodCall {
            receiver,
            arguments,
            ..
        } => {
            expression_references_local(receiver, local_id)
                || arguments
                    .iter()
                    .any(|arg| expression_references_local(&arg.value, local_id))
        },

        ExprKind::DeferredMethodCall {
            receiver,
            arguments,
            ..
        } => {
            expression_references_local(receiver, local_id)
                || arguments
                    .iter()
                    .any(|arg| expression_references_local(&arg.value, local_id))
        },

        ExprKind::DeferredStaticCall { arguments, .. } => arguments
            .iter()
            .any(|arg| expression_references_local(&arg.value, local_id)),

        ExprKind::ImplicitStructInit { arguments, .. } => arguments
            .iter()
            .any(|arg| expression_references_local(&arg.value, local_id)),

        ExprKind::DelegatingInit { arguments, .. } => arguments
            .iter()
            .any(|arg| expression_references_local(&arg.value, local_id)),

        ExprKind::Assignment { target, value } => {
            expression_references_local(target, local_id)
                || expression_references_local(value, local_id)
        },

        ExprKind::If {
            conditions,
            then_branch,
            then_value,
            else_branch,
        } => {
            // Check conditions
            for condition in conditions {
                match condition {
                    IfCondition::Expr(expr) => {
                        if expression_references_local(expr, local_id) {
                            return true;
                        }
                    },
                    IfCondition::Let { value, .. } => {
                        if expression_references_local(value, local_id) {
                            return true;
                        }
                    },
                }
            }

            for stmt in then_branch {
                if statement_references_local(stmt, local_id) {
                    return true;
                }
            }

            if let Some(then_val) = then_value
                && expression_references_local(then_val, local_id)
            {
                return true;
            }

            if let Some(else_br) = else_branch {
                match else_br {
                    ElseBranch::Block { statements, value } => {
                        for stmt in statements {
                            if statement_references_local(stmt, local_id) {
                                return true;
                            }
                        }
                        if let Some(val) = value
                            && expression_references_local(val, local_id)
                        {
                            return true;
                        }
                    },
                    ElseBranch::ElseIf(if_expr) => {
                        if expression_references_local(if_expr, local_id) {
                            return true;
                        }
                    },
                }
            }

            false
        },

        ExprKind::While {
            condition, body, ..
        } => {
            if expression_references_local(condition, local_id) {
                return true;
            }
            for stmt in body {
                if statement_references_local(stmt, local_id) {
                    return true;
                }
            }
            false
        },

        ExprKind::WhileLet {
            conditions, body, ..
        } => {
            // Check all conditions
            for condition in conditions {
                match condition {
                    IfCondition::Expr(expr) => {
                        if expression_references_local(expr, local_id) {
                            return true;
                        }
                    },
                    IfCondition::Let { value, .. } => {
                        if expression_references_local(value, local_id) {
                            return true;
                        }
                    },
                }
            }
            for stmt in body {
                if statement_references_local(stmt, local_id) {
                    return true;
                }
            }
            false
        },

        ExprKind::Loop { body, .. } => {
            for stmt in body {
                if statement_references_local(stmt, local_id) {
                    return true;
                }
            }
            false
        },

        ExprKind::Closure {
            body, tail_expr, ..
        } => {
            // Check nested closure body
            for stmt in body {
                if statement_references_local(stmt, local_id) {
                    return true;
                }
            }
            if let Some(tail) = tail_expr
                && expression_references_local(tail, local_id)
            {
                return true;
            }
            false
        },

        ExprKind::Return { value } => {
            if let Some(val) = value {
                expression_references_local(val, local_id)
            } else {
                false
            }
        },

        // Implicit member access - check arguments if present
        ExprKind::ImplicitMemberAccess { arguments, .. } => {
            if let Some(args) = arguments {
                args.iter()
                    .any(|arg| expression_references_local(&arg.value, local_id))
            } else {
                false
            }
        },

        ExprKind::Match { scrutinee, arms } => {
            expression_references_local(scrutinee, local_id)
                || arms.iter().any(|arm| {
                    arm.guard
                        .as_ref()
                        .map(|g| expression_references_local(g, local_id))
                        .unwrap_or(false)
                        || expression_references_local(&arm.body, local_id)
                })
        },

        ExprKind::Block { statements, value } => {
            for stmt in statements {
                if statement_references_local(stmt, local_id) {
                    return true;
                }
            }
            if let Some(val) = value
                && expression_references_local(val, local_id)
            {
                return true;
            }
            false
        },

        // Lang intrinsic calls - check arguments
        ExprKind::LangIntrinsic { arguments, .. } => arguments
            .iter()
            .any(|arg| expression_references_local(&arg.value, local_id)),

        // Subscript call - check receiver and arguments
        ExprKind::SubscriptCall {
            receiver,
            arguments,
            ..
        } => {
            expression_references_local(receiver, local_id)
                || arguments
                    .iter()
                    .any(|arg| expression_references_local(&arg.value, local_id))
        },

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
        | ExprKind::LangIntrinsicRef(_)
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
        let local_id = ctx
            .local_scope
            .bind("it".to_string(), it_ty.clone(), false, span.clone());
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
                ParamInfo::ImplicitIt {
                    it_type: Box::new(it_ty),
                },
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
    let captures = collect_captures(
        &body,
        tail_expr.as_ref(),
        closure_entry_depth,
        &ctx.local_scope,
    );

    Expression::closure(
        params,
        body,
        tail_expr,
        captures,
        it_was_used,
        implicit_param,
        closure_ty,
        span,
    )
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
                },
                None => (
                    kestrel_semantic_tree::ty::Ty::infer(param_span.clone()),
                    false,
                ),
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
            },
            SyntaxKind::VariableDeclaration => {
                if let Some(stmt) = super::statements::resolve_variable_declaration(child, ctx) {
                    statements.push(stmt);
                }
            },
            SyntaxKind::Expression => {
                // If last child and no semicolon, it's the trailing expression
                if is_last && !has_trailing_semicolon(child) {
                    tail_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let stmt_span = get_node_span(child, ctx.file_id);
                    statements.push(Statement::expr(expr, stmt_span));
                }
            },
            _ if is_expression_kind(child.kind()) => {
                // Handle bare expression kinds (not wrapped in Expression)
                if is_last {
                    tail_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let stmt_span = get_node_span(child, ctx.file_id);
                    statements.push(Statement::expr(expr, stmt_span));
                }
            },
            // Skip tokens like braces, 'in' keyword
            _ => {},
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
    let mut process_local_ref = |local_id: LocalId,
                                 _name: &str,
                                 ty: &kestrel_semantic_tree::ty::Ty,
                                 span: &kestrel_span::Span| {
        // Check if already captured
        if seen_ids.contains(&local_id) {
            return;
        }

        // Check if this local was declared before the closure scope
        // Variables at closure_entry_depth or below are from outer scopes
        if let Some(local_depth) = local_scope.scope_depth_of(local_id)
            && local_depth <= closure_entry_depth
        {
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
    F: FnMut(
        kestrel_semantic_tree::symbol::local::LocalId,
        &str,
        &kestrel_semantic_tree::ty::Ty,
        &kestrel_span::Span,
    ),
{
    use kestrel_semantic_tree::stmt::StatementKind;

    match &stmt.kind {
        StatementKind::Expr(expr) => {
            collect_captures_from_expression(expr, process);
        },
        StatementKind::Binding { value, .. } => {
            // Walk the initializer value if present
            if let Some(expr) = value {
                collect_captures_from_expression(expr, process);
            }
        },
        StatementKind::GuardLet {
            conditions,
            else_block,
        } => {
            // Walk all conditions
            for condition in conditions {
                match condition {
                    IfCondition::Expr(expr) => {
                        collect_captures_from_expression(expr, process);
                    },
                    IfCondition::Let { value, .. } => {
                        collect_captures_from_expression(value, process);
                    },
                }
            }
            // Walk else block
            for else_stmt in &else_block.statements {
                collect_captures_from_statement(else_stmt, process);
            }
            if let Some(yield_expr) = &else_block.yield_expr {
                collect_captures_from_expression(yield_expr, process);
            }
        },
        StatementKind::Deinit { .. } => {
            // Deinit statement doesn't contain expressions that could capture variables
        },
    }
}

/// Walk an expression to find LocalRef expressions.
fn collect_captures_from_expression<F>(expr: &Expression, process: &mut F)
where
    F: FnMut(
        kestrel_semantic_tree::symbol::local::LocalId,
        &str,
        &kestrel_semantic_tree::ty::Ty,
        &kestrel_span::Span,
    ),
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
        },

        // Recursively walk compound expressions
        ExprKind::Grouping(inner) => {
            collect_captures_from_expression(inner, process);
        },
        ExprKind::Call {
            callee, arguments, ..
        } => {
            collect_captures_from_expression(callee, process);
            for arg in arguments {
                collect_captures_from_expression(&arg.value, process);
            }
        },
        ExprKind::PrimitiveMethodCall {
            receiver,
            arguments,
            ..
        } => {
            collect_captures_from_expression(receiver, process);
            for arg in arguments {
                collect_captures_from_expression(&arg.value, process);
            }
        },
        ExprKind::DeferredMethodCall {
            receiver,
            arguments,
            ..
        } => {
            collect_captures_from_expression(receiver, process);
            for arg in arguments {
                collect_captures_from_expression(&arg.value, process);
            }
        },
        ExprKind::DeferredStaticCall { arguments, .. } => {
            for arg in arguments {
                collect_captures_from_expression(&arg.value, process);
            }
        },
        ExprKind::MethodRef { receiver, .. } => {
            collect_captures_from_expression(receiver, process);
        },
        ExprKind::PrimitiveMethodRef { receiver, .. } => {
            collect_captures_from_expression(receiver, process);
        },
        ExprKind::FieldAccess { object, .. } => {
            collect_captures_from_expression(object, process);
        },
        ExprKind::TupleIndex { tuple, .. } => {
            collect_captures_from_expression(tuple, process);
        },
        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments {
                collect_captures_from_expression(&arg.value, process);
            }
        },
        ExprKind::DelegatingInit { arguments, .. } => {
            for arg in arguments {
                collect_captures_from_expression(&arg.value, process);
            }
        },
        ExprKind::Assignment { target, value } => {
            collect_captures_from_expression(target, process);
            collect_captures_from_expression(value, process);
        },
        ExprKind::Tuple(elements) => {
            for elem in elements {
                collect_captures_from_expression(elem, process);
            }
        },
        ExprKind::Array(elements) => {
            for elem in elements {
                collect_captures_from_expression(elem, process);
            }
        },
        ExprKind::If {
            conditions,
            then_branch,
            then_value,
            else_branch,
        } => {
            // Collect from conditions
            for condition in conditions {
                match condition {
                    IfCondition::Expr(expr) => {
                        collect_captures_from_expression(expr, process);
                    },
                    IfCondition::Let { value, .. } => {
                        collect_captures_from_expression(value, process);
                    },
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
        },
        ExprKind::While {
            condition,
            body: while_body,
            ..
        } => {
            collect_captures_from_expression(condition, process);
            for stmt in while_body {
                collect_captures_from_statement(stmt, process);
            }
        },
        ExprKind::WhileLet {
            conditions,
            body: while_body,
            ..
        } => {
            // Walk all conditions
            for condition in conditions {
                match condition {
                    IfCondition::Expr(expr) => {
                        collect_captures_from_expression(expr, process);
                    },
                    IfCondition::Let { value, .. } => {
                        collect_captures_from_expression(value, process);
                    },
                }
            }
            for stmt in while_body {
                collect_captures_from_statement(stmt, process);
            }
        },
        ExprKind::Loop {
            body: loop_body, ..
        } => {
            for stmt in loop_body {
                collect_captures_from_statement(stmt, process);
            }
        },
        ExprKind::Break { .. } => {
            // Break doesn't have a value in this AST
        },
        ExprKind::Continue { .. } => {},
        ExprKind::Return { value } => {
            if let Some(val) = value {
                collect_captures_from_expression(val, process);
            }
        },
        ExprKind::Closure {
            body, tail_expr, ..
        } => {
            // For nested closures, we still walk their bodies to find captures
            // These will be captures from the outer closure's perspective
            for stmt in body {
                collect_captures_from_statement(stmt, process);
            }
            if let Some(tail) = tail_expr {
                collect_captures_from_expression(tail, process);
            }
        },

        // Implicit member access - check arguments if present
        ExprKind::ImplicitMemberAccess { arguments, .. } => {
            if let Some(args) = arguments {
                for arg in args {
                    collect_captures_from_expression(&arg.value, process);
                }
            }
        },

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
        },

        // Block expression - walk statements and value
        ExprKind::Block { statements, value } => {
            for stmt in statements {
                collect_captures_from_statement(stmt, process);
            }
            if let Some(val) = value {
                collect_captures_from_expression(val, process);
            }
        },

        // Lang intrinsic calls - walk arguments
        ExprKind::LangIntrinsic { arguments, .. } => {
            for arg in arguments {
                collect_captures_from_expression(&arg.value, process);
            }
        },

        // Subscript call - walk receiver and arguments
        ExprKind::SubscriptCall {
            receiver,
            arguments,
            ..
        } => {
            collect_captures_from_expression(receiver, process);
            for arg in arguments {
                collect_captures_from_expression(&arg.value, process);
            }
        },

        // Leaf nodes - no recursion needed
        ExprKind::Literal(_)
        | ExprKind::SymbolRef(_)
        | ExprKind::OverloadedRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::AssociatedTypeRef
        | ExprKind::EnumCase { .. }
        | ExprKind::LangIntrinsicRef(_)
        | ExprKind::Error => {},
    }
}

/// Walk an else branch to find LocalRef expressions.
fn collect_captures_from_else_branch<F>(
    else_branch: &kestrel_semantic_tree::expr::ElseBranch,
    process: &mut F,
) where
    F: FnMut(
        kestrel_semantic_tree::symbol::local::LocalId,
        &str,
        &kestrel_semantic_tree::ty::Ty,
        &kestrel_span::Span,
    ),
{
    match else_branch {
        kestrel_semantic_tree::expr::ElseBranch::Block { statements, value } => {
            for stmt in statements {
                collect_captures_from_statement(stmt, process);
            }
            if let Some(val) = value {
                collect_captures_from_expression(val, process);
            }
        },
        kestrel_semantic_tree::expr::ElseBranch::ElseIf(expr) => {
            collect_captures_from_expression(expr, process);
        },
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
    use super::patterns::resolve_pattern;
    use kestrel_semantic_tree::expr::MatchArm;

    let span = get_node_span(node, ctx.file_id);

    // Push a new scope for the arm (pattern bindings are local to the arm)
    ctx.local_scope.push_scope();

    // Find and resolve the pattern with the scrutinee type as expected type
    let pattern_node = node
        .children()
        .find(|c| super::patterns::is_pattern_kind(c.kind()))?;
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

    Some(if let Some(guard_expr) = guard {
        MatchArm::with_guard(pattern, guard_expr, body, span)
    } else {
        MatchArm::new(pattern, body, span)
    })
}

/// Resolve a match arm body, handling block syntax specially.
///
/// When the body is a block (closure syntax without explicit parameters, i.e., just `{ ... }`),
/// we resolve it as an inline block expression rather than a closure. This ensures that
/// pattern bindings from the match arm remain visible in the block without capture analysis.
fn resolve_match_arm_body(body_node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    // Unwrap Expression wrapper if present
    let inner_node = if body_node.kind() == SyntaxKind::Expression {
        body_node.children().next().unwrap_or(body_node.clone())
    } else {
        body_node.clone()
    };

    // Check if it's a block (closure syntax without explicit parameters)
    if inner_node.kind() == SyntaxKind::ExprClosure {
        // Check if it has ClosureParams - if so, it's a real closure with explicit params
        let has_explicit_params = inner_node
            .children()
            .any(|c| c.kind() == SyntaxKind::ClosureParams);

        if !has_explicit_params {
            // It's a block (no params). Resolve the body inline.
            // Don't push a new scope - we're already in the match arm's scope.
            let (statements, tail_expr) = resolve_closure_body(&inner_node, ctx);
            let body_span = get_node_span(&inner_node, ctx.file_id);

            // If there are no statements and just a tail expression, return it directly
            if statements.is_empty() {
                if let Some(expr) = tail_expr {
                    return expr;
                }
                // Empty block body - return unit
                return Expression::unit(body_span);
            }

            // Create a block expression (NOT a closure).
            // Pattern bindings from the match arm remain visible.
            let result_ty = tail_expr
                .as_ref()
                .map(|e| e.ty.clone())
                .unwrap_or_else(|| Ty::unit(body_span.clone()));
            return Expression::block(statements, tail_expr, result_ty, body_span);
        }
    }

    // For all other cases (non-block, or closure with explicit params),
    // resolve normally using the exact same code path as before
    resolve_expression(body_node, ctx)
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
