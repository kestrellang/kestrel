//! Expression emission.
//!
//! Converts parsed [`ExprVariant`] values (and friends) into [`EventSink`]
//! events. The parser in `expr/mod.rs` produces the data; this module is the
//! single source of truth for how that data becomes syntax tree events.

use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxKind;

use crate::block::{BlockItem, CodeBlockData, ElseBlockItem, emit_code_block};
use crate::event::EventSink;
use crate::ty::emit_ty_variant;

use super::data::{
    ArgumentListData, CallArg, ClosureParamsData, ElseClause, ExprVariant, IfCondition, LabelData,
    MatchArmData, PathSegmentData, TypeArgsData,
};

/// Emit events for any expression variant
pub fn emit_expr_variant(sink: &mut EventSink, variant: &ExprVariant) {
    match variant {
        ExprVariant::Unit(lparen, rparen) => {
            emit_unit_expr(sink, lparen.clone(), rparen.clone());
        },
        ExprVariant::Integer(span) => {
            emit_integer_expr(sink, span.clone());
        },
        ExprVariant::Float(span) => {
            emit_float_expr(sink, span.clone());
        },
        ExprVariant::String(span) => {
            emit_string_expr(sink, span.clone());
        },
        ExprVariant::InterpolatedString(span) => {
            emit_interpolated_string_expr(sink, span.clone());
        },
        ExprVariant::RawString(span) => {
            emit_raw_string_expr(sink, span.clone());
        },
        ExprVariant::Char(span) => {
            emit_char_expr(sink, span.clone());
        },
        ExprVariant::Bool(span) => {
            emit_bool_expr(sink, span.clone());
        },
        ExprVariant::Null(span) => {
            emit_null_expr(sink, span.clone());
        },
        ExprVariant::Array(lbracket, elements, commas, rbracket) => {
            emit_array_expr(sink, lbracket.clone(), elements, commas, rbracket.clone());
        },
        ExprVariant::Dictionary {
            lbracket,
            entries,
            commas,
            rbracket,
        } => {
            emit_dictionary_expr(sink, lbracket.clone(), entries, commas, rbracket.clone());
        },
        ExprVariant::Tuple(lparen, elements, commas, rparen) => {
            emit_tuple_expr(sink, lparen.clone(), elements, commas, rparen.clone());
        },
        ExprVariant::Grouping(lparen, inner, rparen) => {
            emit_grouping_expr(sink, lparen.clone(), inner, rparen.clone());
        },
        ExprVariant::Path { segments, dots } => {
            emit_path_expr(sink, segments, dots);
        },
        ExprVariant::MemberAccess {
            base,
            dot,
            member,
            type_args,
        } => {
            emit_member_access_expr(sink, base, dot.clone(), member.as_ref(), type_args.as_ref());
        },
        ExprVariant::TupleIndex { base, dot, index } => {
            emit_tuple_index_expr(sink, base, dot.clone(), index.clone());
        },
        ExprVariant::Unary(tok, span, operand) => {
            emit_unary_expr(sink, tok.clone(), span.clone(), operand);
        },
        ExprVariant::Call {
            callee,
            lparen,
            arguments,
            commas,
            rparen,
        } => {
            emit_call_expr(
                sink,
                callee,
                lparen.as_ref(),
                arguments,
                commas,
                rparen.as_ref(),
            );
        },
        ExprVariant::Assignment { lhs, equals, rhs } => {
            emit_assignment_expr(sink, lhs, equals.clone(), rhs);
        },
        ExprVariant::CompoundAssignment {
            lhs,
            operator,
            operator_span,
            rhs,
        } => {
            emit_compound_assignment_expr(sink, lhs, operator.clone(), operator_span.clone(), rhs);
        },
        ExprVariant::Postfix {
            operand,
            operator,
            operator_span,
        } => {
            emit_postfix_expr(sink, operand, operator.clone(), operator_span.clone());
        },
        ExprVariant::Binary {
            lhs,
            operator,
            operator_span,
            rhs,
        } => {
            emit_binary_expr(sink, lhs, operator.clone(), operator_span.clone(), rhs);
        },
        ExprVariant::If {
            if_span,
            conditions,
            then_block,
            else_clause,
        } => {
            emit_if_expr(
                sink,
                if_span.clone(),
                conditions,
                then_block,
                else_clause.as_ref(),
            );
        },
        ExprVariant::While {
            label,
            while_span,
            condition,
            body,
        } => {
            emit_while_expr(sink, label.as_ref(), while_span.clone(), condition, body);
        },
        ExprVariant::WhileLet {
            label,
            while_span,
            conditions,
            body,
        } => {
            emit_while_let_expr(sink, label.as_ref(), while_span.clone(), conditions, body);
        },
        ExprVariant::Loop {
            label,
            loop_span,
            body,
        } => {
            emit_loop_expr(sink, label.as_ref(), loop_span.clone(), body);
        },
        ExprVariant::For {
            label,
            for_span,
            pattern,
            in_span,
            iterable,
            body,
        } => {
            emit_for_expr(
                sink,
                label.as_ref(),
                for_span.clone(),
                pattern,
                in_span.clone(),
                iterable,
                body,
            );
        },
        ExprVariant::Break { break_span, label } => {
            emit_break_expr(sink, break_span.clone(), label.as_ref());
        },
        ExprVariant::Continue {
            continue_span,
            label,
        } => {
            emit_continue_expr(sink, continue_span.clone(), label.as_ref());
        },
        ExprVariant::Return { return_span, value } => {
            emit_return_expr(sink, return_span.clone(), value.as_deref());
        },
        ExprVariant::Throw { throw_span, value } => {
            emit_throw_expr(sink, throw_span.clone(), value.as_deref());
        },
        ExprVariant::Try { try_span, operand } => {
            emit_try_expr(sink, try_span.clone(), operand);
        },
        ExprVariant::Closure {
            lbrace,
            params,
            in_span,
            body,
            rbrace,
        } => {
            emit_closure_expr(sink, lbrace.clone(), params, in_span, body, rbrace.clone());
        },
        ExprVariant::ImplicitMemberAccess {
            dot,
            member,
            arguments,
        } => {
            emit_implicit_member_access_expr(sink, dot.clone(), member.clone(), arguments.as_ref());
        },
        ExprVariant::Match {
            match_span,
            scrutinee,
            lbrace,
            arms,
            rbrace,
        } => {
            emit_match_expr(
                sink,
                match_span.clone(),
                scrutinee,
                lbrace.clone(),
                arms,
                rbrace.clone(),
            );
        },
    }
}

/// Emit events for a unit expression
pub fn emit_unit_expr(sink: &mut EventSink, lparen: Span, rparen: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprUnit);
    sink.add_token(SyntaxKind::LParen, lparen);
    sink.add_token(SyntaxKind::RParen, rparen);
    sink.finish_node();
    sink.finish_node();
}

fn emit_integer_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprInteger);
    sink.add_token(SyntaxKind::Integer, span);
    sink.finish_node();
    sink.finish_node();
}

fn emit_float_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprFloat);
    sink.add_token(SyntaxKind::Float, span);
    sink.finish_node();
    sink.finish_node();
}

fn emit_string_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprString);
    sink.add_token(SyntaxKind::String, span);
    sink.finish_node();
    sink.finish_node();
}

fn emit_interpolated_string_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprInterpolatedString);
    // For now, we emit the entire string as a single token.
    // The semantic phase (binder) will parse the interpolation parts.
    sink.add_token(SyntaxKind::String, span);
    sink.finish_node();
    sink.finish_node();
}

fn emit_char_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprChar);
    sink.add_token(SyntaxKind::Char, span);
    sink.finish_node();
    sink.finish_node();
}

fn emit_raw_string_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprRawString);
    sink.add_token(SyntaxKind::RawString, span);
    sink.finish_node();
    sink.finish_node();
}

fn emit_bool_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprBool);
    sink.add_token(SyntaxKind::Boolean, span);
    sink.finish_node();
    sink.finish_node();
}

fn emit_null_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprNull);
    sink.add_token(SyntaxKind::Null, span);
    sink.finish_node();
    sink.finish_node();
}

fn emit_array_expr(
    sink: &mut EventSink,
    lbracket: Span,
    elements: &[ExprVariant],
    commas: &[Span],
    rbracket: Span,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprArray);
    sink.add_token(SyntaxKind::LBracket, lbracket);
    for (i, element) in elements.iter().enumerate() {
        emit_expr_variant(sink, element);
        if i < commas.len() {
            sink.add_token(SyntaxKind::Comma, commas[i].clone());
        }
    }
    sink.add_token(SyntaxKind::RBracket, rbracket);
    sink.finish_node();
    sink.finish_node();
}

fn emit_dictionary_expr(
    sink: &mut EventSink,
    lbracket: Span,
    entries: &[(ExprVariant, Span, ExprVariant)], // (key, colon, value)
    commas: &[Span],
    rbracket: Span,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprDictionary);
    sink.add_token(SyntaxKind::LBracket, lbracket);
    for (i, (key, colon, value)) in entries.iter().enumerate() {
        sink.start_node(SyntaxKind::DictionaryEntry);
        emit_expr_variant(sink, key);
        sink.add_token(SyntaxKind::Colon, colon.clone());
        emit_expr_variant(sink, value);
        sink.finish_node();
        if i < commas.len() {
            sink.add_token(SyntaxKind::Comma, commas[i].clone());
        }
    }
    sink.add_token(SyntaxKind::RBracket, rbracket);
    sink.finish_node();
    sink.finish_node();
}

fn emit_tuple_expr(
    sink: &mut EventSink,
    lparen: Span,
    elements: &[ExprVariant],
    commas: &[Span],
    rparen: Span,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprTuple);
    sink.add_token(SyntaxKind::LParen, lparen);
    for (i, element) in elements.iter().enumerate() {
        emit_expr_variant(sink, element);
        if i < commas.len() {
            sink.add_token(SyntaxKind::Comma, commas[i].clone());
        }
    }
    sink.add_token_or_missing(SyntaxKind::RParen, rparen, ")");
    sink.finish_node();
    sink.finish_node();
}

fn emit_grouping_expr(sink: &mut EventSink, lparen: Span, inner: &ExprVariant, rparen: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprGrouping);
    sink.add_token(SyntaxKind::LParen, lparen);
    emit_expr_variant(sink, inner);
    sink.add_token_or_missing(SyntaxKind::RParen, rparen, ")");
    sink.finish_node();
    sink.finish_node();
}

fn emit_type_args(sink: &mut EventSink, type_args: &TypeArgsData) {
    sink.start_node(SyntaxKind::TypeArgumentList);
    sink.add_token(SyntaxKind::LBracket, type_args.lbracket.clone());
    for arg in type_args.args.iter() {
        emit_ty_variant(sink, arg);
    }
    sink.add_token(SyntaxKind::RBracket, type_args.rbracket.clone());
    sink.finish_node();
}

fn emit_path_expr(sink: &mut EventSink, segments: &[PathSegmentData], dots: &[Span]) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprPath);
    for (i, segment) in segments.iter().enumerate() {
        sink.add_token(SyntaxKind::Identifier, segment.name.clone());
        if let Some(ref type_args) = segment.type_args {
            emit_type_args(sink, type_args);
        }
        if i < dots.len() {
            sink.add_token(SyntaxKind::Dot, dots[i].clone());
        }
    }
    sink.finish_node();
    sink.finish_node();
}

fn emit_expr_variant_inner(sink: &mut EventSink, variant: &ExprVariant) {
    match variant {
        ExprVariant::Path { segments, dots } => {
            for (i, segment) in segments.iter().enumerate() {
                sink.add_token(SyntaxKind::Identifier, segment.name.clone());
                if let Some(ref type_args) = segment.type_args {
                    emit_type_args(sink, type_args);
                }
                if i < dots.len() {
                    sink.add_token(SyntaxKind::Dot, dots[i].clone());
                }
            }
        },
        ExprVariant::MemberAccess {
            base,
            dot,
            member,
            type_args,
        } => {
            emit_expr_variant_inner(sink, base);
            sink.add_token(SyntaxKind::Dot, dot.clone());
            match member {
                Some(span) => sink.add_token(SyntaxKind::Identifier, span.clone()),
                None => {
                    let at = Span::new(dot.file_id, dot.end..dot.end);
                    sink.missing_token(SyntaxKind::Identifier, at);
                },
            }
            if let Some(type_args) = type_args {
                emit_type_args(sink, type_args);
            }
        },
        ExprVariant::TupleIndex { .. } => emit_expr_variant(sink, variant),
        _ => emit_expr_variant(sink, variant),
    }
}

fn emit_member_access_expr(
    sink: &mut EventSink,
    base: &ExprVariant,
    dot: Span,
    member: Option<&Span>,
    type_args: Option<&TypeArgsData>,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprPath);
    emit_expr_variant_inner(sink, base);
    let dot_end = dot.end;
    let dot_file_id = dot.file_id;
    sink.add_token(SyntaxKind::Dot, dot);
    match member {
        Some(span) => sink.add_token(SyntaxKind::Identifier, span.clone()),
        None => {
            let at = Span::new(dot_file_id, dot_end..dot_end);
            sink.missing_token(SyntaxKind::Identifier, at);
        },
    }
    if let Some(type_args) = type_args {
        emit_type_args(sink, type_args);
    }
    sink.finish_node();
    sink.finish_node();
}

fn emit_tuple_index_expr(sink: &mut EventSink, base: &ExprVariant, dot: Span, index: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprTupleIndex);
    emit_expr_variant(sink, base);
    sink.add_token(SyntaxKind::Dot, dot);
    sink.add_token(SyntaxKind::Integer, index);
    sink.finish_node();
    sink.finish_node();
}

fn emit_unary_expr(sink: &mut EventSink, tok: Token, span: Span, operand: &ExprVariant) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprUnary);
    sink.add_token(SyntaxKind::from(tok), span);
    emit_expr_variant(sink, operand);
    sink.finish_node();
    sink.finish_node();
}

fn emit_call_expr(
    sink: &mut EventSink,
    callee: &ExprVariant,
    lparen: Option<&Span>,
    arguments: &[CallArg],
    commas: &[Span],
    rparen: Option<&Span>,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprCall);
    emit_expr_variant(sink, callee);
    sink.start_node(SyntaxKind::ArgumentList);
    if let Some(lp) = lparen {
        sink.add_token(SyntaxKind::LParen, lp.clone());
    }
    for (i, arg) in arguments.iter().enumerate() {
        sink.start_node(SyntaxKind::Argument);
        if let (Some(label), Some(colon)) = (&arg.label, &arg.colon) {
            sink.add_token(SyntaxKind::Identifier, label.clone());
            sink.add_token(SyntaxKind::Colon, colon.clone());
        }
        emit_expr_variant(sink, &arg.value);
        sink.finish_node();
        if i < commas.len() {
            sink.add_token(SyntaxKind::Comma, commas[i].clone());
        }
    }
    match rparen {
        Some(rp) => sink.add_token(SyntaxKind::RParen, rp.clone()),
        None => {
            // Phase-4 recovery: synthesize a zero-width `)` at the
            // current emit cursor. Pass `lparen.end` (or 0 as a fallback)
            // as the `at` position — `TreeBuilder::emit_trivia_until`
            // skips flushing when `at <= source_pos`, which is always
            // true once the args have been emitted, so the Missing
            // wrapper lands right after the last argument with no trivia
            // consumed. The trivia between the would-be `)` and whatever
            // follows stays available for the surrounding parser /
            // recovery to wrap correctly.
            let at_byte = lparen.map(|l| l.end).unwrap_or(0);
            let file_id = lparen.map(|l| l.file_id).unwrap_or(0);
            sink.missing_token(
                SyntaxKind::RParen,
                Span::new(file_id, at_byte..at_byte),
            );
        },
    }
    sink.finish_node();
    sink.finish_node();
    sink.finish_node();
}

fn emit_assignment_expr(sink: &mut EventSink, lhs: &ExprVariant, equals: Span, rhs: &ExprVariant) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprAssignment);
    emit_expr_variant(sink, lhs);
    sink.add_token(SyntaxKind::Equals, equals);
    emit_expr_variant(sink, rhs);
    sink.finish_node();
    sink.finish_node();
}

fn emit_compound_assignment_expr(
    sink: &mut EventSink,
    lhs: &ExprVariant,
    operator: Token,
    operator_span: Span,
    rhs: &ExprVariant,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprCompoundAssignment);
    emit_expr_variant(sink, lhs);
    sink.add_token(SyntaxKind::from(operator), operator_span);
    emit_expr_variant(sink, rhs);
    sink.finish_node();
    sink.finish_node();
}

fn emit_postfix_expr(
    sink: &mut EventSink,
    operand: &ExprVariant,
    operator: Token,
    operator_span: Span,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprPostfix);
    emit_expr_variant(sink, operand);
    sink.add_token(SyntaxKind::from(operator), operator_span);
    sink.finish_node();
    sink.finish_node();
}

fn emit_binary_expr(
    sink: &mut EventSink,
    lhs: &ExprVariant,
    operator: Token,
    operator_span: Span,
    rhs: &ExprVariant,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprBinary);
    emit_expr_variant(sink, lhs);
    sink.add_token(SyntaxKind::from(operator), operator_span);
    emit_expr_variant(sink, rhs);
    sink.finish_node();
    sink.finish_node();
}

/// Emit a single condition (either a let-binding or a boolean expression)
/// Used by if-let, while-let, and guard-let chains.
/// The `condition_node_kind` parameter specifies the syntax kind for let conditions
/// (e.g., IfLetCondition, WhileLetCondition, GuardLetCondition).
pub fn emit_if_condition(
    sink: &mut EventSink,
    condition: &IfCondition,
    condition_node_kind: SyntaxKind,
) {
    match condition {
        IfCondition::Expr(expr) => {
            emit_expr_variant(sink, expr);
        },
        IfCondition::Let {
            let_span,
            pattern,
            equals_span,
            value,
        } => {
            sink.start_node(condition_node_kind);
            sink.add_token(SyntaxKind::Let, let_span.clone());
            crate::pattern::emit_pattern_variant(sink, pattern);
            sink.add_token(SyntaxKind::Equals, equals_span.clone());
            emit_expr_variant(sink, value);
            sink.finish_node();
        },
    }
}

fn emit_if_expr(
    sink: &mut EventSink,
    if_span: Span,
    conditions: &[IfCondition],
    then_block: &CodeBlockData,
    else_clause: Option<&ElseClause>,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprIf);
    sink.add_token(SyntaxKind::If, if_span);
    // Emit each condition
    for (i, condition) in conditions.iter().enumerate() {
        emit_if_condition(sink, condition, SyntaxKind::IfLetCondition);
        // Add comma between conditions (but not after last)
        if i < conditions.len() - 1 {
            // Note: We don't track comma spans in the parsed data,
            // so we skip emitting commas. The tree structure is still correct.
        }
    }
    emit_code_block(sink, then_block);
    if let Some(else_clause) = else_clause {
        sink.start_node(SyntaxKind::ElseClause);
        match else_clause {
            ElseClause::Block { else_span, block } => {
                sink.add_token(SyntaxKind::Else, else_span.clone());
                emit_code_block(sink, block);
            },
            ElseClause::ElseIf { else_span, if_expr } => {
                sink.add_token(SyntaxKind::Else, else_span.clone());
                emit_expr_variant(sink, if_expr);
            },
        }
        sink.finish_node();
    }
    sink.finish_node();
    sink.finish_node();
}

fn emit_match_expr(
    sink: &mut EventSink,
    match_span: Span,
    scrutinee: &ExprVariant,
    lbrace: Span,
    arms: &[MatchArmData],
    rbrace: Span,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprMatch);
    sink.add_token(SyntaxKind::Match, match_span);
    emit_expr_variant(sink, scrutinee);
    sink.add_token(SyntaxKind::LBrace, lbrace);
    for arm in arms {
        sink.start_node(SyntaxKind::MatchArm);
        crate::pattern::emit_pattern_variant(sink, &arm.pattern);
        if let Some(guard) = &arm.guard {
            sink.start_node(SyntaxKind::MatchArmGuard);
            sink.add_token(SyntaxKind::If, guard.if_span.clone());
            emit_expr_variant(sink, &guard.condition);
            sink.finish_node();
        }
        sink.add_token(SyntaxKind::FatArrow, arm.fat_arrow.clone());
        emit_expr_variant(sink, &arm.body);
        sink.finish_node();
    }
    sink.add_token(SyntaxKind::RBrace, rbrace);
    sink.finish_node();
    sink.finish_node();
}

fn emit_while_expr(
    sink: &mut EventSink,
    label: Option<&LabelData>,
    while_span: Span,
    condition: &ExprVariant,
    body: &CodeBlockData,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprWhile);
    if let Some(label_data) = label {
        sink.start_node(SyntaxKind::LoopLabel);
        sink.add_token(SyntaxKind::Identifier, label_data.name.clone());
        sink.add_token(SyntaxKind::Colon, label_data.colon.clone());
        sink.finish_node();
    }
    sink.add_token(SyntaxKind::While, while_span);
    emit_expr_variant(sink, condition);
    emit_code_block(sink, body);
    sink.finish_node();
    sink.finish_node();
}

fn emit_while_let_expr(
    sink: &mut EventSink,
    label: Option<&LabelData>,
    while_span: Span,
    conditions: &[IfCondition],
    body: &CodeBlockData,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprWhile);
    if let Some(label_data) = label {
        sink.start_node(SyntaxKind::LoopLabel);
        sink.add_token(SyntaxKind::Identifier, label_data.name.clone());
        sink.add_token(SyntaxKind::Colon, label_data.colon.clone());
        sink.finish_node();
    }
    sink.add_token(SyntaxKind::While, while_span);
    // Emit each condition in the chain
    for condition in conditions {
        emit_if_condition(sink, condition, SyntaxKind::WhileLetCondition);
    }
    emit_code_block(sink, body);
    sink.finish_node();
    sink.finish_node();
}

fn emit_loop_expr(
    sink: &mut EventSink,
    label: Option<&LabelData>,
    loop_span: Span,
    body: &CodeBlockData,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprLoop);
    if let Some(label_data) = label {
        sink.start_node(SyntaxKind::LoopLabel);
        sink.add_token(SyntaxKind::Identifier, label_data.name.clone());
        sink.add_token(SyntaxKind::Colon, label_data.colon.clone());
        sink.finish_node();
    }
    sink.add_token(SyntaxKind::Loop, loop_span);
    emit_code_block(sink, body);
    sink.finish_node();
    sink.finish_node();
}

fn emit_for_expr(
    sink: &mut EventSink,
    label: Option<&LabelData>,
    for_span: Span,
    pattern: &crate::pattern::PatternVariant,
    in_span: Span,
    iterable: &ExprVariant,
    body: &CodeBlockData,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprFor);
    if let Some(label_data) = label {
        sink.start_node(SyntaxKind::LoopLabel);
        sink.add_token(SyntaxKind::Identifier, label_data.name.clone());
        sink.add_token(SyntaxKind::Colon, label_data.colon.clone());
        sink.finish_node();
    }
    sink.add_token(SyntaxKind::For, for_span);
    // Emit pattern wrapped in ForPattern node
    sink.start_node(SyntaxKind::ForPattern);
    crate::pattern::emit_pattern_variant(sink, pattern);
    sink.finish_node();
    sink.add_token(SyntaxKind::In, in_span);
    // Emit iterable wrapped in ForIterable node
    sink.start_node(SyntaxKind::ForIterable);
    emit_expr_variant(sink, iterable);
    sink.finish_node();
    emit_code_block(sink, body);
    sink.finish_node();
    sink.finish_node();
}

fn emit_break_expr(sink: &mut EventSink, break_span: Span, label: Option<&Span>) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprBreak);
    sink.add_token(SyntaxKind::Break, break_span);
    if let Some(label_span) = label {
        sink.add_token(SyntaxKind::Identifier, label_span.clone());
    }
    sink.finish_node();
    sink.finish_node();
}

fn emit_continue_expr(sink: &mut EventSink, continue_span: Span, label: Option<&Span>) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprContinue);
    sink.add_token(SyntaxKind::Continue, continue_span);
    if let Some(label_span) = label {
        sink.add_token(SyntaxKind::Identifier, label_span.clone());
    }
    sink.finish_node();
    sink.finish_node();
}

fn emit_return_expr(sink: &mut EventSink, return_span: Span, value: Option<&ExprVariant>) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprReturn);
    sink.add_token(SyntaxKind::Return, return_span);
    if let Some(val) = value {
        emit_expr_variant(sink, val);
    }
    sink.finish_node();
    sink.finish_node();
}

fn emit_throw_expr(sink: &mut EventSink, throw_span: Span, value: Option<&ExprVariant>) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprThrow);
    sink.add_token(SyntaxKind::Throw, throw_span);
    if let Some(val) = value {
        emit_expr_variant(sink, val);
    }
    sink.finish_node();
    sink.finish_node();
}

fn emit_try_expr(sink: &mut EventSink, try_span: Span, operand: &ExprVariant) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprTry);
    sink.add_token(SyntaxKind::Try, try_span);
    emit_expr_variant(sink, operand);
    sink.finish_node();
    sink.finish_node();
}

fn emit_closure_expr(
    sink: &mut EventSink,
    lbrace: Span,
    params: &Option<ClosureParamsData>,
    in_span: &Option<Span>,
    body: &[BlockItem],
    rbrace: Span,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprClosure);
    sink.add_token(SyntaxKind::LBrace, lbrace);
    if let Some(params_data) = params {
        sink.start_node(SyntaxKind::ClosureParams);
        sink.add_token(SyntaxKind::LParen, params_data.lparen.clone());
        for (i, param) in params_data.params.iter().enumerate() {
            if i > 0 && i <= params_data.commas.len() {
                sink.add_token(SyntaxKind::Comma, params_data.commas[i - 1].clone());
            }
            sink.start_node(SyntaxKind::ClosureParam);
            crate::pattern::emit_pattern_variant(sink, &param.pattern);
            if let Some(ref colon) = param.colon {
                sink.add_token(SyntaxKind::Colon, colon.clone());
            }
            if let Some(ref ty) = param.ty {
                emit_ty_variant(sink, ty);
            }
            sink.finish_node();
        }
        sink.add_token(SyntaxKind::RParen, params_data.rparen.clone());
        sink.finish_node();
    }
    if let Some(in_sp) = in_span {
        sink.add_token(SyntaxKind::In, in_sp.clone());
    }
    for item in body {
        emit_block_item(sink, item);
    }
    sink.add_token_or_missing(SyntaxKind::RBrace, rbrace, "}");
    sink.finish_node();
    sink.finish_node();
}

fn emit_block_item(sink: &mut EventSink, item: &BlockItem) {
    match item {
        BlockItem::Statement(stmt) => {
            use crate::stmt::emit_stmt_variant;
            emit_stmt_variant(sink, stmt);
        },
        BlockItem::StatementExpr(expr) => {
            emit_expr_variant(sink, expr);
        },
        BlockItem::TrailingExpression(expr) => {
            emit_expr_variant(sink, expr);
        },
        BlockItem::GuardLet(guard_data) => {
            // Guard-let in a closure/expression context
            use crate::stmt::emit_stmt_variant;

            sink.start_node(SyntaxKind::Statement);
            sink.start_node(SyntaxKind::GuardLetStatement);
            sink.add_token(SyntaxKind::Guard, guard_data.guard_span.clone());
            // Emit each condition in the chain
            for condition in &guard_data.conditions {
                emit_if_condition(sink, condition, SyntaxKind::GuardLetCondition);
            }
            sink.add_token(SyntaxKind::Else, guard_data.else_span.clone());

            sink.start_node(SyntaxKind::CodeBlock);
            sink.add_token(SyntaxKind::LBrace, guard_data.else_lbrace.clone());
            for else_item in &guard_data.else_items {
                match else_item {
                    ElseBlockItem::Statement(stmt) => {
                        emit_stmt_variant(sink, stmt);
                    },
                    ElseBlockItem::StatementExpr(expr) => {
                        sink.start_node(SyntaxKind::Statement);
                        sink.start_node(SyntaxKind::ExpressionStatement);
                        emit_expr_variant(sink, expr);
                        sink.finish_node();
                        sink.finish_node();
                    },
                    ElseBlockItem::TrailingExpression(expr) => {
                        emit_expr_variant(sink, expr);
                    },
                }
            }
            sink.add_token(SyntaxKind::RBrace, guard_data.else_rbrace.clone());
            sink.finish_node(); // CodeBlock

            sink.finish_node(); // GuardLetStatement
            sink.finish_node(); // Statement
        },
        BlockItem::Recovered(span) => {
            // Same recovery shape as `block::emit_code_block` — wrap the
            // skipped tokens in a `SyntaxKind::Error` node so the source
            // round-trips and the LSP can spot the broken stretch.
            sink.start_node(SyntaxKind::Error);
            sink.add_token(SyntaxKind::Error, span.clone());
            sink.finish_node();
        },
    }
}

fn emit_implicit_member_access_expr(
    sink: &mut EventSink,
    dot: Span,
    member: Span,
    arguments: Option<&ArgumentListData>,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprImplicitMemberAccess);
    sink.add_token(SyntaxKind::Dot, dot);
    sink.start_node(SyntaxKind::Name);
    sink.add_token(SyntaxKind::Identifier, member);
    sink.finish_node();
    if let Some(args) = arguments {
        sink.start_node(SyntaxKind::ArgumentList);
        sink.add_token(SyntaxKind::LParen, args.lparen.clone());
        for (i, arg) in args.arguments.iter().enumerate() {
            sink.start_node(SyntaxKind::Argument);
            if let (Some(label), Some(colon)) = (&arg.label, &arg.colon) {
                sink.add_token(SyntaxKind::Identifier, label.clone());
                sink.add_token(SyntaxKind::Colon, colon.clone());
            }
            emit_expr_variant(sink, &arg.value);
            sink.finish_node();
            if i < args.commas.len() {
                sink.add_token(SyntaxKind::Comma, args.commas[i].clone());
            }
        }
        sink.add_token(SyntaxKind::RParen, args.rparen.clone());
        sink.finish_node();
    }
    sink.finish_node();
    sink.finish_node();
}

/// Check if a string literal contains interpolation (`\(`)
fn string_contains_interpolation(source: &str, span: &Span) -> bool {
    if span.end > source.len() || span.start >= span.end {
        return false;
    }
    let text = &source[span.start..span.end];
    // Look for `\(` that is not escaped (i.e., not preceded by `\\`)
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\'
            && let Some(&next) = chars.peek()
        {
            if next == '(' {
                return true;
            }
            // Skip the escaped character
            chars.next();
        }
    }
    false
}

/// Transform an ExprVariant, converting String to InterpolatedString where source contains `\(`
///
/// This is a simple check - we only transform top-level strings in the parse result.
/// Nested strings in complex expressions will be handled by the semantic phase which
/// has full source access.
pub(super) fn maybe_convert_to_interpolated(source: &str, variant: ExprVariant) -> ExprVariant {
    match variant {
        ExprVariant::String(span) => {
            if string_contains_interpolation(source, &span) {
                ExprVariant::InterpolatedString(span)
            } else {
                ExprVariant::String(span)
            }
        },
        // For other variants, we don't recursively transform here.
        // The semantic phase will handle nested expressions.
        other => other,
    }
}
