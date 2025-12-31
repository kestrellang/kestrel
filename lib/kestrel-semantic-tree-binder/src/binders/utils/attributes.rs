//! Attribute resolution utilities for the binder.
//!
//! This module provides functions to extract and resolve attributes from syntax nodes.

use crate::diagnostics::UnknownAttributeWarning;
use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_tree::attributes::{Attribute, AttributeArg};
use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

/// Resolve attributes from a declaration syntax node.
///
/// This function extracts any attributes from the AttributeList child of the
/// given syntax node and creates an AttributesBehavior with resolved attributes.
/// It also emits warnings for unknown attributes.
///
/// # Arguments
/// * `syntax` - The declaration syntax node (e.g., FunctionDeclaration, StructDeclaration)
/// * `source` - The source text
/// * `diagnostics` - The diagnostic context for emitting warnings
///
/// # Returns
/// An AttributesBehavior containing all resolved attributes, or an empty one if none.
pub fn resolve_attributes(
    syntax: &SyntaxNode,
    source: &str,
    diagnostics: &mut DiagnosticContext,
) -> AttributesBehavior {
    let attributes = extract_attributes(syntax, source);

    // Emit warnings for unknown attributes
    for attr in &attributes {
        if attr.is_unknown() {
            diagnostics.throw(UnknownAttributeWarning {
                name: attr.name.clone(),
                span: attr.span.clone(),
            });
        }
    }

    AttributesBehavior::new(attributes)
}

/// Extract attributes from a syntax node.
///
/// Looks for an AttributeList child and extracts all Attribute nodes from it.
fn extract_attributes(syntax: &SyntaxNode, source: &str) -> Vec<Attribute> {
    let mut attributes = Vec::new();

    // Find the AttributeList child
    let Some(attr_list) = syntax
        .children()
        .find(|c| c.kind() == SyntaxKind::AttributeList)
    else {
        return attributes;
    };

    // Extract each Attribute from the list
    for attr_node in attr_list
        .children()
        .filter(|c| c.kind() == SyntaxKind::Attribute)
    {
        if let Some(attr) = extract_single_attribute(&attr_node, source) {
            attributes.push(attr);
        }
    }

    attributes
}

/// Extract a single attribute from an Attribute syntax node.
fn extract_single_attribute(node: &SyntaxNode, source: &str) -> Option<Attribute> {
    // Find the @ token and the identifier (name)
    let mut name: Option<String> = None;
    let mut at_span: Option<Span> = None;
    let mut name_end: usize = 0;

    for child in node.children_with_tokens() {
        if let Some(tok) = child.into_token() {
            if tok.kind() == SyntaxKind::At {
                at_span = Some(Span::from(tok.text_range().start().into()..tok.text_range().end().into()));
            } else if tok.kind() == SyntaxKind::Identifier && name.is_none() {
                name = Some(tok.text().to_string());
                name_end = tok.text_range().end().into();
            }
        }
    }

    let Some(attr_name) = name else {
        return None;
    };
    let Some(start_span) = at_span else {
        return None;
    };

    // Extract arguments if present
    let args = extract_attribute_args(node, source);

    // Calculate the full span (from @ to end of args or end of name)
    let attr_end = if let Some(args_node) = node
        .children()
        .find(|c| c.kind() == SyntaxKind::AttributeArgs)
    {
        args_node.text_range().end().into()
    } else {
        name_end
    };

    let full_span = Span::from(start_span.start..attr_end);

    Some(Attribute::new(attr_name, args, full_span))
}

/// Extract arguments from an AttributeArgs node.
fn extract_attribute_args(attr_node: &SyntaxNode, source: &str) -> Vec<AttributeArg> {
    let mut args = Vec::new();

    let Some(args_node) = attr_node
        .children()
        .find(|c| c.kind() == SyntaxKind::AttributeArgs)
    else {
        return args;
    };

    // Extract each AttributeArg
    for arg_node in args_node
        .children()
        .filter(|c| c.kind() == SyntaxKind::AttributeArg)
    {
        if let Some(arg) = extract_single_arg(&arg_node, source) {
            args.push(arg);
        }
    }

    args
}

/// Extract a single argument from an AttributeArg node.
fn extract_single_arg(node: &SyntaxNode, _source: &str) -> Option<AttributeArg> {
    let mut label: Option<String> = None;
    let mut has_colon = false;
    let mut value_span: Option<Span> = None;
    let arg_start: usize = node.text_range().start().into();
    let arg_end: usize = node.text_range().end().into();

    for child in node.children_with_tokens() {
        if let Some(tok) = child.into_token() {
            match tok.kind() {
                SyntaxKind::Identifier if !has_colon && label.is_none() => {
                    // This could be a label or a path value
                    // We'll treat it as a label if followed by colon
                    label = Some(tok.text().to_string());
                }
                SyntaxKind::Colon => {
                    has_colon = true;
                }
                SyntaxKind::Identifier if has_colon => {
                    // This is a value identifier (after colon)
                    value_span = Some(Span::from(tok.text_range().start().into()..tok.text_range().end().into()));
                }
                SyntaxKind::String
                | SyntaxKind::Integer
                | SyntaxKind::Float
                | SyntaxKind::Boolean => {
                    value_span = Some(Span::from(tok.text_range().start().into()..tok.text_range().end().into()));
                }
                SyntaxKind::Dot => {
                    // Part of implicit member access - skip
                }
                _ => {}
            }
        }
    }

    let full_span = Span::from(arg_start..arg_end);

    // Determine if this is a labeled or unlabeled argument
    if has_colon {
        // Labeled: label: value
        let label_str = label?;
        // If no value_span, use the full span minus the label
        let val_span = value_span.unwrap_or_else(|| full_span.clone());
        Some(AttributeArg::labeled(label_str, val_span, full_span))
    } else if label.is_some() {
        // Unlabeled path value (identifier was stored in label)
        // The value_span is the whole arg span since it's just an identifier
        Some(AttributeArg::unlabeled(full_span))
    } else if let Some(val) = value_span {
        // Unlabeled literal value
        Some(AttributeArg::unlabeled(val))
    } else {
        // Unable to extract meaningful data
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require constructing syntax nodes, which is complex.
    // The actual testing happens through the integration tests.
}
