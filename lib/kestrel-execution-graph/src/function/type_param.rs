//! Type parameter definitions.

use crate::id::{Enum, Function, Id, Protocol, Struct};
use crate::metadata::{Metadata, Prior};

/// A type parameter definition.
#[derive(Debug, Clone)]
pub struct TypeParamDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<TypeParamDef>>,
    /// Name of this type parameter (e.g., "T").
    pub name: String,
    /// What item owns this type parameter.
    pub owner: TypeParamOwner,
}

impl TypeParamDef {
    pub fn new(name: impl Into<String>, owner: TypeParamOwner) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            name: name.into(),
            owner,
        }
    }
}

/// The item that owns a type parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeParamOwner {
    Function(Id<Function>),
    Struct(Id<Struct>),
    Enum(Id<Enum>),
    Protocol(Id<Protocol>),
}
