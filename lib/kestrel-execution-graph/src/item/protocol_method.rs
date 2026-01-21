//! Protocol method definitions.

use crate::id::{Id, Ty, TypeParam};
use crate::metadata::{Metadata, Prior};

/// A method signature in a protocol.
#[derive(Debug, Clone)]
pub struct ProtocolMethodDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<ProtocolMethodDef>>,
    /// Method name.
    pub name: String,
    /// Type parameters for this method (e.g., `H` in `func hash[H](...)`).
    pub type_params: Vec<Id<TypeParam>>,
    /// Parameters as (name, type) pairs.
    pub params: Vec<(String, Id<Ty>)>,
    /// Return type.
    pub ret: Id<Ty>,
    /// Whether this method has a default implementation.
    pub has_default: bool,
}

impl ProtocolMethodDef {
    pub fn new(name: impl Into<String>, ret: Id<Ty>) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            name: name.into(),
            type_params: Vec::new(),
            params: Vec::new(),
            ret,
            has_default: false,
        }
    }

    pub fn add_param(&mut self, name: impl Into<String>, ty: Id<Ty>) {
        self.params.push((name.into(), ty));
    }

    pub fn with_default(mut self) -> Self {
        self.has_default = true;
        self
    }
}
