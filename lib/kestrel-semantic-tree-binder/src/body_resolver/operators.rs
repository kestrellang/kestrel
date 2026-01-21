//! Operator expression resolution.
//!
//! This module handles resolving unary, binary, and postfix operators,
//! including Pratt parsing for proper precedence handling and desugaring
//! operators into method calls on primitive types.

use std::sync::LazyLock;

use kestrel_semantic_tree::expr::{CallArgument, Expression, PrimitiveMethod};
use kestrel_semantic_tree::operators::{BinaryOp, InfixAction, OperatorRegistry, UnaryOp};
use kestrel_semantic_tree::ty::{FloatBits, IntBits, Ty, TyKind};
use kestrel_span::Span;
use kestrel_syntax_tree::utils::get_node_span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use super::context::BodyResolutionContext;
use super::expressions::resolve_expression;
use crate::diagnostics::OperatorOnLangIntrinsicType;

/// Global operator registry used for Pratt parsing.
static OPERATOR_REGISTRY: LazyLock<OperatorRegistry> = LazyLock::new(OperatorRegistry::new);

/// Resolve a prefix unary expression: -expr, !expr, not expr
pub fn resolve_unary_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Find the operator token
    let operator_token = node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| {
            matches!(
                t.kind(),
                SyntaxKind::Minus | SyntaxKind::Bang | SyntaxKind::Not
            )
        });

    let Some(op_token) = operator_token else {
        return Expression::error(span);
    };

    let op_span: Span = {
        let range = op_token.text_range();
        Span::new(span.file_id, range.start().into()..range.end().into())
    };

    // Determine the unary operator
    let op = match op_token.kind() {
        SyntaxKind::Minus => UnaryOp::Neg,
        SyntaxKind::Bang => UnaryOp::BitNot,
        SyntaxKind::Not => UnaryOp::LogicalNot,
        _ => return Expression::error(span),
    };

    // Find and resolve the operand expression
    let operand_node = node.children().find(|c| c.kind() == SyntaxKind::Expression);
    let Some(operand_node) = operand_node else {
        return Expression::error(span);
    };
    let operand = resolve_expression(&operand_node, ctx);

    // Desugar to method call: operand.method_name()
    desugar_unary_op(op, operand, op_span, span, ctx)
}

/// Resolve a postfix expression: expr!
pub fn resolve_postfix_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Find the operator token (!)
    let operator_token = node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Bang);

    let Some(op_token) = operator_token else {
        return Expression::error(span);
    };

    let op_span: Span = {
        let range = op_token.text_range();
        Span::new(span.file_id, range.start().into()..range.end().into())
    };

    // Find and resolve the operand expression
    let operand_node = node.children().find(|c| c.kind() == SyntaxKind::Expression);
    let Some(operand_node) = operand_node else {
        return Expression::error(span);
    };
    let operand = resolve_expression(&operand_node, ctx);

    // Desugar to method call: operand.unwrap()
    desugar_unary_op(UnaryOp::Unwrap, operand, op_span, span, ctx)
}

/// Resolve a binary expression: a + b
/// Uses Pratt parsing to handle operator precedence.
pub fn resolve_binary_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Collect all operands and operators from the flat binary tree
    let (operands, operators) = collect_binary_operands(node, ctx);

    if operands.is_empty() {
        return Expression::error(span);
    }

    if operands.len() == 1 {
        return operands.into_iter().next().unwrap();
    }

    // Apply Pratt parsing to restructure according to precedence
    pratt_parse(operands, operators, span, ctx)
}

/// Collect all operands and operators from a potentially nested binary expression tree.
/// Returns (operands, operators) where operators[i] is between operands[i] and operands[i+1].
fn collect_binary_operands(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> (Vec<Expression>, Vec<(SyntaxKind, Span)>) {
    let mut operands = Vec::new();
    let mut operators = Vec::new();

    // The tree structure is: ExprBinary { Expression, operator_token, Expression }
    // where each Expression may itself be an ExprBinary

    let children: Vec<_> = node.children().collect();

    // Should have 2 Expression children
    let expr_children: Vec<_> = children
        .iter()
        .filter(|c| c.kind() == SyntaxKind::Expression)
        .collect();

    if expr_children.len() != 2 {
        // Fallback: just resolve what we can
        for child in children.iter() {
            if child.kind() == SyntaxKind::Expression {
                operands.push(resolve_expression(child, ctx));
            }
        }
        return (operands, operators);
    }

    // Get the operator token between them
    let op_token = node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| is_binary_operator_token(t.kind()));

    let op_info = op_token.map(|t| {
        let range = t.text_range();
        let span: Span = Span::new(ctx.file_id, range.start().into()..range.end().into());
        (t.kind(), span)
    });

    // Recursively collect from left operand
    let lhs_node = expr_children[0];
    let (lhs_operands, lhs_ops) = if has_binary_child(lhs_node) {
        let binary_child = lhs_node
            .children()
            .find(|c| c.kind() == SyntaxKind::ExprBinary)
            .unwrap();
        collect_binary_operands(&binary_child, ctx)
    } else {
        (vec![resolve_expression(lhs_node, ctx)], vec![])
    };

    operands.extend(lhs_operands);
    operators.extend(lhs_ops);

    // Add the operator
    if let Some(op) = op_info {
        operators.push(op);
    }

    // Recursively collect from right operand
    let rhs_node = expr_children[1];
    let (rhs_operands, rhs_ops) = if has_binary_child(rhs_node) {
        let binary_child = rhs_node
            .children()
            .find(|c| c.kind() == SyntaxKind::ExprBinary)
            .unwrap();
        collect_binary_operands(&binary_child, ctx)
    } else {
        (vec![resolve_expression(rhs_node, ctx)], vec![])
    };

    operands.extend(rhs_operands);
    operators.extend(rhs_ops);

    (operands, operators)
}

/// Check if an Expression node contains a direct ExprBinary child
fn has_binary_child(node: &SyntaxNode) -> bool {
    node.children().any(|c| c.kind() == SyntaxKind::ExprBinary)
}

/// Check if a SyntaxKind is a binary operator token
fn is_binary_operator_token(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Plus
            | SyntaxKind::Minus
            | SyntaxKind::Star
            | SyntaxKind::Slash
            | SyntaxKind::Percent
            | SyntaxKind::Ampersand
            | SyntaxKind::Pipe
            | SyntaxKind::Caret
            | SyntaxKind::LessLess
            | SyntaxKind::GreaterGreater
            | SyntaxKind::Less
            | SyntaxKind::Greater
            | SyntaxKind::LessEquals
            | SyntaxKind::GreaterEquals
            | SyntaxKind::EqualsEquals
            | SyntaxKind::BangEquals
            | SyntaxKind::And
            | SyntaxKind::Or
            | SyntaxKind::QuestionQuestion
            | SyntaxKind::DotDotEquals
            | SyntaxKind::DotDotLess
    )
}

/// Apply Pratt parsing to a flat list of operands and operators.
fn pratt_parse(
    mut operands: Vec<Expression>,
    operators: Vec<(SyntaxKind, Span)>,
    full_span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    if operands.is_empty() {
        return Expression::error(full_span);
    }

    if operators.is_empty() {
        return operands.pop().unwrap();
    }

    // Use a simple precedence-climbing approach
    pratt_parse_bp(
        &mut operands.into_iter().peekable(),
        &mut operators.into_iter().peekable(),
        0,
        full_span,
        ctx,
    )
}

/// Pratt parser using binding power (precedence climbing).
fn pratt_parse_bp<I, J>(
    operands: &mut std::iter::Peekable<I>,
    operators: &mut std::iter::Peekable<J>,
    min_bp: u8,
    full_span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression
where
    I: Iterator<Item = Expression>,
    J: Iterator<Item = (SyntaxKind, Span)>,
{
    let Some(mut lhs) = operands.next() else {
        return Expression::error(full_span.clone());
    };

    loop {
        let Some(&(op_kind, ref op_span)) = operators.peek() else {
            break;
        };

        match OPERATOR_REGISTRY.infix_action(op_kind, min_bp) {
            InfixAction::Stop => break,

            InfixAction::InfixLeft(op, prec) => {
                let op_span = op_span.clone();
                operators.next(); // consume operator

                let rhs = pratt_parse_bp(operands, operators, prec + 1, full_span.clone(), ctx);
                let expr_span = Span::new(lhs.span.file_id, lhs.span.start..rhs.span.end);
                lhs = desugar_binary_op(op, lhs, rhs, op_span, expr_span, ctx);
            },

            InfixAction::InfixRight(op, prec) => {
                let op_span = op_span.clone();
                operators.next(); // consume operator

                let rhs = pratt_parse_bp(operands, operators, prec, full_span.clone(), ctx);
                let expr_span = Span::new(lhs.span.file_id, lhs.span.start..rhs.span.end);
                lhs = desugar_binary_op(op, lhs, rhs, op_span, expr_span, ctx);
            },

            InfixAction::Postfix(op) => {
                let op_span = op_span.clone();
                operators.next(); // consume operator

                let expr_span = Span::new(lhs.span.file_id, lhs.span.start..op_span.end);
                lhs = desugar_unary_op(op, lhs, op_span, expr_span, ctx);
            },
        }
    }

    lhs
}

/// Desugar a binary operator into a method call: lhs.method_name(rhs)
fn desugar_binary_op(
    op: BinaryOp,
    lhs: Expression,
    rhs: Expression,
    _op_span: Span,
    full_span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    // If either operand has a poison type, propagate error without cascading diagnostics.
    if lhs.ty.is_poison() || rhs.ty.is_poison() {
        return Expression::error(full_span);
    }

    // Check if LHS is a lang intrinsic type - operators are not allowed on these types
    if is_lang_intrinsic_type(&lhs.ty) {
        ctx.diagnostics.throw(OperatorOnLangIntrinsicType {
            span: full_span.clone(),
            operator: op.symbol().to_string(),
            type_name: lang_intrinsic_type_name(&lhs.ty),
            suggested_intrinsic: suggested_binary_intrinsic(&lhs.ty, op),
        });
        return Expression::error(full_span);
    }

    let method_name = op.method_name();

    // For non-primitive types, create a DeferredMethodCall.
    // Type inference will resolve this to a concrete protocol method call.
    // All operators use Infer - type inference will determine the actual return type
    // from the resolved method (e.g., Bool for comparisons, Output associated type for arithmetic).
    let result_ty = Ty::infer(full_span.clone());
    let arg = CallArgument::unlabeled(rhs.clone(), rhs.span.clone());
    Expression::deferred_method_call(
        lhs,
        method_name.to_string(),
        vec![arg],
        result_ty,
        full_span,
    )
}

/// Desugar a unary operator into a method call: operand.method_name()
fn desugar_unary_op(
    op: UnaryOp,
    operand: Expression,
    _op_span: Span,
    full_span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    // If operand has a poison type, propagate error without cascading diagnostics.
    if operand.ty.is_poison() {
        return Expression::error(full_span);
    }

    // Check if operand is a lang intrinsic type - operators are not allowed on these types
    // Exception: unwrap operator (!) is allowed as it's not a numeric/bitwise operation
    if op != UnaryOp::Unwrap && is_lang_intrinsic_type(&operand.ty) {
        ctx.diagnostics.throw(OperatorOnLangIntrinsicType {
            span: full_span.clone(),
            operator: op.symbol().to_string(),
            type_name: lang_intrinsic_type_name(&operand.ty),
            suggested_intrinsic: suggested_unary_intrinsic(&operand.ty, op),
        });
        return Expression::error(full_span);
    }

    let method_name = op.method_name();

    // For non-primitive types, create a DeferredMethodCall.
    // Type inference will resolve this to a concrete protocol method call.
    // Use Infer so type inference determines the actual return type from the resolved method.
    let result_ty = Ty::infer(full_span.clone());
    Expression::deferred_method_call(
        operand,
        method_name.to_string(),
        vec![],
        result_ty,
        full_span,
    )
}

/// Check if a type is a lang intrinsic type that should not support operators.
/// These types require explicit intrinsic function calls.
fn is_lang_intrinsic_type(ty: &Ty) -> bool {
    matches!(
        ty.kind(),
        TyKind::Int(_) | TyKind::Float(_) | TyKind::Bool | TyKind::Pointer(_) | TyKind::Array(_)
    )
}

/// Get the display name for a lang intrinsic type.
fn lang_intrinsic_type_name(ty: &Ty) -> String {
    match ty.kind() {
        TyKind::Int(bits) => match bits {
            IntBits::I8 => "lang.i8".to_string(),
            IntBits::I16 => "lang.i16".to_string(),
            IntBits::I32 => "lang.i32".to_string(),
            IntBits::I64 => "lang.i64".to_string(),
        },
        TyKind::Float(bits) => match bits {
            FloatBits::F16 => "lang.f16".to_string(),
            FloatBits::F32 => "lang.f32".to_string(),
            FloatBits::F64 => "lang.f64".to_string(),
        },
        TyKind::Bool => "lang.i1".to_string(),
        TyKind::Pointer(elem) => format!("lang.ptr[{}]", elem),
        TyKind::Array(elem) => format!("lang.array[{}]", elem),
        _ => format!("{}", ty),
    }
}

/// Get the suggested intrinsic function name for a binary operator on a type.
fn suggested_binary_intrinsic(ty: &Ty, op: BinaryOp) -> String {
    let prefix = match ty.kind() {
        TyKind::Int(bits) => match bits {
            IntBits::I8 => "lang.i8",
            IntBits::I16 => "lang.i16",
            IntBits::I32 => "lang.i32",
            IntBits::I64 => "lang.i64",
        },
        TyKind::Float(bits) => match bits {
            FloatBits::F16 => "lang.f16",
            FloatBits::F32 => "lang.f32",
            FloatBits::F64 => "lang.f64",
        },
        TyKind::Bool => "lang.i1",
        TyKind::Pointer(_) => "lang.ptr",
        TyKind::Array(_) => "lang.array",
        _ => "lang",
    };

    let op_name = match op {
        BinaryOp::Add => "_add",
        BinaryOp::Sub => "_sub",
        BinaryOp::Mul => "_mul",
        BinaryOp::Div => "_signed_div", // Suggest signed by default
        BinaryOp::Rem => "_signed_rem",
        BinaryOp::Eq => "_eq",
        BinaryOp::Ne => "_ne",
        BinaryOp::Lt => "_signed_lt",
        BinaryOp::Le => "_signed_le",
        BinaryOp::Gt => "_signed_gt",
        BinaryOp::Ge => "_signed_ge",
        BinaryOp::BitAnd => "_and",
        BinaryOp::BitOr => "_or",
        BinaryOp::BitXor => "_xor",
        BinaryOp::Shl => "_shl",
        BinaryOp::Shr => "_signed_shr",
        BinaryOp::And => "_and",
        BinaryOp::Or => "_or",
        _ => "(a, b)",
    };

    format!("{}{}(a, b)", prefix, op_name)
}

/// Get the suggested intrinsic function name for a unary operator on a type.
fn suggested_unary_intrinsic(ty: &Ty, op: UnaryOp) -> String {
    let prefix = match ty.kind() {
        TyKind::Int(bits) => match bits {
            IntBits::I8 => "lang.i8",
            IntBits::I16 => "lang.i16",
            IntBits::I32 => "lang.i32",
            IntBits::I64 => "lang.i64",
        },
        TyKind::Float(bits) => match bits {
            FloatBits::F16 => "lang.f16",
            FloatBits::F32 => "lang.f32",
            FloatBits::F64 => "lang.f64",
        },
        TyKind::Bool => "lang.i1",
        TyKind::Pointer(_) => "lang.ptr",
        _ => "lang",
    };

    let op_name = match op {
        UnaryOp::Neg => "_neg",
        UnaryOp::BitNot => "_not",
        UnaryOp::LogicalNot => "_not",
        UnaryOp::Unwrap => "", // No intrinsic for unwrap
    };

    format!("{}{}(a)", prefix, op_name)
}

/// Look up a primitive method on a type for binary operators.
/// Uses the centralized PrimitiveMethod::lookup.
#[allow(dead_code)]
fn lookup_primitive_binary_method(ty: &Ty, method_name: &str) -> Option<PrimitiveMethod> {
    PrimitiveMethod::lookup(ty, method_name)
}

/// Look up a primitive method on a type for unary operators.
/// Uses the centralized PrimitiveMethod::lookup, with special handling
/// for `!` (bitwiseNot) on Bool which maps to logicalNot.
#[allow(dead_code)]
fn lookup_primitive_unary_method(ty: &Ty, method_name: &str) -> Option<PrimitiveMethod> {
    // Special case: `!` (bitwiseNot) on Bool maps to logicalNot for compatibility
    if matches!(ty.kind(), TyKind::Bool) && method_name == "bitwiseNot" {
        return Some(PrimitiveMethod::BoolNot);
    }
    PrimitiveMethod::lookup(ty, method_name)
}
