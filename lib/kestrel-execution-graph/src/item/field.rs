//! Field definitions for structs and enum cases.

use crate::id::{Id, Ty};
use crate::metadata::{Metadata, Prior};

/// A field in a struct or enum case.
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<FieldDef>>,
    /// Field name (or numeric index for tuple-like fields like "0", "1").
    pub name: String,
    /// The type of this field.
    pub ty: Id<Ty>,
}

impl FieldDef {
    pub fn new(name: impl Into<String>, ty: Id<Ty>) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            name: name.into(),
            ty,
        }
    }
}
