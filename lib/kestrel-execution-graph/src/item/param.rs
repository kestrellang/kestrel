//! Function parameter definitions.

use crate::id::{Id, Local, Ty};
use crate::metadata::{Metadata, Prior};

/// A function parameter.
#[derive(Debug, Clone)]
pub struct ParamDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<ParamDef>>,
    /// Parameter name.
    pub name: String,
    /// The local variable this parameter is bound to.
    pub local: Id<Local>,
    /// Parameter type.
    pub ty: Id<Ty>,
    /// External label for this parameter (used in mangling).
    /// `None` means unlabeled (bare type in signature), `Some(label)` means labeled.
    pub external_label: Option<String>,
}

impl ParamDef {
    pub fn new(name: impl Into<String>, local: Id<Local>, ty: Id<Ty>) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            name: name.into(),
            local,
            ty,
            external_label: None,
        }
    }

    pub fn with_label(
        name: impl Into<String>,
        local: Id<Local>,
        ty: Id<Ty>,
        external_label: Option<String>,
    ) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            name: name.into(),
            local,
            ty,
            external_label,
        }
    }
}
