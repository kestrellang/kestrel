use kestrel_hecs::Entity;

use crate::TyId;

use super::{TypeInfo, TypeParamDef};

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub name: String,
    pub ty: TyId,
}

impl FieldDef {
    pub fn new(name: impl Into<String>, ty: TyId) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub entity: Entity,
    pub name: String,
    pub type_params: Vec<TypeParamDef>,
    pub fields: Vec<FieldDef>,
    pub type_info: TypeInfo,
    /// Type-param positions that gate this type's *conditional* Copyable
    /// conformance (`struct X: not Copyable` + `extend X: Copyable where
    /// T: Copyable`). Empty unless conditionally copyable. Drives
    /// per-instantiation `copy_behavior`: `X[args]` is Copyable iff every gating
    /// `args[i]` is. See `kestrel_semantics::ConditionalCopyableParams`.
    pub conditionally_copyable: Vec<usize>,
}

impl StructDef {
    pub fn new(entity: Entity, name: impl Into<String>) -> Self {
        Self {
            entity,
            name: name.into(),
            type_params: Vec::new(),
            fields: Vec::new(),
            type_info: TypeInfo::default(),
            conditionally_copyable: Vec::new(),
        }
    }

    pub fn add_field(&mut self, field: FieldDef) {
        self.fields.push(field);
    }
}
