pub mod enum_def;
pub mod function;
pub mod protocol;
pub mod static_def;
pub mod struct_def;
pub mod witness;

use kestrel_hecs::Entity;

use crate::layout::{EnumLayout, StructLayout};
use crate::{FieldIdx, VariantIdx};

pub use enum_def::{EnumCaseDef, EnumDef};
pub use function::{
    CallingConvention, ExternInfo, FunctionDef, FunctionKind, ParamDef, WhereClause,
    WhereConstraint,
};
pub use protocol::{AssociatedTypeDef, ProtocolDef, ProtocolMethodDef};
pub use static_def::{FileConstantData, StaticDef};
pub use struct_def::{FieldDef, StructDef};
pub use witness::{WitnessDef, WitnessMethodBinding, WitnessMethodKey};

#[derive(Debug, Clone, PartialEq)]
pub struct TypeParamDef {
    pub entity: Entity,
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

#[derive(Debug, Clone, PartialEq)]
pub enum CopyBehavior {
    Bitwise,
    Clone(Entity),
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DropBehavior {
    None,
    StructDrop {
        deinit: Option<Entity>,
        fields: Vec<FieldIdx>,
    },
    EnumDrop {
        deinit: Option<Entity>,
        variants: Vec<(VariantIdx, Vec<FieldIdx>)>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeInfo {
    pub copy: CopyBehavior,
    pub drop: DropBehavior,
    pub layout: Option<Layout>,
}

impl TypeInfo {
    pub fn none() -> Self {
        Self {
            copy: CopyBehavior::Bitwise,
            drop: DropBehavior::None,
            layout: None,
        }
    }

    pub fn bitwise() -> Self {
        Self {
            copy: CopyBehavior::Bitwise,
            drop: DropBehavior::None,
            layout: None,
        }
    }
}

impl Default for TypeInfo {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Layout {
    Struct(StructLayout),
    Enum(EnumLayout),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TargetConfig {
    pub pointer_width: u64,
}

impl TargetConfig {
    pub fn host_64() -> Self {
        Self { pointer_width: 8 }
    }
}
