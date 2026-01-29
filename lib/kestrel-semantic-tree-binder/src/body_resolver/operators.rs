//! Operator expression resolution.
//!
//! This module handles resolving unary, binary, and postfix operators,
//! including Pratt parsing for proper precedence handling and desugaring
//! operators into method calls on primitive types.

use std::sync::LazyLock;

use kestrel_semantic_tree::expr::{
    CallArgument, Capture, CaptureKind, Expression, PrimitiveMethod,
};
use kestrel_semantic_tree::operators::{BinaryOp, InfixAction, OperatorRegistry, UnaryOp};
use kestrel_semantic_tree::symbol::local::LocalId;
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
/// Uses the MethodRef pattern with builtin registry to produce proper
/// "does not conform to X" errors instead of "no member Y".
///
/// For short-circuiting operators (and/or), the RHS is wrapped in a closure
/// so that it's only evaluated when needed.
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
    let result_ty = Ty::infer(full_span.clone());

    // For short-circuiting operators (and/or/??), wrap RHS in a closure
    let arg = if matches!(op, BinaryOp::And | BinaryOp::Or | BinaryOp::Coalesce) {
        let closure = wrap_in_closure(rhs, ctx);
        CallArgument::unlabeled(closure.clone(), closure.span.clone())
    } else {
        CallArgument::unlabeled(rhs.clone(), rhs.span.clone())
    };

    // Try to use the MethodRef pattern with builtin registry for better error messages.
    // This produces "does not conform to X" errors instead of "no member Y".
    if let Some(feature) = op.method_feature() {
        if let Some(method_id) = ctx.model.builtin_registry().method(feature) {
            // Create MethodRef with the protocol method as candidate, then wrap in Call
            let method_ref = Expression::method_ref(
                lhs,
                vec![method_id],
                method_name.to_string(),
                full_span.clone(),
            );
            return Expression::call(method_ref, vec![arg], result_ty, full_span);
        }
    }

    // Fallback: use DeferredMethodCall if builtin not registered
    Expression::deferred_method_call(
        lhs,
        method_name.to_string(),
        vec![arg],
        result_ty,
        full_span,
    )
}

/// Desugar a unary operator into a method call: operand.method_name()
/// Uses the MethodRef pattern with builtin registry to produce proper
/// "does not conform to X" errors instead of "no member Y".
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
    let result_ty = Ty::infer(full_span.clone());

    // Try to use the MethodRef pattern with builtin registry for better error messages.
    // This produces "does not conform to X" errors instead of "no member Y".
    if let Some(feature) = op.method_feature() {
        if let Some(method_id) = ctx.model.builtin_registry().method(feature) {
            // Create MethodRef with the protocol method as candidate, then wrap in Call
            let method_ref = Expression::method_ref(
                operand,
                vec![method_id],
                method_name.to_string(),
                full_span.clone(),
            );
            return Expression::call(method_ref, vec![], result_ty, full_span);
        }
    }

    // Fallback: use DeferredMethodCall if builtin not registered
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
        TyKind::Int(_) | TyKind::Float(_) | TyKind::Bool | TyKind::Pointer(_)
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

/// Wrap an expression in a zero-argument closure for short-circuit evaluation.
///
/// Given expression `e`, this creates `{ e }` - a closure that returns `e` when called.
/// The closure captures any local variables referenced by `e`.
fn wrap_in_closure(expr: Expression, ctx: &mut BodyResolutionContext) -> Expression {
    let span = expr.span.clone();
    let return_ty = expr.ty.clone();

    // Collect captures from the expression
    let closure_entry_depth = ctx.local_scope.depth();
    let captures = collect_captures_from_expr(&expr, closure_entry_depth, &ctx.local_scope);

    // Create the closure type: () -> T
    let closure_ty = Ty::function(vec![], return_ty, span.clone());

    // Create the closure with:
    // - params: Some(vec![]) - explicit empty params (no implicit `it`)
    // - body: vec![] - no statements
    // - tail_expr: Some(expr) - the expression to evaluate
    // - captures: collected from the expression
    // - uses_it: false - we don't use implicit `it`
    // - implicit_param: None - no implicit parameter
    Expression::closure(
        Some(vec![]), // explicit empty params
        vec![],       // no statements
        Some(expr),   // tail expression
        captures,
        false, // uses_it
        None,  // implicit_param
        closure_ty,
        span,
    )
}

/// Collect captured variables from an expression.
///
/// This walks the expression tree looking for LocalRef nodes that reference
/// variables from scopes outside the closure (depth <= closure_entry_depth).
fn collect_captures_from_expr(
    expr: &Expression,
    closure_entry_depth: usize,
    local_scope: &crate::LocalScope,
) -> Vec<Capture> {
    use std::collections::HashSet;

    let mut captures = Vec::new();
    let mut seen_ids: HashSet<LocalId> = HashSet::new();

    collect_captures_recursive(
        expr,
        closure_entry_depth,
        local_scope,
        &mut captures,
        &mut seen_ids,
    );

    captures
}

/// Recursively collect captures from an expression and its children.
fn collect_captures_recursive(
    expr: &Expression,
    closure_entry_depth: usize,
    local_scope: &crate::LocalScope,
    captures: &mut Vec<Capture>,
    seen_ids: &mut std::collections::HashSet<LocalId>,
) {
    use kestrel_semantic_tree::expr::ExprKind;

    match &expr.kind {
        // LocalRef - check if it needs to be captured
        ExprKind::LocalRef(local_id) => {
            if !seen_ids.contains(local_id) {
                // Check if this local was declared before the closure scope
                if let Some(local_depth) = local_scope.scope_depth_of(*local_id) {
                    if local_depth <= closure_entry_depth {
                        // This is a capture
                        let name = local_scope
                            .get_local(*local_id)
                            .map(|info| info.name().to_string())
                            .unwrap_or_default();

                        captures.push(Capture {
                            local_id: *local_id,
                            name,
                            ty: expr.ty.clone(),
                            kind: CaptureKind::Value,
                            span: expr.span.clone(),
                        });
                        seen_ids.insert(*local_id);
                    }
                }
            }
        },

        // Recursively walk compound expressions
        ExprKind::Grouping(inner) => {
            collect_captures_recursive(inner, closure_entry_depth, local_scope, captures, seen_ids);
        },

        ExprKind::Call {
            callee, arguments, ..
        } => {
            collect_captures_recursive(
                callee,
                closure_entry_depth,
                local_scope,
                captures,
                seen_ids,
            );
            for arg in arguments {
                collect_captures_recursive(
                    &arg.value,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::MethodRef { receiver, .. } => {
            collect_captures_recursive(
                receiver,
                closure_entry_depth,
                local_scope,
                captures,
                seen_ids,
            );
        },

        ExprKind::DeferredMethodCall {
            receiver,
            arguments,
            ..
        } => {
            collect_captures_recursive(
                receiver,
                closure_entry_depth,
                local_scope,
                captures,
                seen_ids,
            );
            for arg in arguments {
                collect_captures_recursive(
                    &arg.value,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::FieldAccess { object, .. } => {
            collect_captures_recursive(
                object,
                closure_entry_depth,
                local_scope,
                captures,
                seen_ids,
            );
        },

        ExprKind::ProtocolPropertyAccess { receiver, .. } => {
            collect_captures_recursive(
                receiver,
                closure_entry_depth,
                local_scope,
                captures,
                seen_ids,
            );
        },

        ExprKind::TupleIndex { tuple, .. } => {
            collect_captures_recursive(tuple, closure_entry_depth, local_scope, captures, seen_ids);
        },

        ExprKind::If {
            conditions,
            then_branch,
            then_value,
            else_branch,
        } => {
            for cond in conditions {
                match cond {
                    kestrel_semantic_tree::expr::IfCondition::Expr(e) => {
                        collect_captures_recursive(
                            e,
                            closure_entry_depth,
                            local_scope,
                            captures,
                            seen_ids,
                        );
                    },
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        collect_captures_recursive(
                            value,
                            closure_entry_depth,
                            local_scope,
                            captures,
                            seen_ids,
                        );
                    },
                }
            }
            for stmt in then_branch {
                collect_captures_from_stmt(
                    stmt,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
            if let Some(val) = then_value {
                collect_captures_recursive(
                    val,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
            if let Some(else_br) = else_branch {
                collect_captures_from_else(
                    else_br,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::Closure {
            body, tail_expr, ..
        } => {
            // Note: nested closures have their own captures, but we still need to
            // walk them to find variables that need to be captured by the outer closure
            for stmt in body {
                collect_captures_from_stmt(
                    stmt,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
            if let Some(tail) = tail_expr {
                collect_captures_recursive(
                    tail,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::Block { statements, value } => {
            for stmt in statements {
                collect_captures_from_stmt(
                    stmt,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
            if let Some(val) = value {
                collect_captures_recursive(
                    val,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::Tuple(elements) | ExprKind::Array(elements) => {
            for elem in elements {
                collect_captures_recursive(
                    elem,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::Dictionary(pairs) => {
            for (key, value) in pairs {
                collect_captures_recursive(
                    key,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
                collect_captures_recursive(
                    value,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::While {
            condition, body, ..
        } => {
            collect_captures_recursive(
                condition,
                closure_entry_depth,
                local_scope,
                captures,
                seen_ids,
            );
            for stmt in body {
                collect_captures_from_stmt(
                    stmt,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::WhileLet {
            conditions, body, ..
        } => {
            for cond in conditions {
                match cond {
                    kestrel_semantic_tree::expr::IfCondition::Expr(e) => {
                        collect_captures_recursive(
                            e,
                            closure_entry_depth,
                            local_scope,
                            captures,
                            seen_ids,
                        );
                    },
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        collect_captures_recursive(
                            value,
                            closure_entry_depth,
                            local_scope,
                            captures,
                            seen_ids,
                        );
                    },
                }
            }
            for stmt in body {
                collect_captures_from_stmt(
                    stmt,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::Loop { body, .. } => {
            for stmt in body {
                collect_captures_from_stmt(
                    stmt,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::Return { value } => {
            if let Some(e) = value {
                collect_captures_recursive(e, closure_entry_depth, local_scope, captures, seen_ids);
            }
        },

        ExprKind::Match { scrutinee, arms } => {
            collect_captures_recursive(
                scrutinee,
                closure_entry_depth,
                local_scope,
                captures,
                seen_ids,
            );
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_captures_recursive(
                        guard,
                        closure_entry_depth,
                        local_scope,
                        captures,
                        seen_ids,
                    );
                }
                collect_captures_recursive(
                    &arm.body,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::Assignment { target, value } => {
            collect_captures_recursive(
                target,
                closure_entry_depth,
                local_scope,
                captures,
                seen_ids,
            );
            collect_captures_recursive(value, closure_entry_depth, local_scope, captures, seen_ids);
        },

        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments {
                collect_captures_recursive(
                    &arg.value,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::SubscriptCall {
            receiver,
            arguments,
            ..
        } => {
            collect_captures_recursive(
                receiver,
                closure_entry_depth,
                local_scope,
                captures,
                seen_ids,
            );
            for arg in arguments {
                collect_captures_recursive(
                    &arg.value,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::PrimitiveMethodCall {
            receiver,
            arguments,
            ..
        } => {
            collect_captures_recursive(
                receiver,
                closure_entry_depth,
                local_scope,
                captures,
                seen_ids,
            );
            for arg in arguments {
                collect_captures_recursive(
                    &arg.value,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::PrimitiveMethodRef { receiver, .. } => {
            collect_captures_recursive(
                receiver,
                closure_entry_depth,
                local_scope,
                captures,
                seen_ids,
            );
        },

        ExprKind::DeferredStaticCall { arguments, .. } => {
            for arg in arguments {
                collect_captures_recursive(
                    &arg.value,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::DelegatingInit { arguments, .. } => {
            for arg in arguments {
                collect_captures_recursive(
                    &arg.value,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::LangIntrinsic { arguments, .. } => {
            for arg in arguments {
                collect_captures_recursive(
                    &arg.value,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },

        ExprKind::ImplicitMemberAccess { arguments, .. } => {
            if let Some(args) = arguments {
                for arg in args {
                    collect_captures_recursive(
                        &arg.value,
                        closure_entry_depth,
                        local_scope,
                        captures,
                        seen_ids,
                    );
                }
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
        | ExprKind::Break { .. }
        | ExprKind::Continue { .. }
        | ExprKind::Error => {},
    }
}

/// Collect captures from a statement.
fn collect_captures_from_stmt(
    stmt: &kestrel_semantic_tree::stmt::Statement,
    closure_entry_depth: usize,
    local_scope: &crate::LocalScope,
    captures: &mut Vec<Capture>,
    seen_ids: &mut std::collections::HashSet<LocalId>,
) {
    use kestrel_semantic_tree::stmt::StatementKind;

    match &stmt.kind {
        StatementKind::Expr(expr) => {
            collect_captures_recursive(expr, closure_entry_depth, local_scope, captures, seen_ids);
        },
        StatementKind::Binding { value, .. } => {
            if let Some(expr) = value {
                collect_captures_recursive(
                    expr,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },
        StatementKind::GuardLet {
            conditions,
            else_block,
        } => {
            for condition in conditions {
                match condition {
                    kestrel_semantic_tree::expr::IfCondition::Expr(expr) => {
                        collect_captures_recursive(
                            expr,
                            closure_entry_depth,
                            local_scope,
                            captures,
                            seen_ids,
                        );
                    },
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        collect_captures_recursive(
                            value,
                            closure_entry_depth,
                            local_scope,
                            captures,
                            seen_ids,
                        );
                    },
                }
            }
            for stmt in &else_block.statements {
                collect_captures_from_stmt(
                    stmt,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
            if let Some(yield_expr) = &else_block.yield_expr {
                collect_captures_recursive(
                    yield_expr,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },
        StatementKind::Deinit { .. } => {
            // Deinit doesn't contain expressions to recurse into
        },
    }
}

/// Collect captures from an else branch.
fn collect_captures_from_else(
    else_branch: &kestrel_semantic_tree::expr::ElseBranch,
    closure_entry_depth: usize,
    local_scope: &crate::LocalScope,
    captures: &mut Vec<Capture>,
    seen_ids: &mut std::collections::HashSet<LocalId>,
) {
    match else_branch {
        kestrel_semantic_tree::expr::ElseBranch::Block { statements, value } => {
            for stmt in statements {
                collect_captures_from_stmt(
                    stmt,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
            if let Some(val) = value {
                collect_captures_recursive(
                    val,
                    closure_entry_depth,
                    local_scope,
                    captures,
                    seen_ids,
                );
            }
        },
        kestrel_semantic_tree::expr::ElseBranch::ElseIf(expr) => {
            collect_captures_recursive(expr, closure_entry_depth, local_scope, captures, seen_ids);
        },
    }
}
