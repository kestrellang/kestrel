//! Local variable definitions.

use crate::id::{Id, Ty};
use crate::metadata::{Metadata, Prior};

/// A local variable in a function.
#[derive(Debug, Clone)]
pub struct LocalDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<LocalDef>>,
    /// Variable name (without the `%` prefix).
    pub name: String,
    /// Type of this local.
    pub ty: Id<Ty>,
}

impl LocalDef {
    pub fn new(name: impl Into<String>, ty: Id<Ty>) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            name: name.into(),
            ty,
        }
    }
}
