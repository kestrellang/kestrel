//! Witness definitions — protocol implementation evidence.

use crate::item::TypeParamDef;
use crate::ty::MirTy;
use indexmap::IndexMap;
use kestrel_hecs::Entity;
use std::fmt;

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
    /// Method bindings: protocol method signature → implementation details.
    pub method_bindings: IndexMap<WitnessMethodKey, MethodBinding>,
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
    pub fn bind_method(&mut self, key: impl Into<WitnessMethodKey>, binding: MethodBinding) {
        self.method_bindings.insert(key.into(), binding);
    }
}

/// Stable key for a protocol method binding in a witness.
///
/// Protocols can expose overloads with the same name, so witness dispatch must
/// distinguish at least the externally visible call shape. The label vector
/// includes arity: `foo()` and `foo(bar:)` are different keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WitnessMethodKey {
    pub name: String,
    pub labels: Vec<Option<String>>,
}

impl WitnessMethodKey {
    pub fn new(name: impl Into<String>, labels: Vec<Option<String>>) -> Self {
        Self {
            name: name.into(),
            labels,
        }
    }

    pub fn bare(name: impl Into<String>) -> Self {
        Self::new(name, Vec::new())
    }
}

impl From<&str> for WitnessMethodKey {
    fn from(name: &str) -> Self {
        Self::bare(name)
    }
}

impl From<String> for WitnessMethodKey {
    fn from(name: String) -> Self {
        Self::bare(name)
    }
}

impl fmt::Display for WitnessMethodKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.name)?;
        for (i, label) in self.labels.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            match label {
                Some(label) => write!(f, "{label}:")?,
                None => write!(f, "_:")?,
            }
        }
        write!(f, ")")
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
