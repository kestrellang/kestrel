//! Attribute types for the Kestrel semantic model.
//!
//! Attributes are metadata annotations on declarations, using the `@name(args)` syntax.
//! This module defines the resolved semantic representation of attributes.

use kestrel_span::Span;

/// Known attribute types that the compiler understands.
///
/// New attributes should be added to this enum when they have specific
/// semantic meaning that the compiler needs to recognize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttributeKind {
    /// `@dummy` - A placeholder attribute for testing the infrastructure.
    /// This attribute is recognized but has no semantic effect.
    Dummy,

    /// Unknown attribute - parsed but not recognized by the compiler.
    /// A warning is emitted for unknown attributes.
    Unknown,
}

impl AttributeKind {
    /// Get the attribute kind from a name string.
    pub fn from_name(name: &str) -> Self {
        match name {
            "dummy" => AttributeKind::Dummy,
            _ => AttributeKind::Unknown,
        }
    }

    /// Check if this attribute kind is known (not Unknown).
    pub fn is_known(&self) -> bool {
        !matches!(self, AttributeKind::Unknown)
    }
}

/// A resolved attribute on a declaration.
///
/// This represents an attribute after semantic analysis, with the attribute
/// kind determined and arguments stored.
#[derive(Debug, Clone)]
pub struct Attribute {
    /// The kind of attribute (known or unknown).
    pub kind: AttributeKind,

    /// The original name as written in source code.
    pub name: String,

    /// The resolved arguments.
    pub args: Vec<AttributeArg>,

    /// Source span of the entire attribute (from @ to closing paren or end of name).
    pub span: Span,
}

impl Attribute {
    /// Create a new attribute.
    pub fn new(name: String, args: Vec<AttributeArg>, span: Span) -> Self {
        let kind = AttributeKind::from_name(&name);
        Self {
            kind,
            name,
            args,
            span,
        }
    }

    /// Check if this attribute has the given name.
    pub fn has_name(&self, name: &str) -> bool {
        self.name == name
    }

    /// Check if this attribute is of a specific kind.
    pub fn is_kind(&self, kind: AttributeKind) -> bool {
        self.kind == kind
    }

    /// Check if this is an unknown attribute.
    pub fn is_unknown(&self) -> bool {
        self.kind == AttributeKind::Unknown
    }
}

/// A single argument in an attribute.
///
/// Arguments can be labeled (`key: value`) or unlabeled (`value`).
/// For now, we store the value as a span - the actual value interpretation
/// is deferred to specific attribute handlers in later phases.
#[derive(Debug, Clone)]
pub struct AttributeArg {
    /// Optional label for this argument (e.g., `iOS` in `iOS: 15.0`).
    pub label: Option<String>,

    /// Span of the value expression.
    /// The actual value interpretation depends on the specific attribute.
    pub value_span: Span,

    /// Full span of this argument (including label if present).
    pub span: Span,
}

impl AttributeArg {
    /// Create a new unlabeled argument.
    pub fn unlabeled(value_span: Span) -> Self {
        Self {
            label: None,
            value_span: value_span.clone(),
            span: value_span,
        }
    }

    /// Create a new labeled argument.
    pub fn labeled(label: String, value_span: Span, span: Span) -> Self {
        Self {
            label: Some(label),
            value_span,
            span,
        }
    }

    /// Check if this argument has a label.
    pub fn is_labeled(&self) -> bool {
        self.label.is_some()
    }

    /// Get the label if present.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
}
