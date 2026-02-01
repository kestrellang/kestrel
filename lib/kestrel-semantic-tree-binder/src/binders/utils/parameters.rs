use kestrel_semantic_tree::behavior::callable::ParameterAccessMode;
use kestrel_semantic_tree::symbol::function::Parameter;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Spanned;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::declaration_binder::BindingContext;
use crate::diagnostics::RequiredParameterAfterDefaultError;
use crate::resolution::type_resolver::{TypeSyntaxContext, resolve_type_from_ty_node};
use kestrel_reporting::DiagnosticContext;
use kestrel_syntax_tree::utils::{extract_identifier_from_name, find_child, get_node_span};

pub(crate) fn resolve_parameters_from_syntax(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    implicit_labels: bool,
) -> Vec<Parameter> {
    let param_list = match find_child(syntax, SyntaxKind::ParameterList) {
        Some(node) => node,
        None => return Vec::new(),
    };

    param_list
        .children()
        .filter(|child| child.kind() == SyntaxKind::Parameter)
        .filter_map(|param_node| {
            resolve_single_parameter(
                &param_node,
                source,
                file_id,
                context_id,
                ctx,
                implicit_labels,
            )
        })
        .collect()
}

/// Extract access mode from a parameter syntax node.
///
/// Looks for `Mutating` or `Consuming` tokens as direct children of the parameter node.
fn extract_access_mode(param_node: &SyntaxNode) -> ParameterAccessMode {
    for child in param_node.children_with_tokens() {
        if let Some(token) = child.as_token() {
            match token.kind() {
                SyntaxKind::Mutating => return ParameterAccessMode::Mutating,
                SyntaxKind::Consuming => return ParameterAccessMode::Consuming,
                _ => {},
            }
        }
    }
    ParameterAccessMode::Borrow
}

/// Extract the primary binding name from a pattern.
///
/// For simple binding patterns like `x`, returns the identifier.
/// For complex patterns like `(a, b)`, returns the first binding name found.
/// For wildcards `_`, returns None.
fn extract_primary_name_from_pattern(pattern_node: &SyntaxNode, file_id: usize) -> Option<(String, kestrel_span::Span)> {
    fn extract_recursive(node: &SyntaxNode, file_id: usize) -> Option<(String, kestrel_span::Span)> {
        match node.kind() {
            SyntaxKind::Pattern => {
                // Pattern wrapper - recurse into first child
                for child in node.children() {
                    if let Some(result) = extract_recursive(&child, file_id) {
                        return Some(result);
                    }
                }
                None
            }
            SyntaxKind::BindingPattern => {
                // Extract the identifier from the binding pattern
                for child in node.children_with_tokens() {
                    if let Some(token) = child.as_token() {
                        if token.kind() == SyntaxKind::Identifier {
                            let span = get_node_span(node, file_id);
                            return Some((token.text().to_string(), span));
                        }
                    }
                }
                None
            }
            SyntaxKind::TuplePattern => {
                // Return first binding in the tuple
                for child in node.children() {
                    if let Some(result) = extract_recursive(&child, file_id) {
                        return Some(result);
                    }
                }
                None
            }
            SyntaxKind::TuplePatternElement => {
                // Recurse into element
                for child in node.children() {
                    if let Some(result) = extract_recursive(&child, file_id) {
                        return Some(result);
                    }
                }
                None
            }
            SyntaxKind::StructPattern => {
                // Return first field binding
                for child in node.children() {
                    if child.kind() == SyntaxKind::StructPatternField {
                        if let Some(result) = extract_recursive(&child, file_id) {
                            return Some(result);
                        }
                    }
                }
                None
            }
            SyntaxKind::StructPatternField => {
                // Check for explicit binding or shorthand
                for inner in node.children() {
                    if inner.kind() == SyntaxKind::Pattern
                        || inner.kind() == SyntaxKind::BindingPattern
                    {
                        if let Some(result) = extract_recursive(&inner, file_id) {
                            return Some(result);
                        }
                    }
                }
                // Shorthand: use field name
                if let Some(name_node) = node.children().find(|c| c.kind() == SyntaxKind::Name) {
                    for token in name_node.children_with_tokens() {
                        if let Some(t) = token.as_token() {
                            if t.kind() == SyntaxKind::Identifier {
                                let span = get_node_span(&name_node, file_id);
                                return Some((t.text().to_string(), span));
                            }
                        }
                    }
                }
                None
            }
            SyntaxKind::WildcardPattern => None, // Wildcards have no binding name
            _ => {
                // For other nodes, recurse into children
                for child in node.children() {
                    if let Some(result) = extract_recursive(&child, file_id) {
                        return Some(result);
                    }
                }
                None
            }
        }
    }

    extract_recursive(pattern_node, file_id)
}

fn resolve_single_parameter(
    param_node: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    implicit_labels: bool,
) -> Option<Parameter> {
    // Extract access mode first (mutating/consuming keyword)
    let access_mode = extract_access_mode(param_node);

    // Collect Name nodes (for labels) and Pattern nodes (for bindings)
    let name_nodes: Vec<SyntaxNode> = param_node
        .children()
        .filter(|child| child.kind() == SyntaxKind::Name)
        .collect();

    let pattern_node = param_node
        .children()
        .find(|child| child.kind() == SyntaxKind::Pattern);

    // Determine label and bind_name based on what nodes are present
    let (label, bind_name) = if let Some(pattern) = &pattern_node {
        // New-style parameter with pattern
        // If there's a Name node, it's the label; the pattern provides the binding
        let label = if !name_nodes.is_empty() {
            extract_identifier_from_name(&name_nodes[0])
                .map(|n| Spanned::new(n, get_node_span(&name_nodes[0], file_id)))
        } else {
            None
        };

        // Extract primary name from pattern for the bind_name field
        let (name, span) = extract_primary_name_from_pattern(pattern, file_id)
            .unwrap_or_else(|| {
                // Fallback for wildcards or unparseable patterns
                ("_".to_string(), get_node_span(pattern, file_id))
            });
        let bind_name = Spanned::new(name, span);

        (label, bind_name)
    } else if !name_nodes.is_empty() {
        // Old-style parameter with Name nodes only (backward compatibility)
        if name_nodes.len() >= 2 {
            let label_name = extract_identifier_from_name(&name_nodes[0]);
            let bind_name = Spanned::new(
                extract_identifier_from_name(&name_nodes[1])?,
                get_node_span(&name_nodes[1], file_id),
            );
            (
                label_name.map(|n| Spanned::new(n, get_node_span(&name_nodes[0], file_id))),
                bind_name,
            )
        } else {
            // Single name - use it as both the label and internal binding name.
            let name = extract_identifier_from_name(&name_nodes[0])?;
            let span = get_node_span(&name_nodes[0], file_id);
            let label = implicit_labels.then(|| Spanned::new(name.clone(), span.clone()));
            let bind_name = Spanned::new(name, span);
            (label, bind_name)
        }
    } else {
        // No pattern or name found - invalid parameter
        return None;
    };

    let ty = if let Some(ty_node) = param_node.children().find(|c| c.kind() == SyntaxKind::Ty) {
        let mut type_ctx =
            TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
        resolve_type_from_ty_node(&ty_node, &mut type_ctx)
    } else {
        Ty::infer(get_node_span(param_node, file_id))
    };

    // Check if there's a DefaultValue child node
    let has_default = param_node
        .children()
        .any(|c| c.kind() == SyntaxKind::DefaultValue);

    Some(Parameter {
        access_mode,
        label,
        bind_name,
        ty,
        has_default,
    })
}

/// Validates that required parameters do not follow parameters with default values.
///
/// Once a parameter has a default value, all subsequent parameters must also have defaults.
/// This ensures call-site argument matching is unambiguous.
pub(crate) fn validate_default_parameter_order(
    params: &[Parameter],
    diagnostics: &mut DiagnosticContext,
) {
    let mut first_default_param: Option<&Parameter> = None;

    for param in params {
        if param.has_default {
            // Track the first parameter with a default
            if first_default_param.is_none() {
                first_default_param = Some(param);
            }
        } else if let Some(default_param) = first_default_param {
            // Error: required parameter after a default parameter
            diagnostics.throw(RequiredParameterAfterDefaultError {
                required_name: param.bind_name.value.clone(),
                required_span: param.bind_name.span.clone(),
                default_param_name: default_param.bind_name.value.clone(),
                default_param_span: default_param.bind_name.span.clone(),
            });
        }
    }
}
