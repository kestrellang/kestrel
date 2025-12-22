//! Pattern resolution.
//!
//! This module handles resolving patterns from syntax nodes into semantic
//! Pattern representations. It converts the parsed pattern syntax tree into
//! the semantic pattern types used for type checking and code generation.
//!
//! # Pattern Types
//!
//! - Wildcard (`_`): Matches anything, binds nothing
//! - Binding (`x` or `var x`): Binds a value to a name
//! - Tuple (`(a, b, c)`): Destructures a tuple
//! - Literal (`42`, `"hello"`, `true`): Matches a specific value
//! - Enum (`.Case` or `.Case(args)`): Matches an enum variant

use kestrel_semantic_tree::expr::LiteralValue;
use kestrel_semantic_tree::pattern::{EnumPatternBinding, Mutability, Pattern, PatternKind};
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use kestrel_syntax_tree::utils::get_node_span;

use super::context::BodyResolutionContext;

/// Resolve a pattern syntax node into a semantic Pattern.
///
/// This function handles all pattern kinds including:
/// - Wildcard patterns (`_`)
/// - Binding patterns (`x` or `var x`)
/// - Tuple patterns (`(a, b, c)`)
/// - Literal patterns (`42`, `"hello"`, `true`)
/// - Enum variant patterns (`.Case` or `.Case(args)`)
///
/// # Arguments
/// * `pattern_node` - The syntax node for the pattern
/// * `ctx` - The body resolution context
/// * `expected_ty` - Optional expected type hint for the pattern
///
/// # Returns
/// A resolved `Pattern` with its type and span.
pub fn resolve_pattern(
    pattern_node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
) -> Pattern {
    let span = get_node_span(pattern_node, ctx.file_id);

    // Handle Pattern wrapper node
    if pattern_node.kind() == SyntaxKind::Pattern {
        if let Some(inner) = pattern_node.children().next() {
            return resolve_pattern_inner(&inner, ctx, expected_ty, span);
        }
        return Pattern::error(span);
    }

    resolve_pattern_inner(pattern_node, ctx, expected_ty, span)
}

/// Resolve the inner pattern node (unwrapped from Pattern wrapper).
fn resolve_pattern_inner(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
    span: Span,
) -> Pattern {
    match node.kind() {
        SyntaxKind::WildcardPattern => resolve_wildcard_pattern(node, ctx, expected_ty),
        SyntaxKind::BindingPattern => resolve_binding_pattern(node, ctx, expected_ty),
        SyntaxKind::TuplePattern => resolve_tuple_pattern(node, ctx, expected_ty),
        SyntaxKind::LiteralPattern => resolve_literal_pattern(node, ctx, expected_ty),
        SyntaxKind::EnumPattern => resolve_enum_pattern(node, ctx, expected_ty),
        SyntaxKind::ErrorPattern => Pattern::error(span),
        _ => {
            // Unknown pattern kind - treat as error
            Pattern::error(span)
        }
    }
}

/// Resolve a wildcard pattern (`_`).
fn resolve_wildcard_pattern(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
) -> Pattern {
    let span = get_node_span(node, ctx.file_id);
    let ty = expected_ty.cloned().unwrap_or_else(|| Ty::infer(span.clone()));
    Pattern::wildcard(ty, span)
}

/// Resolve a binding pattern (`x` or `var x`).
fn resolve_binding_pattern(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
) -> Pattern {
    let span = get_node_span(node, ctx.file_id);

    // Check for `var` keyword (mutable binding)
    let is_mutable = node
        .children_with_tokens()
        .any(|elem| elem.kind() == SyntaxKind::Var);
    let mutability = if is_mutable {
        Mutability::Mutable
    } else {
        Mutability::Immutable
    };

    // Extract the identifier name
    let name = node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string());

    let name = match name {
        Some(n) => n,
        None => return Pattern::error(span),
    };

    // Determine type from expected type or use infer
    let ty = expected_ty.cloned().unwrap_or_else(|| Ty::infer(span.clone()));

    // Get span for the name token
    let name_span = node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| {
            let text_range = t.text_range();
            Span::new(ctx.file_id, text_range.start().into()..text_range.end().into())
        })
        .unwrap_or_else(|| span.clone());

    // Bind the local variable
    let local_id = ctx.local_scope.bind(
        name.clone(),
        ty.clone(),
        is_mutable,
        name_span.clone(),
    );

    Pattern::local(local_id, mutability, name, ty, span)
}

/// Resolve a tuple pattern (`(a, b, c)`).
fn resolve_tuple_pattern(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
) -> Pattern {
    let span = get_node_span(node, ctx.file_id);

    // Get expected element types if we have a tuple type
    let expected_element_types: Option<Vec<Ty>> = expected_ty.and_then(|ty| {
        if let kestrel_semantic_tree::ty::TyKind::Tuple(element_types) = ty.kind() {
            Some(element_types.clone())
        } else {
            None
        }
    });

    // Collect tuple pattern elements
    let elements: Vec<Pattern> = node
        .children()
        .filter(|c| c.kind() == SyntaxKind::TuplePatternElement)
        .enumerate()
        .map(|(i, elem_node)| {
            let expected_elem_ty = expected_element_types
                .as_ref()
                .and_then(|tys| tys.get(i));
            
            // The element node contains the actual pattern
            if let Some(inner_pattern) = elem_node.children().next() {
                resolve_pattern_inner(&inner_pattern, ctx, expected_elem_ty, get_node_span(&elem_node, ctx.file_id))
            } else {
                Pattern::error(get_node_span(&elem_node, ctx.file_id))
            }
        })
        .collect();

    // Build tuple type from element types
    let element_types: Vec<Ty> = elements.iter().map(|p| p.ty.clone()).collect();
    let ty = Ty::tuple(element_types, span.clone());

    Pattern::tuple(elements, ty, span)
}

/// Resolve a literal pattern (`42`, `"hello"`, `true`).
fn resolve_literal_pattern(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    _expected_ty: Option<&Ty>,
) -> Pattern {
    let span = get_node_span(node, ctx.file_id);

    // Find the literal token
    for child in node.children_with_tokens() {
        if let Some(token) = child.into_token() {
            let text = token.text();
            match token.kind() {
                SyntaxKind::Integer => {
                    let value = parse_integer(text);
                    let ty = Ty::int(kestrel_semantic_tree::ty::IntBits::I64, span.clone());
                    return Pattern::literal(LiteralValue::Integer(value), ty, span);
                }
                SyntaxKind::Float => {
                    let value = text.parse::<f64>().unwrap_or(0.0);
                    let ty = Ty::float(kestrel_semantic_tree::ty::FloatBits::F64, span.clone());
                    return Pattern::literal(LiteralValue::Float(value), ty, span);
                }
                SyntaxKind::String => {
                    // Remove quotes
                    let value = text.trim_matches('"').to_string();
                    let ty = Ty::string(span.clone());
                    return Pattern::literal(LiteralValue::String(value), ty, span);
                }
                SyntaxKind::Boolean => {
                    let value = text == "true";
                    let ty = Ty::bool(span.clone());
                    return Pattern::literal(LiteralValue::Bool(value), ty, span);
                }
                _ => {}
            }
        }
    }

    Pattern::error(span)
}

/// Parse an integer literal (handles hex, binary, octal).
fn parse_integer(text: &str) -> i64 {
    if text.starts_with("0x") || text.starts_with("0X") {
        i64::from_str_radix(&text[2..], 16).unwrap_or(0)
    } else if text.starts_with("0b") || text.starts_with("0B") {
        i64::from_str_radix(&text[2..], 2).unwrap_or(0)
    } else if text.starts_with("0o") || text.starts_with("0O") {
        i64::from_str_radix(&text[2..], 8).unwrap_or(0)
    } else {
        text.parse::<i64>().unwrap_or(0)
    }
}

/// Resolve an enum variant pattern (`.Case` or `.Case(args)`).
fn resolve_enum_pattern(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
) -> Pattern {
    let span = get_node_span(node, ctx.file_id);

    // Extract case name (identifier after the dot)
    let case_name = node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string());

    let case_name = match case_name {
        Some(n) => n,
        None => return Pattern::error(span),
    };

    // Extract bindings from EnumPatternArg nodes
    let bindings: Vec<EnumPatternBinding> = node
        .children()
        .filter(|c| c.kind() == SyntaxKind::EnumPatternArg)
        .map(|arg_node| resolve_enum_pattern_arg(&arg_node, ctx))
        .collect();

    // Type is inferred from context - will be resolved during type inference
    let ty = expected_ty.cloned().unwrap_or_else(|| Ty::infer(span.clone()));

    // case_id is None - resolved during type inference
    Pattern::enum_variant(None, case_name, bindings, ty, span)
}

/// Resolve a single enum pattern argument.
fn resolve_enum_pattern_arg(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> EnumPatternBinding {
    let span = get_node_span(node, ctx.file_id);

    // Get the label (first identifier)
    let label = node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string());

    // Check if there's a colon (labeled binding with nested pattern)
    let has_colon = node
        .children_with_tokens()
        .any(|elem| elem.kind() == SyntaxKind::Colon);

    if has_colon {
        // Labeled binding: `label: pattern`
        // Find the nested pattern
        let pattern = node
            .children()
            .find(|c| is_pattern_kind(c.kind()))
            .map(|p| resolve_pattern(&p, ctx, None))
            .unwrap_or_else(|| Pattern::error(span.clone()));

        EnumPatternBinding::new(label, pattern, span)
    } else {
        // Simple binding: just an identifier that becomes a binding pattern
        let label_str = label.unwrap_or_else(|| "_".to_string());
        
        // Create a binding pattern for this label
        let binding_ty = Ty::infer(span.clone());
        let local_id = ctx.local_scope.bind(
            label_str.clone(),
            binding_ty.clone(),
            false, // immutable by default
            span.clone(),
        );
        let pattern = Pattern::local(local_id, Mutability::Immutable, label_str, binding_ty, span.clone());

        EnumPatternBinding::new(None, pattern, span)
    }
}

/// Check if a SyntaxKind is a pattern kind.
fn is_pattern_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Pattern
            | SyntaxKind::WildcardPattern
            | SyntaxKind::BindingPattern
            | SyntaxKind::TuplePattern
            | SyntaxKind::LiteralPattern
            | SyntaxKind::EnumPattern
            | SyntaxKind::ErrorPattern
    )
}
