//! Associated type definitions for protocols.

use crate::id::{Id, Ty};
use crate::metadata::{Metadata, Prior};

/// An associated type in a protocol.
#[derive(Debug, Clone)]
pub struct AssociatedTypeDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<AssociatedTypeDef>>,
    /// Name of this associated type (e.g., "Item").
    pub name: String,
    /// Optional default type.
    pub default: Option<Id<Ty>>,
}

impl AssociatedTypeDef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            name: name.into(),
            default: None,
        }
    }

    pub fn with_default(mut self, default: Id<Ty>) -> Self {
        self.default = Some(default);
        self
    }
}
