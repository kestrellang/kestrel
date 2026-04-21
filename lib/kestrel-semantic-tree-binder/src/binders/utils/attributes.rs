//! Attribute resolution utilities for the binder.
//!
//! This module provides functions to extract and resolve attributes from syntax nodes.

use crate::diagnostics::{
    BuiltinInvalidArgumentError, BuiltinRequiresArgumentError, BuiltinWrongKindError,
    DuplicateBuiltinError, ExternInvalidCallingConventionError,
    ExternRequiresCallingConventionError, ExternUnknownCallingConventionError,
    PlatformInvalidArgumentError, PlatformRequiresArgumentError, PlatformUnknownPlatformError,
    UnknownAttributeWarning, UnknownLanguageFeatureError,
};
use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_tree::attributes::{Attribute, AttributeArg, AttributeKind};
use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;
use kestrel_semantic_tree::behavior::extern_fn::CallingConvention;
use kestrel_semantic_tree::builtins::LanguageFeature;
use kestrel_semantic_tree::platform::TargetPlatform;
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
fn extract_single_attribute(node: &SyntaxNode, source: &str, file_id: usize) -> Option<Attribute> {
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

    let attr_name = name?;
    let start_span = at_span?;

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
                },
                SyntaxKind::Colon => {
                    has_colon = true;
                },
                SyntaxKind::Identifier if has_colon => {
                    // This is a value identifier (after colon)
                    value_span = Some(Span::new(
                        file_id,
                        tok.text_range().start().into()..tok.text_range().end().into(),
                    ));
                },
                SyntaxKind::String
                | SyntaxKind::Integer
                | SyntaxKind::Float
                | SyntaxKind::Boolean => {
                    value_span = Some(Span::new(
                        file_id,
                        tok.text_range().start().into()..tok.text_range().end().into(),
                    ));
                },
                SyntaxKind::Dot => {
                    // Part of implicit member access - skip
                },
                _ => {},
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
    } else {
        value_span.map(AttributeArg::unlabeled)
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
        },
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
        },
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

/// Result of parsing a `@fileconstant("path")` attribute.
pub enum FileConstantParseResult {
    /// Successfully parsed: contains the relative file path
    Success { relative_path: String, span: Span },
    /// Not a fileconstant attribute
    NotFileConstant,
    /// Error occurred during parsing (diagnostic already emitted)
    Error,
}

/// Parse a `@fileconstant("path.bin")` attribute from an AttributesBehavior.
///
/// This function checks if the attributes contain a `@fileconstant` attribute,
/// validates its arguments, and returns the parsed file path.
///
/// # Arguments
/// * `attributes` - The resolved attributes behavior
/// * `source` - The source text (needed to extract string value)
/// * `diagnostics` - The diagnostic context for emitting errors
///
/// # Returns
/// - `FileConstantParseResult::Success { ... }` if a valid fileconstant attribute was found
/// - `FileConstantParseResult::NotFileConstant` if no fileconstant attribute is present
/// - `FileConstantParseResult::Error` if a fileconstant attribute is present but invalid
pub fn parse_fileconstant_attribute(
    attributes: &AttributesBehavior,
    source: &str,
    diagnostics: &mut DiagnosticContext,
) -> FileConstantParseResult {
    use crate::diagnostics::{
        FileConstantInvalidArgumentError, FileConstantRequiresPathError,
        FileConstantRequiresStringError,
    };

    // Find the @fileconstant attribute
    let Some(attr) = attributes.get_kind(AttributeKind::FileConstant) else {
        return FileConstantParseResult::NotFileConstant;
    };

    // Must have exactly one argument (the file path)
    if attr.args.is_empty() {
        diagnostics.throw(FileConstantRequiresPathError {
            span: attr.span.clone(),
        });
        return FileConstantParseResult::Error;
    }

    let path_arg = &attr.args[0];

    // Argument must be unlabeled
    if path_arg.is_labeled() {
        diagnostics.throw(FileConstantInvalidArgumentError {
            span: path_arg.span.clone(),
        });
        return FileConstantParseResult::Error;
    }

    // Extract the path from the source using the value span
    let arg_text = &source[path_arg.value_span.range()];

    // Must be a string literal
    if !arg_text.starts_with('"') || !arg_text.ends_with('"') || arg_text.len() < 2 {
        diagnostics.throw(FileConstantRequiresStringError {
            span: path_arg.value_span.clone(),
        });
        return FileConstantParseResult::Error;
    }

    // Extract the path (remove quotes)
    let relative_path = arg_text[1..arg_text.len() - 1].to_string();

    FileConstantParseResult::Success {
        relative_path,
        span: attr.span.clone(),
    }
}

/// Result of parsing a `@platform(.darwin)` attribute.
pub enum PlatformParseResult {
    /// Successfully parsed: contains the target platform
    Success(TargetPlatform),
    /// Not a platform attribute
    NotPlatform,
    /// Error occurred during parsing (diagnostic already emitted)
    Error,
}

/// Parse a `@platform(.darwin)` or `@platform(.linux)` attribute from an AttributesBehavior.
///
/// This function checks if the attributes contain a `@platform` attribute,
/// validates its arguments, and returns the parsed `TargetPlatform`.
///
/// Note: By the time this runs, non-matching platform declarations have already
/// been filtered out by the semantic model builder. This validation catches
/// malformed `@platform` attributes on declarations that passed the filter.
pub fn parse_platform_attribute(
    attributes: &AttributesBehavior,
    source: &str,
    diagnostics: &mut DiagnosticContext,
) -> PlatformParseResult {
    let Some(attr) = attributes.get_kind(AttributeKind::Platform) else {
        return PlatformParseResult::NotPlatform;
    };

    if attr.args.is_empty() {
        diagnostics.throw(PlatformRequiresArgumentError {
            span: attr.span.clone(),
        });
        return PlatformParseResult::Error;
    }

    let arg = &attr.args[0];

    if arg.is_labeled() {
        diagnostics.throw(PlatformInvalidArgumentError {
            span: arg.span.clone(),
        });
        return PlatformParseResult::Error;
    }

    let arg_text = &source[arg.value_span.range()];

    if !arg_text.starts_with('.') {
        diagnostics.throw(PlatformInvalidArgumentError {
            span: arg.span.clone(),
        });
        return PlatformParseResult::Error;
    }

    let platform_name = &arg_text[1..];
    match TargetPlatform::from_name(platform_name) {
        Some(platform) => PlatformParseResult::Success(platform),
        None => {
            diagnostics.throw(PlatformUnknownPlatformError {
                span: arg.value_span.clone(),
                name: platform_name.to_string(),
            });
            PlatformParseResult::Error
        },
    }
}

/// Validate a @builtin attribute: check kind matches and no duplicate registration.
///
/// This is the shared logic for struct, enum, protocol, function, and type alias binders.
/// The `kind_check` predicate tests whether the feature's kind matches the declaration,
/// and `registry_lookup` retrieves any existing registration for duplicate detection.
pub fn validate_builtin_attribute(
    symbol: &std::sync::Arc<
        dyn semantic_tree::symbol::Symbol<kestrel_semantic_tree::language::KestrelLanguage>,
    >,
    attributes: &AttributesBehavior,
    source: &str,
    context: &mut crate::declaration_binder::BindingContext,
    actual_kind: &str,
    kind_check: impl Fn(&kestrel_semantic_tree::builtins::BuiltinKind) -> bool,
    registry_lookup: impl Fn(LanguageFeature) -> Option<semantic_tree::symbol::SymbolId>,
) {
    let feature = match parse_builtin_attribute(attributes, source, context.diagnostics) {
        BuiltinParseResult::Success(f) => f,
        BuiltinParseResult::NotBuiltin | BuiltinParseResult::Error => return,
    };

    let definition = feature.definition();
    let attr_span = attributes
        .get_kind(AttributeKind::Builtin)
        .map(|a| a.span.clone())
        .unwrap_or_else(|| symbol.metadata().span().clone());

    if !kind_check(&definition.kind) {
        context.diagnostics.throw(BuiltinWrongKindError {
            span: attr_span,
            feature_name: feature.name().to_string(),
            expected_kind: definition.kind.kind_name().to_string(),
            actual_kind: actual_kind.to_string(),
        });
        return;
    }

    let symbol_id = symbol.metadata().id();
    let existing = registry_lookup(feature);
    if existing.is_some() && existing != Some(symbol_id) {
        context.diagnostics.throw(DuplicateBuiltinError {
            span: attr_span,
            feature_name: feature.name().to_string(),
        });
    }
}

#[cfg(test)]
mod tests {

    // Tests would require constructing syntax nodes, which is complex.
    // The actual testing happens through the integration tests.
}
