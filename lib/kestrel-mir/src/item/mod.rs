//! Top-level item definitions in MIR.

mod behavior;
mod closure;
mod enum_def;
mod function;
mod protocol;
mod static_def;
mod struct_def;
mod witness;

pub use behavior::*;
pub use closure::*;
pub use enum_def::*;
pub use function::*;
pub use protocol::*;
pub use static_def::*;
pub use struct_def::*;
pub use witness::*;

use kestrel_hecs::Entity;

/// A type parameter definition.
#[derive(Debug, Clone)]
pub struct TypeParamDef {
    /// The ECS entity for this type parameter.
    pub entity: Entity,
    /// Name of this type parameter (e.g., "T").
    pub name: String,
}

impl TypeParamDef {
    pub fn new(entity: Entity, name: impl Into<String>) -> Self {
        Self {
            entity,
            name: name.into(),
        }
    }
}
