//! Witness definitions — protocol implementation evidence.

use crate::item::TypeParamDef;
use crate::ty::MirTy;
use indexmap::IndexMap;
use kestrel_hecs::Entity;

/// A witness proves that a type implements a protocol.
#[derive(Debug, Clone)]
pub struct WitnessDef {
    /// The type that implements the protocol.
    pub implementing_type: MirTy,
    /// The protocol being implemented.
    pub protocol: Entity,
    /// Protocol type argument bindings (e.g., `And[Rhs = Bool]` → { "Rhs": Bool }).
    pub protocol_type_args: IndexMap<String, MirTy>,
    /// Type parameters for this witness (for generic implementations).
    pub type_params: Vec<TypeParamDef>,
    /// Associated type bindings: name → concrete type.
    pub type_bindings: IndexMap<String, MirTy>,
    /// Method bindings: method name → implementation details.
    pub method_bindings: IndexMap<String, MethodBinding>,
}

impl WitnessDef {
    pub fn new(implementing_type: MirTy, protocol: Entity) -> Self {
        Self {
            implementing_type,
            protocol,
            protocol_type_args: IndexMap::new(),
            type_params: Vec::new(),
            type_bindings: IndexMap::new(),
            method_bindings: IndexMap::new(),
        }
    }

    /// Bind an associated type to a concrete type.
    pub fn bind_type(&mut self, name: impl Into<String>, ty: MirTy) {
        self.type_bindings.insert(name.into(), ty);
    }

    /// Bind a method to its implementation.
    pub fn bind_method(&mut self, name: impl Into<String>, binding: MethodBinding) {
        self.method_bindings.insert(name.into(), binding);
    }
}

/// A method implementation binding in a witness.
#[derive(Debug, Clone)]
pub struct MethodBinding {
    /// The function entity that implements this method.
    pub implementation: Entity,
    /// Type arguments for the implementation function.
    pub type_args: Vec<MirTy>,
    /// Where the implementation comes from.
    pub source: MethodSource,
}

impl MethodBinding {
    pub fn direct(implementation: Entity, type_args: Vec<MirTy>) -> Self {
        Self {
            implementation,
            type_args,
            source: MethodSource::Direct,
        }
    }

    pub fn extension(implementation: Entity, type_args: Vec<MirTy>, protocol: Entity) -> Self {
        Self {
            implementation,
            type_args,
            source: MethodSource::Extension { protocol },
        }
    }
}

/// Where a method implementation comes from.
#[derive(Debug, Clone)]
pub enum MethodSource {
    /// Defined directly on the implementing type.
    Direct,
    /// Default implementation from a protocol extension.
    Extension { protocol: Entity },
}
