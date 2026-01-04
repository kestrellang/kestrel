//! AttributesBehavior for storing resolved attributes on symbols.

use semantic_tree::behavior::Behavior;

use crate::{
    attributes::{Attribute, AttributeKind},
    behavior::KestrelBehaviorKind,
    language::KestrelLanguage,
};

/// Behavior that stores resolved attributes on a symbol.
///
/// This is used for declarations that can have attributes:
/// - Protocols, structs, enums
/// - Functions, methods, initializers
/// - Fields, enum cases
///
/// The attributes are resolved during the binding phase and stored here
/// for later access during semantic analysis and code generation.
#[derive(Debug, Clone)]
pub struct AttributesBehavior {
    /// The resolved attributes on this symbol.
    attributes: Vec<Attribute>,
}

impl Behavior<KestrelLanguage> for AttributesBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::Attributes
    }
}

impl AttributesBehavior {
    /// Create a new AttributesBehavior with the given attributes.
    pub fn new(attributes: Vec<Attribute>) -> Self {
        Self { attributes }
    }

    /// Create an empty AttributesBehavior.
    pub fn empty() -> Self {
        Self {
            attributes: Vec::new(),
        }
    }

    /// Get all resolved attributes.
    pub fn attributes(&self) -> &[Attribute] {
        &self.attributes
    }

    /// Check if this symbol has any attributes.
    pub fn has_attributes(&self) -> bool {
        !self.attributes.is_empty()
    }

    /// Check if this symbol has a specific attribute by name.
    pub fn has(&self, name: &str) -> bool {
        self.attributes.iter().any(|a| a.name == name)
    }

    /// Check if this symbol has a specific attribute kind.
    pub fn has_kind(&self, kind: AttributeKind) -> bool {
        self.attributes.iter().any(|a| a.kind == kind)
    }

    /// Get the first attribute with the given name.
    pub fn get(&self, name: &str) -> Option<&Attribute> {
        self.attributes.iter().find(|a| a.name == name)
    }

    /// Get the first attribute of the given kind.
    pub fn get_kind(&self, kind: AttributeKind) -> Option<&Attribute> {
        self.attributes.iter().find(|a| a.kind == kind)
    }

    /// Get all attributes with the given name (for repeatable attributes).
    pub fn get_all(&self, name: &str) -> Vec<&Attribute> {
        self.attributes.iter().filter(|a| a.name == name).collect()
    }

    /// Get all unknown attributes (for emitting warnings).
    pub fn unknown_attributes(&self) -> Vec<&Attribute> {
        self.attributes
            .iter()
            .filter(|a| a.kind == AttributeKind::Unknown)
            .collect()
    }
}
