//! Attribute resolution utilities for the binder.
//!
//! This module provides functions to extract and resolve attributes from syntax nodes.

use crate::diagnostics::{
    BuiltinInvalidArgumentError, BuiltinRequiresArgumentError, UnknownAttributeWarning,
    UnknownLanguageFeatureError,
};
use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_tree::attributes::{Attribute, AttributeArg, AttributeKind};
use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;
use kestrel_semantic_tree::builtins::LanguageFeature;
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

/// Result of parsing a `@builtin(.Feature)` attribute.
pub enum BuiltinParseResult {
    /// Successfully parsed: contains the language feature
    Success(LanguageFeature),
    /// Not a builtin attribute
    NotBuiltin,
    /// Error occurred during parsing (diagnostic already emitted)
    Error,
}

/// Parse a `@builtin(.Feature)` attribute from an AttributesBehavior.
///
/// This function checks if the attributes contain a `@builtin` attribute,
/// validates its arguments, and returns the parsed `LanguageFeature`.
///
/// # Arguments
/// * `attributes` - The resolved attributes behavior
/// * `source` - The source text (needed to extract implicit member name)
/// * `diagnostics` - The diagnostic context for emitting errors
///
/// # Returns
/// - `BuiltinParseResult::Success(feature)` if a valid builtin attribute was found
/// - `BuiltinParseResult::NotBuiltin` if no builtin attribute is present
/// - `BuiltinParseResult::Error` if a builtin attribute is present but invalid
pub fn parse_builtin_attribute(
    attributes: &AttributesBehavior,
    source: &str,
    diagnostics: &mut DiagnosticContext,
) -> BuiltinParseResult {
    // Find the @builtin attribute
    let Some(attr) = attributes.get_kind(AttributeKind::Builtin) else {
        return BuiltinParseResult::NotBuiltin;
    };

    // Validate: must have exactly one argument
    if attr.args.is_empty() {
        diagnostics.throw(BuiltinRequiresArgumentError {
            span: attr.span.clone(),
        });
        return BuiltinParseResult::Error;
    }

    let arg = &attr.args[0];

    // Validate: argument must be unlabeled (no `label: .Value` syntax)
    if arg.is_labeled() {
        diagnostics.throw(BuiltinInvalidArgumentError {
            span: arg.span.clone(),
        });
        return BuiltinParseResult::Error;
    }

    // Extract the feature name from the source using the value span
    // The argument should be `.FeatureName` (implicit member syntax)
    let arg_text = &source[arg.value_span.range()];

    // Must start with '.' for implicit member syntax
    if !arg_text.starts_with('.') {
        diagnostics.throw(BuiltinInvalidArgumentError {
            span: arg.span.clone(),
        });
        return BuiltinParseResult::Error;
    }

    // Extract the feature name (everything after the '.')
    let feature_name = &arg_text[1..];

    // Parse the feature name
    match LanguageFeature::from_name(feature_name) {
        Some(feature) => BuiltinParseResult::Success(feature),
        None => {
            diagnostics.throw(UnknownLanguageFeatureError {
                span: arg.value_span.clone(),
                name: feature_name.to_string(),
            });
            BuiltinParseResult::Error
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require constructing syntax nodes, which is complex.
    // The actual testing happens through the integration tests.
}
