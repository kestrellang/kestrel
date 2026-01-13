//! Attribute resolution utilities for the binder.
//!
//! This module provides functions to extract and resolve attributes from syntax nodes.

use crate::diagnostics::{
    BuiltinInvalidArgumentError, BuiltinRequiresArgumentError, ExternInvalidCallingConventionError,
    ExternRequiresCallingConventionError, ExternUnknownCallingConventionError,
    UnknownAttributeWarning, UnknownLanguageFeatureError,
};
use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_tree::attributes::{Attribute, AttributeArg, AttributeKind};
use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;
use kestrel_semantic_tree::behavior::extern_fn::CallingConvention;
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
    file_id: usize,
    diagnostics: &mut DiagnosticContext,
) -> AttributesBehavior {
    let attributes = extract_attributes(syntax, source, file_id);

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
fn extract_attributes(syntax: &SyntaxNode, source: &str, file_id: usize) -> Vec<Attribute> {
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
        if let Some(attr) = extract_single_attribute(&attr_node, source, file_id) {
            attributes.push(attr);
        }
    }

    attributes
}

/// Extract a single attribute from an Attribute syntax node.
fn extract_single_attribute(
    node: &SyntaxNode,
    source: &str,
    file_id: usize,
) -> Option<Attribute> {
    // Find the @ token and the identifier (name)
    let mut name: Option<String> = None;
    let mut at_span: Option<Span> = None;
    let mut name_end: usize = 0;

    for child in node.children_with_tokens() {
        if let Some(tok) = child.into_token() {
            if tok.kind() == SyntaxKind::At {
                at_span = Some(Span::new(
                    file_id,
                    tok.text_range().start().into()..tok.text_range().end().into(),
                ));
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
    let args = extract_attribute_args(node, source, file_id);

    // Calculate the full span (from @ to end of args or end of name)
    let attr_end = if let Some(args_node) = node
        .children()
        .find(|c| c.kind() == SyntaxKind::AttributeArgs)
    {
        args_node.text_range().end().into()
    } else {
        name_end
    };

    let full_span = Span::new(start_span.file_id, start_span.start..attr_end);

    Some(Attribute::new(attr_name, args, full_span))
}

/// Extract arguments from an AttributeArgs node.
fn extract_attribute_args(
    attr_node: &SyntaxNode,
    source: &str,
    file_id: usize,
) -> Vec<AttributeArg> {
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
        if let Some(arg) = extract_single_arg(&arg_node, source, file_id) {
            args.push(arg);
        }
    }

    args
}

/// Extract a single argument from an AttributeArg node.
fn extract_single_arg(node: &SyntaxNode, _source: &str, file_id: usize) -> Option<AttributeArg> {
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
                    value_span = Some(Span::new(
                        file_id,
                        tok.text_range().start().into()..tok.text_range().end().into(),
                    ));
                }
                SyntaxKind::String
                | SyntaxKind::Integer
                | SyntaxKind::Float
                | SyntaxKind::Boolean => {
                    value_span = Some(Span::new(
                        file_id,
                        tok.text_range().start().into()..tok.text_range().end().into(),
                    ));
                }
                SyntaxKind::Dot => {
                    // Part of implicit member access - skip
                }
                _ => {}
            }
        }
    }

    let full_span = Span::new(file_id, arg_start..arg_end);

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

/// Result of parsing a `@extern(.C)` attribute.
pub enum ExternParseResult {
    /// Successfully parsed
    Success {
        calling_convention: CallingConvention,
        mangle_name: Option<String>,
    },
    /// Not an extern attribute
    NotExtern,
    /// Error occurred during parsing (diagnostic already emitted)
    Error,
}

/// Parse a `@extern(.C, mangleName: "...")` attribute from an AttributesBehavior.
///
/// This function checks if the attributes contain an `@extern` attribute,
/// validates its arguments, and returns the parsed calling convention and
/// optional mangle name.
///
/// # Arguments
/// * `attributes` - The resolved attributes behavior
/// * `source` - The source text (needed to extract values)
/// * `diagnostics` - The diagnostic context for emitting errors
///
/// # Returns
/// - `ExternParseResult::Success { ... }` if a valid extern attribute was found
/// - `ExternParseResult::NotExtern` if no extern attribute is present
/// - `ExternParseResult::Error` if an extern attribute is present but invalid
pub fn parse_extern_attribute(
    attributes: &AttributesBehavior,
    source: &str,
    diagnostics: &mut DiagnosticContext,
) -> ExternParseResult {
    // Find the @extern attribute
    let Some(attr) = attributes.get_kind(AttributeKind::Extern) else {
        return ExternParseResult::NotExtern;
    };

    // Must have at least one argument (calling convention)
    if attr.args.is_empty() {
        diagnostics.throw(ExternRequiresCallingConventionError {
            span: attr.span.clone(),
        });
        return ExternParseResult::Error;
    }

    let conv_arg = &attr.args[0];

    // First arg must be unlabeled (implicit member like .C)
    if conv_arg.is_labeled() {
        diagnostics.throw(ExternInvalidCallingConventionError {
            span: conv_arg.span.clone(),
        });
        return ExternParseResult::Error;
    }

    // Extract the calling convention from the source
    let arg_text = &source[conv_arg.value_span.range()];

    // Must start with '.' for implicit member syntax
    if !arg_text.starts_with('.') {
        diagnostics.throw(ExternInvalidCallingConventionError {
            span: conv_arg.span.clone(),
        });
        return ExternParseResult::Error;
    }

    // Parse the calling convention
    let conv_name = &arg_text[1..];
    let calling_convention = match conv_name {
        "C" => CallingConvention::C,
        _ => {
            diagnostics.throw(ExternUnknownCallingConventionError {
                span: conv_arg.value_span.clone(),
                name: conv_name.to_string(),
            });
            return ExternParseResult::Error;
        }
    };

    // Check for optional mangleName parameter
    let mangle_name = attr.args.get(1).and_then(|arg| {
        if arg.label.as_deref() == Some("mangleName") {
            let val_text = &source[arg.value_span.range()];
            // Remove quotes from string literal
            if val_text.starts_with('"') && val_text.ends_with('"') && val_text.len() >= 2 {
                Some(val_text[1..val_text.len() - 1].to_string())
            } else {
                // Not a valid string literal, but we'll silently ignore for now
                // Could add a diagnostic here
                None
            }
        } else {
            None
        }
    });

    ExternParseResult::Success {
        calling_convention,
        mangle_name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require constructing syntax nodes, which is complex.
    // The actual testing happens through the integration tests.
}
