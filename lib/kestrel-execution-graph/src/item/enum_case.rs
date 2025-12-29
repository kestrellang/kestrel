//! Enum case definitions in MIR.

use crate::id::{Id, QualifiedName, Struct};
use crate::metadata::{Metadata, Prior};

/// An enum case definition.
///
/// Each case maps to a struct in the `"cases"` namespace containing the payload fields.
#[derive(Debug, Clone)]
pub struct EnumCaseDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<EnumCaseDef>>,
    /// Case name (e.g., "Some", "None").
    pub name: String,
    /// The discriminant value for this case.
    pub discriminant: u32,
    /// The associated struct that holds this case's payload.
    /// Points to a struct like `Module.Path.EnumName."cases".CaseName`.
    pub struct_name: Id<QualifiedName>,
    /// Optional reference to the actual struct definition.
    pub struct_def: Option<Id<Struct>>,
}

impl EnumCaseDef {
    pub fn new(name: impl Into<String>, discriminant: u32, struct_name: Id<QualifiedName>) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            name: name.into(),
            discriminant,
            struct_name,
            struct_def: None,
        }
    }
}
