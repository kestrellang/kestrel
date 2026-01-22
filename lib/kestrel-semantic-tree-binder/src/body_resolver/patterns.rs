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

use std::collections::HashMap;

use kestrel_reporting::IntoDiagnostic;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::expr::LiteralValue;
use kestrel_semantic_tree::pattern::{
    EnumPatternBinding, Mutability, Pattern, PatternKind, RangeBound, StructPatternField,
};
use kestrel_semantic_tree::ty::{Ty, TyKind};
use kestrel_span::Span;
use kestrel_syntax_tree::utils::get_node_span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

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
    resolve_pattern_with_mutability(pattern_node, ctx, expected_ty, false)
}

/// Resolve a pattern syntax node with an optional forced mutability.
///
/// When `force_mutable` is true, all binding patterns will be created as mutable,
/// even if they don't have the `var` keyword. This is used for `var (a, b)` style
/// declarations where mutability is specified at the statement level.
///
/// # Arguments
/// * `pattern_node` - The syntax node for the pattern
/// * `ctx` - The body resolution context
/// * `expected_ty` - Optional expected type hint for the pattern
/// * `force_mutable` - If true, all bindings will be mutable
///
/// # Returns
/// A resolved `Pattern` with its type and span.
pub fn resolve_pattern_with_mutability(
    pattern_node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
    force_mutable: bool,
) -> Pattern {
    let span = get_node_span(pattern_node, ctx.file_id);

    // Handle Pattern wrapper node
    let pattern = if pattern_node.kind() == SyntaxKind::Pattern {
        if let Some(inner) = pattern_node.children().next() {
            resolve_pattern_inner(&inner, ctx, expected_ty, span, force_mutable)
        } else {
            Pattern::error(span)
        }
    } else {
        resolve_pattern_inner(pattern_node, ctx, expected_ty, span, force_mutable)
    };

    // Check for duplicate bindings within the pattern
    check_duplicate_bindings(&pattern, ctx);

    pattern
}

/// Resolve the inner pattern node (unwrapped from Pattern wrapper).
fn resolve_pattern_inner(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
    span: Span,
    force_mutable: bool,
) -> Pattern {
    match node.kind() {
        SyntaxKind::WildcardPattern => resolve_wildcard_pattern(node, ctx, expected_ty),
        SyntaxKind::BindingPattern => {
            resolve_binding_pattern(node, ctx, expected_ty, force_mutable)
        },
        SyntaxKind::TuplePattern => resolve_tuple_pattern(node, ctx, expected_ty, force_mutable),
        SyntaxKind::LiteralPattern => resolve_literal_pattern(node, ctx, expected_ty),
        SyntaxKind::EnumPattern => resolve_enum_pattern(node, ctx, expected_ty, force_mutable),
        SyntaxKind::RangePattern => resolve_range_pattern(node, ctx, expected_ty),
        SyntaxKind::OrPattern => resolve_or_pattern(node, ctx, expected_ty, force_mutable),
        SyntaxKind::StructPattern => resolve_struct_pattern(node, ctx, expected_ty, force_mutable),
        SyntaxKind::ArrayPattern => resolve_array_pattern(node, ctx, expected_ty, force_mutable),
        SyntaxKind::AtPattern => resolve_at_pattern(node, ctx, expected_ty, force_mutable),
        SyntaxKind::RestPattern => resolve_rest_pattern(node, ctx, expected_ty),
        SyntaxKind::ErrorPattern => Pattern::error(span),
        _ => {
            // Unknown pattern kind - treat as error
            Pattern::error(span)
        },
    }
}

/// Resolve a wildcard pattern (`_`).
fn resolve_wildcard_pattern(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
) -> Pattern {
    let span = get_node_span(node, ctx.file_id);
    let ty = expected_ty
        .cloned()
        .unwrap_or_else(|| Ty::infer(span.clone()));
    Pattern::wildcard(ty, span)
}

/// Resolve a binding pattern (`x` or `var x`).
fn resolve_binding_pattern(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
    force_mutable: bool,
) -> Pattern {
    let span = get_node_span(node, ctx.file_id);

    // Check for `var` keyword (mutable binding) or force_mutable from statement level
    let has_var_keyword = node
        .children_with_tokens()
        .any(|elem| elem.kind() == SyntaxKind::Var);
    let is_mutable = force_mutable || has_var_keyword;
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
    let ty = expected_ty
        .cloned()
        .unwrap_or_else(|| Ty::infer(span.clone()));

    // Get span for the name token
    let name_span = node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| {
            let text_range = t.text_range();
            Span::new(
                ctx.file_id,
                text_range.start().into()..text_range.end().into(),
            )
        })
        .unwrap_or_else(|| span.clone());

    // Bind the local variable
    let local_id = ctx
        .local_scope
        .bind(name.clone(), ty.clone(), is_mutable, name_span.clone());

    Pattern::local(local_id, mutability, name, ty, span)
}

/// Resolve a tuple pattern (`(a, b, c)`).
fn resolve_tuple_pattern(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
    force_mutable: bool,
) -> Pattern {
    let span = get_node_span(node, ctx.file_id);

    // First pass: collect all elements and find rest patterns
    let element_nodes: Vec<_> = node
        .children()
        .filter(|c| c.kind() == SyntaxKind::TuplePatternElement)
        .collect();

    // Check if this is a single-element pattern in parentheses (grouping, not a tuple)
    // A single-element tuple requires a trailing comma like (x,), while (x) is grouping
    // Since the parser doesn't distinguish, we check: if there's exactly one element
    // and the expected type is NOT a tuple, treat it as grouping.
    if element_nodes.len() == 1 {
        // Check if the expected type is a single-element tuple
        let is_expected_single_tuple = expected_ty
            .map(|ty| matches!(ty.kind(), kestrel_semantic_tree::ty::TyKind::Tuple(elems) if elems.len() == 1))
            .unwrap_or(false);

        if !is_expected_single_tuple {
            // This is grouping, not a tuple. Unwrap the inner pattern.
            if let Some(inner_pattern) = element_nodes[0].children().next() {
                return resolve_pattern_inner(
                    &inner_pattern,
                    ctx,
                    expected_ty,
                    span,
                    force_mutable,
                );
            }
        }
    }

    // Get expected element types if we have a tuple type
    let expected_element_types: Option<Vec<Ty>> = expected_ty.and_then(|ty| {
        if let kestrel_semantic_tree::ty::TyKind::Tuple(element_types) = ty.kind() {
            Some(element_types.clone())
        } else {
            None
        }
    });

    // Find rest pattern indices
    let mut rest_indices: Vec<usize> = Vec::new();
    for (i, elem_node) in element_nodes.iter().enumerate() {
        if let Some(inner) = elem_node.children().next()
            && inner.kind() == SyntaxKind::RestPattern
        {
            rest_indices.push(i);
        }
    }

    // Check for multiple rest patterns
    if rest_indices.len() > 1 {
        use crate::diagnostics::MultipleRestPatternsError;
        let error = MultipleRestPatternsError { span: span.clone() };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Pattern::error(span);
    }

    let has_rest = !rest_indices.is_empty();
    let rest_index = rest_indices.first().copied();

    // Calculate expected types for prefix and suffix
    let _expected_tuple_len = expected_element_types.as_ref().map(|t| t.len());

    // Split elements into prefix and suffix based on rest pattern position
    let (prefix_nodes, suffix_nodes): (Vec<_>, Vec<_>) = if let Some(rest_idx) = rest_index {
        let prefix = element_nodes[..rest_idx].to_vec();
        let suffix = element_nodes[rest_idx + 1..].to_vec();
        (prefix, suffix)
    } else {
        (element_nodes, vec![])
    };

    // Resolve prefix patterns
    let prefix: Vec<Pattern> = prefix_nodes
        .iter()
        .enumerate()
        .map(|(i, elem_node)| {
            let expected_elem_ty = expected_element_types.as_ref().and_then(|tys| tys.get(i));

            if let Some(inner_pattern) = elem_node.children().next() {
                resolve_pattern_inner(
                    &inner_pattern,
                    ctx,
                    expected_elem_ty,
                    get_node_span(elem_node, ctx.file_id),
                    force_mutable,
                )
            } else {
                Pattern::error(get_node_span(elem_node, ctx.file_id))
            }
        })
        .collect();

    // Resolve suffix patterns
    // Suffix patterns are matched from the END of the expected type
    let suffix: Vec<Pattern> = suffix_nodes
        .iter()
        .enumerate()
        .map(|(i, elem_node)| {
            // Calculate index from end: for suffix elements, we need to index from the end
            let expected_elem_ty = expected_element_types.as_ref().and_then(|tys| {
                // Suffix index from end: suffix[0] matches tys[len - suffix_len], etc.
                let suffix_start = tys.len().saturating_sub(suffix_nodes.len());
                tys.get(suffix_start + i)
            });

            if let Some(inner_pattern) = elem_node.children().next() {
                resolve_pattern_inner(
                    &inner_pattern,
                    ctx,
                    expected_elem_ty,
                    get_node_span(elem_node, ctx.file_id),
                    force_mutable,
                )
            } else {
                Pattern::error(get_node_span(elem_node, ctx.file_id))
            }
        })
        .collect();

    // Build tuple type
    // For patterns with rest, use the expected type if available
    // Otherwise, build from prefix + suffix (which may be incomplete)
    let ty = if has_rest {
        expected_ty.cloned().unwrap_or_else(|| {
            // Fallback: create a tuple type from prefix + suffix
            // This may be wrong but type inference will catch it
            let element_types: Vec<Ty> = prefix
                .iter()
                .chain(suffix.iter())
                .map(|p| p.ty.clone())
                .collect();
            Ty::tuple(element_types, span.clone())
        })
    } else {
        // No rest pattern - build type from all elements
        let element_types: Vec<Ty> = prefix.iter().map(|p| p.ty.clone()).collect();
        Ty::tuple(element_types, span.clone())
    };

    Pattern::tuple_with_rest(prefix, has_rest, suffix, ty, span)
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
                    // Use inference placeholder - let type inference determine the actual type
                    // based on the scrutinee type and ExpressibleByIntLiteral conformance
                    let ty = Ty::infer(span.clone());
                    return Pattern::literal(LiteralValue::Integer(value), ty, span);
                },
                SyntaxKind::Float => {
                    // Float literals are not allowed in patterns
                    use crate::diagnostics::FloatLiteralInPatternError;
                    let error = FloatLiteralInPatternError { span: span.clone() };
                    ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                    return Pattern::error(span);
                },
                SyntaxKind::String => {
                    // Remove quotes and process escape sequences
                    let text_range = token.text_range();
                    let token_start: usize = text_range.start().into();
                    let inner = if text.len() >= 2 {
                        &text[1..text.len() - 1]
                    } else {
                        text
                    };
                    let value = super::expressions::unescape_string(
                        inner,
                        ctx.file_id,
                        token_start + 1,
                        ctx,
                    );
                    let ty = Ty::string(span.clone());
                    return Pattern::literal(LiteralValue::String(value), ty, span);
                },
                SyntaxKind::RawString => {
                    // Raw strings: strip quotes, no escape processing
                    let quote_count = text.chars().take_while(|&c| c == '"').count();
                    let value = if text.len() >= quote_count * 2 {
                        text[quote_count..text.len() - quote_count].to_string()
                    } else {
                        text.to_string()
                    };
                    let ty = Ty::string(span.clone());
                    return Pattern::literal(LiteralValue::String(value), ty, span);
                },
                SyntaxKind::Boolean => {
                    let value = text == "true";
                    // Use infer type so type inference can unify with scrutinee type
                    // based on the scrutinee type and ExpressibleByBoolLiteral conformance
                    let ty = Ty::infer(span.clone());
                    return Pattern::literal(LiteralValue::Bool(value), ty, span);
                },
                SyntaxKind::Char => {
                    // Process char literal: strip quotes, process escapes, validate single codepoint
                    let text_range = token.text_range();
                    let token_start: usize = text_range.start().into();
                    let token_span =
                        Span::new(ctx.file_id, token_start..token_start + text.len());

                    let value = if text.len() >= 2 {
                        let inner = &text[1..text.len() - 1];

                        if inner.is_empty() {
                            // Empty character literal ''
                            use crate::diagnostics::EmptyCharacterLiteralError;
                            let error = EmptyCharacterLiteralError { span: token_span };
                            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                            0
                        } else {
                            // Process escape sequences
                            let unescaped = super::expressions::unescape_string(
                                inner,
                                ctx.file_id,
                                token_start + 1,
                                ctx,
                            );
                            let code_points: Vec<char> = unescaped.chars().collect();

                            if code_points.is_empty() {
                                use crate::diagnostics::EmptyCharacterLiteralError;
                                let error = EmptyCharacterLiteralError { span: token_span };
                                ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                                0
                            } else if code_points.len() > 1 {
                                use crate::diagnostics::MultipleCodepointsInCharLiteralError;
                                let error = MultipleCodepointsInCharLiteralError {
                                    span: token_span,
                                    count: code_points.len(),
                                };
                                ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                                code_points[0] as u32
                            } else {
                                code_points[0] as u32
                            }
                        }
                    } else {
                        0
                    };

                    // Use infer type so type inference can unify with scrutinee type
                    // based on the scrutinee type and ExpressibleByCharLiteral conformance
                    let ty = Ty::infer(span.clone());
                    return Pattern::literal(LiteralValue::Char(value), ty, span);
                },
                _ => {},
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
    force_mutable: bool,
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

    // Try to get expected types for bindings from the enum case definition.
    // This allows pattern-bound variables to have concrete types immediately,
    // which enables member access in conditions like `if let .Some(value: p) = opt, p.x > 0`.
    let binding_expected_types = get_enum_case_binding_types(expected_ty, &case_name);

    // Collect EnumPatternArg nodes
    let arg_nodes: Vec<_> = node
        .children()
        .filter(|c| c.kind() == SyntaxKind::EnumPatternArg)
        .collect();

    // Extract bindings from EnumPatternArg nodes with expected types
    let bindings: Vec<EnumPatternBinding> = arg_nodes
        .iter()
        .enumerate()
        .map(|(i, arg_node)| {
            let expected_binding_ty = binding_expected_types.as_ref().and_then(|tys| tys.get(i));
            resolve_enum_pattern_arg(arg_node, ctx, force_mutable, expected_binding_ty)
        })
        .collect();

    // Type is inferred from context - will be resolved during type inference
    let ty = expected_ty
        .cloned()
        .unwrap_or_else(|| Ty::infer(span.clone()));

    // case_id is None - resolved during type inference
    Pattern::enum_variant(None, case_name, bindings, ty, span)
}

/// Get the expected types for enum case bindings from the expected enum type.
///
/// If `expected_ty` is an enum type and we can find the case, returns the
/// parameter types from the case's CallableBehavior, with substitutions applied.
fn get_enum_case_binding_types(expected_ty: Option<&Ty>, case_name: &str) -> Option<Vec<Ty>> {
    let ty = expected_ty?;

    // Check if the expected type is an enum
    let (enum_sym, substitutions) = match ty.kind() {
        TyKind::Enum {
            symbol,
            substitutions,
        } => (symbol, substitutions),
        _ => return None,
    };

    // Find the case by name
    let case = enum_sym
        .cases()
        .into_iter()
        .find(|c| c.metadata().name().value == case_name)?;

    // Get the CallableBehavior (contains parameter types)
    let callable: CallableBehavior = (*case.callable_behavior()?).clone();

    // Get parameter types and apply substitutions
    let param_types: Vec<Ty> = callable
        .parameters()
        .iter()
        .map(|param| {
            let ty = param.ty.clone();
            // Apply substitutions to handle generic enums like Option[Point]
            // where the case Some(value: T) should yield Point for the binding
            substitutions.apply(&ty)
        })
        .collect();

    Some(param_types)
}

/// Resolve a single enum pattern argument.
///
/// The `expected_ty` parameter is the expected type for this binding position,
/// derived from the enum case's parameter types. This allows pattern-bound
/// variables to have concrete types immediately.
fn resolve_enum_pattern_arg(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    force_mutable: bool,
    expected_ty: Option<&Ty>,
) -> EnumPatternBinding {
    let span = get_node_span(node, ctx.file_id);

    // Get the label (first identifier token, not from a nested pattern)
    let label = node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string());

    // Check if there's a colon (labeled binding with nested pattern)
    let has_colon = node
        .children_with_tokens()
        .any(|elem| elem.kind() == SyntaxKind::Colon);

    // Check if there's a nested pattern node (for unlabeled patterns like `_`)
    let nested_pattern = node.children().find(|c| is_pattern_kind(c.kind()));

    if has_colon {
        // Labeled binding: `label: pattern`
        // Find the nested pattern and pass the expected type
        let pattern = nested_pattern
            .map(|p| resolve_pattern_with_mutability(&p, ctx, expected_ty, force_mutable))
            .unwrap_or_else(|| Pattern::error(span.clone()));

        EnumPatternBinding::new(label, pattern, span)
    } else if label.is_none() && nested_pattern.is_some() {
        // Unlabeled pattern: just a pattern like `_`, `(a, b)`, `.Nested(x)`, etc.
        let pattern = resolve_pattern_with_mutability(
            &nested_pattern.unwrap(),
            ctx,
            expected_ty,
            force_mutable,
        );

        EnumPatternBinding::new(None, pattern, span)
    } else {
        // Simple binding: just an identifier that becomes a binding pattern
        let label_str = label.unwrap_or_else(|| "_".to_string());

        // Create a binding pattern for this label.
        // Use expected type if available, otherwise create inference placeholder.
        let binding_ty = expected_ty
            .cloned()
            .unwrap_or_else(|| Ty::infer(span.clone()));
        let is_mutable = force_mutable;
        let mutability = if is_mutable {
            Mutability::Mutable
        } else {
            Mutability::Immutable
        };
        let local_id = ctx.local_scope.bind(
            label_str.clone(),
            binding_ty.clone(),
            is_mutable,
            span.clone(),
        );
        let pattern = Pattern::local(local_id, mutability, label_str, binding_ty, span.clone());

        EnumPatternBinding::new(None, pattern, span)
    }
}

/// Resolve a struct pattern (`Point { x, y }` or `Point { x: a, y: b }`).
fn resolve_struct_pattern(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
    force_mutable: bool,
) -> Pattern {
    let span = get_node_span(node, ctx.file_id);

    // Extract struct name (first identifier)
    let struct_name = node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string());

    let struct_name = match struct_name {
        Some(n) => n,
        None => return Pattern::error(span),
    };

    // Check for rest pattern (..)
    let has_rest = node
        .children()
        .any(|c| c.kind() == SyntaxKind::StructPatternRest);

    // Extract fields from StructPatternField nodes
    let fields: Vec<StructPatternField> = node
        .children()
        .filter(|c| c.kind() == SyntaxKind::StructPatternField)
        .map(|field_node| resolve_struct_pattern_field(&field_node, ctx, force_mutable))
        .collect();

    // Type is inferred from context - will be resolved during type inference
    let ty = expected_ty
        .cloned()
        .unwrap_or_else(|| Ty::infer(span.clone()));

    // struct_id is None - resolved during type inference
    Pattern::struct_pattern(None, struct_name, fields, has_rest, ty, span)
}

/// Resolve a single struct pattern field.
fn resolve_struct_pattern_field(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    force_mutable: bool,
) -> StructPatternField {
    let span = get_node_span(node, ctx.file_id);

    // Get the field name (first identifier)
    let field_name = node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string())
        .unwrap_or_else(|| "_".to_string());

    // Check if there's a colon (explicit binding with nested pattern)
    let has_colon = node
        .children_with_tokens()
        .any(|elem| elem.kind() == SyntaxKind::Colon);

    if has_colon {
        // Explicit binding: `field: pattern`
        // Find the nested pattern
        let pattern = node
            .children()
            .find(|c| is_pattern_kind(c.kind()))
            .map(|p| resolve_pattern_with_mutability(&p, ctx, None, force_mutable))
            .unwrap_or_else(|| Pattern::error(span.clone()));

        StructPatternField::new(field_name, pattern, span)
    } else {
        // Shorthand binding: just an identifier that becomes a binding pattern
        // `{ x }` is equivalent to `{ x: x }`
        let binding_ty = Ty::infer(span.clone());
        let is_mutable = force_mutable;
        let mutability = if is_mutable {
            Mutability::Mutable
        } else {
            Mutability::Immutable
        };
        let local_id = ctx.local_scope.bind(
            field_name.clone(),
            binding_ty.clone(),
            is_mutable,
            span.clone(),
        );
        let pattern = Pattern::local(
            local_id,
            mutability,
            field_name.clone(),
            binding_ty,
            span.clone(),
        );

        StructPatternField::new(field_name, pattern, span)
    }
}

/// Resolve a range pattern (`0..=9` or `0..<10`).
fn resolve_range_pattern(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
) -> Pattern {
    let span = get_node_span(node, ctx.file_id);

    // Collect all tokens in order
    let tokens: Vec<_> = node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .collect();

    // We expect: start_literal, range_operator, end_literal
    // The range operator is ..= (inclusive) or ..< (exclusive)
    let mut start_bound: Option<RangeBound> = None;
    let mut end_bound: Option<RangeBound> = None;
    let mut inclusive = true;
    let mut found_operator = false;

    for token in &tokens {
        match token.kind() {
            SyntaxKind::Integer => {
                let text = token.text();
                let value = parse_integer(text);
                let bound = RangeBound::Integer(value);
                if !found_operator {
                    start_bound = Some(bound);
                } else {
                    end_bound = Some(bound);
                }
            },
            SyntaxKind::Char => {
                // Handle char literals like 'a', '\n', '\u{1F600}'
                let text = token.text();
                let text_range = token.text_range();
                let token_start: usize = text_range.start().into();

                if text.len() >= 2 {
                    let inner = &text[1..text.len() - 1];
                    // Process escape sequences
                    let unescaped = super::expressions::unescape_string(
                        inner,
                        ctx.file_id,
                        token_start + 1,
                        ctx,
                    );

                    if let Some(c) = unescaped.chars().next() {
                        let bound = RangeBound::Char(c);
                        if !found_operator {
                            start_bound = Some(bound);
                        } else {
                            end_bound = Some(bound);
                        }
                    }
                }
            },
            SyntaxKind::DotDotEquals => {
                found_operator = true;
                inclusive = true;
            },
            SyntaxKind::DotDotLess => {
                found_operator = true;
                inclusive = false;
            },
            _ => {},
        }
    }

    // Validate we have both bounds
    let (start, end) = match (start_bound, end_bound) {
        (Some(s), Some(e)) => (s, e),
        _ => return Pattern::error(span),
    };

    // Validate that start <= end (or start < end for exclusive ranges)
    let is_valid_range = match (&start, &end) {
        (RangeBound::Integer(s), RangeBound::Integer(e)) => {
            if inclusive {
                *s <= *e
            } else {
                *s < *e
            }
        },
        (RangeBound::Char(s), RangeBound::Char(e)) => {
            if inclusive {
                *s <= *e
            } else {
                *s < *e
            }
        },
        _ => false, // Mismatched types are invalid
    };

    if !is_valid_range {
        use crate::diagnostics::InvalidRangeBoundsError;
        let error = InvalidRangeBoundsError {
            span: span.clone(),
            inclusive,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Pattern::error(span);
    }

    // Determine the type based on the bounds
    let ty = match (&start, &end) {
        (RangeBound::Integer(_), RangeBound::Integer(_)) => expected_ty
            .cloned()
            .unwrap_or_else(|| Ty::int(kestrel_semantic_tree::ty::IntBits::I64, span.clone())),
        (RangeBound::Char(_), RangeBound::Char(_)) => {
            // We'd need a Char type here - for now use infer
            expected_ty
                .cloned()
                .unwrap_or_else(|| Ty::infer(span.clone()))
        },
        // Mismatched bounds (e.g., int..=char) - error
        _ => return Pattern::error(span),
    };

    Pattern::range(start, end, inclusive, ty, span)
}

/// Resolve an or-pattern (`p1 or p2 or ...`).
fn resolve_or_pattern(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
    force_mutable: bool,
) -> Pattern {
    use std::collections::HashMap;

    let span = get_node_span(node, ctx.file_id);

    // Collect all child pattern nodes (filter out the `or` keywords)
    let alt_nodes: Vec<_> = node
        .children()
        .filter(|c| is_pattern_kind(c.kind()))
        .collect();

    if alt_nodes.len() < 2 {
        // Or-patterns need at least 2 alternatives
        return Pattern::error(span);
    }

    // Snapshot bindings before resolving any alternatives
    let pre_or_bindings = ctx.local_scope.snapshot_bindings();

    // Resolve the first alternative (this creates the "canonical" bindings)
    let first_alt = resolve_pattern_inner(
        &alt_nodes[0],
        ctx,
        expected_ty,
        get_node_span(&alt_nodes[0], ctx.file_id),
        force_mutable,
    );

    // Snapshot the first alternative's bindings (these are what the arm body should see)
    let first_alt_bindings = ctx.local_scope.snapshot_bindings();

    let mut alternatives = vec![first_alt];

    // Resolve remaining alternatives, restoring bindings before each
    for alt_node in alt_nodes.iter().skip(1) {
        // Restore to pre-or-pattern state before resolving this alternative
        ctx.local_scope.restore_bindings(pre_or_bindings.clone());

        let alt = resolve_pattern_inner(
            alt_node,
            ctx,
            expected_ty,
            get_node_span(alt_node, ctx.file_id),
            force_mutable,
        );
        alternatives.push(alt);
    }

    // Restore the first alternative's bindings so the arm body sees the correct LocalIds
    ctx.local_scope.restore_bindings(first_alt_bindings);

    // Validate that all alternatives bind the same names
    fn collect_bindings(pattern: &Pattern) -> HashMap<String, Ty> {
        let mut bindings = HashMap::new();
        collect_bindings_inner(pattern, &mut bindings);
        bindings
    }

    fn collect_bindings_inner(pattern: &Pattern, bindings: &mut HashMap<String, Ty>) {
        use kestrel_semantic_tree::pattern::PatternKind;
        match &pattern.kind {
            PatternKind::Local { name, .. } => {
                bindings.insert(name.clone(), pattern.ty.clone());
            },
            PatternKind::Tuple { prefix, suffix, .. } => {
                for elem in prefix.iter().chain(suffix.iter()) {
                    collect_bindings_inner(elem, bindings);
                }
            },
            PatternKind::EnumVariant {
                bindings: enum_bindings,
                ..
            } => {
                for binding in enum_bindings {
                    collect_bindings_inner(&binding.pattern, bindings);
                }
            },
            PatternKind::Or { alternatives } => {
                // For nested or-patterns, use the first alternative's bindings
                if let Some(first) = alternatives.first() {
                    collect_bindings_inner(first, bindings);
                }
            },
            _ => {},
        }
    }

    // Get bindings from first alternative as reference
    let first_bindings = collect_bindings(&alternatives[0]);

    // Check all alternatives have the same bindings
    for (i, alt) in alternatives.iter().enumerate().skip(1) {
        let alt_bindings = collect_bindings(alt);

        // Check that both have the same set of names
        let first_names: std::collections::HashSet<_> = first_bindings.keys().collect();
        let alt_names: std::collections::HashSet<_> = alt_bindings.keys().collect();

        if first_names != alt_names {
            use crate::diagnostics::InconsistentOrPatternBindingsError;
            let error = InconsistentOrPatternBindingsError {
                span: span.clone(),
                alternative_index: i + 1,
            };
            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            return Pattern::error(span);
        }

        // TODO: Also check that the types are compatible
    }

    // Type is the type of the first alternative (they should all be compatible)
    let ty = expected_ty
        .cloned()
        .unwrap_or_else(|| alternatives[0].ty.clone());

    Pattern::or_pattern(alternatives, ty, span)
}

/// Resolve an array pattern (`[a, b, ..rest]`).
fn resolve_array_pattern(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
    force_mutable: bool,
) -> Pattern {
    let span = get_node_span(node, ctx.file_id);

    // Get expected element type if we have an array type
    let expected_element_ty: Option<&Ty> = expected_ty.and_then(|ty| {
        if let kestrel_semantic_tree::ty::TyKind::Array(element_type) = ty.kind() {
            Some(element_type.as_ref())
        } else {
            None
        }
    });

    let mut prefix: Vec<Pattern> = Vec::new();
    let mut suffix: Vec<Pattern> = Vec::new();
    let mut rest: Option<(
        Option<String>,
        Option<kestrel_semantic_tree::symbol::local::LocalId>,
    )> = None;
    let mut in_suffix = false;

    for child in node.children() {
        match child.kind() {
            SyntaxKind::ArrayPatternElement => {
                // Get the inner pattern
                let pattern = if let Some(inner) = child.children().next() {
                    resolve_pattern_with_mutability(&inner, ctx, expected_element_ty, force_mutable)
                } else {
                    Pattern::error(get_node_span(&child, ctx.file_id))
                };

                if in_suffix {
                    suffix.push(pattern);
                } else {
                    prefix.push(pattern);
                }
            },
            SyntaxKind::ArrayPatternRest => {
                // We're now in the suffix
                in_suffix = true;

                // Check if there's a named binding
                let name_token = child
                    .children_with_tokens()
                    .filter_map(|elem| elem.into_token())
                    .find(|t| t.kind() == SyntaxKind::Identifier);

                if let Some(token) = name_token {
                    let name = token.text().to_string();
                    let name_span = {
                        let text_range = token.text_range();
                        Span::new(
                            ctx.file_id,
                            text_range.start().into()..text_range.end().into(),
                        )
                    };

                    // The rest binding will be a slice/array of the element type
                    let rest_ty = expected_element_ty
                        .cloned()
                        .map(|elem_ty| Ty::array(elem_ty, span.clone()))
                        .unwrap_or_else(|| Ty::infer(span.clone()));

                    let is_mutable = force_mutable;
                    let local_id =
                        ctx.local_scope
                            .bind(name.clone(), rest_ty, is_mutable, name_span);
                    rest = Some((Some(name), Some(local_id)));
                } else {
                    // Anonymous rest: `..` - just ignore remaining elements
                    rest = Some((None, None));
                }
            },
            _ => {},
        }
    }

    // Check for suffix elements - not yet supported
    if !suffix.is_empty() {
        use crate::diagnostics::ArraySuffixPatternError;
        let error = ArraySuffixPatternError { span: span.clone() };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Pattern::error(span);
    }

    // Build element type from patterns
    let element_ty = if let Some(first) = prefix.first() {
        first.ty.clone()
    } else {
        expected_element_ty
            .cloned()
            .unwrap_or_else(|| Ty::infer(span.clone()))
    };

    let ty = expected_ty
        .cloned()
        .unwrap_or_else(|| Ty::array(element_ty, span.clone()));

    Pattern::array(prefix, rest, suffix, ty, span)
}

/// Resolve an @-pattern (`name @ subpattern`).
fn resolve_at_pattern(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    expected_ty: Option<&Ty>,
    force_mutable: bool,
) -> Pattern {
    let span = get_node_span(node, ctx.file_id);

    // Check for `var` keyword (mutable binding) or force_mutable from statement level
    let has_var_keyword = node
        .children_with_tokens()
        .any(|elem| elem.kind() == SyntaxKind::Var);
    let is_mutable = force_mutable || has_var_keyword;
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

    // Get span for the name token
    let name_span = node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| {
            let text_range = t.text_range();
            Span::new(
                ctx.file_id,
                text_range.start().into()..text_range.end().into(),
            )
        })
        .unwrap_or_else(|| span.clone());

    // Find and resolve the subpattern
    let subpattern = node
        .children()
        .find(|c| is_pattern_kind(c.kind()))
        .map(|p| resolve_pattern_with_mutability(&p, ctx, expected_ty, force_mutable))
        .unwrap_or_else(|| Pattern::error(span.clone()));

    // Check for nested @ patterns - these are not allowed
    if subpattern.is_at() {
        use crate::diagnostics::NestedAtPatternError;
        let error = NestedAtPatternError { span: span.clone() };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Pattern::error(span);
    }

    // Determine type from expected type or subpattern's type
    let ty = expected_ty
        .cloned()
        .unwrap_or_else(|| subpattern.ty.clone());

    // Bind the local variable
    let local_id = ctx
        .local_scope
        .bind(name.clone(), ty.clone(), is_mutable, name_span);

    Pattern::at_pattern(name, local_id, mutability, subpattern, ty, span)
}

/// Resolve a rest pattern (`..`).
fn resolve_rest_pattern(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
    _expected_ty: Option<&Ty>,
) -> Pattern {
    let span = get_node_span(node, ctx.file_id);
    // Rest patterns have unit type (they don't bind a value, just match remaining elements)
    let ty = Ty::unit(span.clone());
    Pattern::rest(ty, span)
}

/// Check if a SyntaxKind is a pattern kind.
pub fn is_pattern_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Pattern
            | SyntaxKind::WildcardPattern
            | SyntaxKind::BindingPattern
            | SyntaxKind::TuplePattern
            | SyntaxKind::LiteralPattern
            | SyntaxKind::EnumPattern
            | SyntaxKind::RangePattern
            | SyntaxKind::OrPattern
            | SyntaxKind::StructPattern
            | SyntaxKind::ArrayPattern
            | SyntaxKind::AtPattern
            | SyntaxKind::RestPattern
            | SyntaxKind::ErrorPattern
    )
}

/// Check for duplicate binding names within a pattern and report errors.
fn check_duplicate_bindings(pattern: &Pattern, ctx: &mut BodyResolutionContext) {
    let mut bindings: HashMap<String, Span> = HashMap::new();
    collect_bindings_for_duplicate_check(pattern, &mut bindings, ctx);
}

/// Recursively collect binding names and check for duplicates.
fn collect_bindings_for_duplicate_check(
    pattern: &Pattern,
    bindings: &mut HashMap<String, Span>,
    ctx: &mut BodyResolutionContext,
) {
    match &pattern.kind {
        PatternKind::Local { name, .. } => {
            if let Some(first_span) = bindings.get(name) {
                use crate::diagnostics::DuplicateBindingInPatternError;
                let error = DuplicateBindingInPatternError {
                    span: pattern.span.clone(),
                    name: name.clone(),
                    first_span: first_span.clone(),
                };
                ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            } else {
                bindings.insert(name.clone(), pattern.span.clone());
            }
        },
        PatternKind::Tuple { prefix, suffix, .. } => {
            for elem in prefix.iter().chain(suffix.iter()) {
                collect_bindings_for_duplicate_check(elem, bindings, ctx);
            }
        },
        PatternKind::EnumVariant {
            bindings: enum_bindings,
            ..
        } => {
            for binding in enum_bindings {
                collect_bindings_for_duplicate_check(&binding.pattern, bindings, ctx);
            }
        },
        PatternKind::Struct { fields, .. } => {
            for field in fields {
                collect_bindings_for_duplicate_check(&field.pattern, bindings, ctx);
            }
        },
        PatternKind::Array { prefix, suffix, .. } => {
            for elem in prefix {
                collect_bindings_for_duplicate_check(elem, bindings, ctx);
            }
            for elem in suffix {
                collect_bindings_for_duplicate_check(elem, bindings, ctx);
            }
        },
        PatternKind::Or { .. } => {
            // For or-patterns, each alternative can have the same bindings
            // (they should have the same set of names). Don't check across alternatives.
        },
        PatternKind::At {
            name, subpattern, ..
        } => {
            if let Some(first_span) = bindings.get(name) {
                use crate::diagnostics::DuplicateBindingInPatternError;
                let error = DuplicateBindingInPatternError {
                    span: pattern.span.clone(),
                    name: name.clone(),
                    first_span: first_span.clone(),
                };
                ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            } else {
                bindings.insert(name.clone(), pattern.span.clone());
            }
            collect_bindings_for_duplicate_check(subpattern, bindings, ctx);
        },
        PatternKind::Wildcard
        | PatternKind::Literal { .. }
        | PatternKind::Range { .. }
        | PatternKind::Rest
        | PatternKind::Error => {},
    }
}
