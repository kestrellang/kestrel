use kestrel_hecs::Entity;

use super::struct_def::FieldDef;
use super::{TypeInfo, TypeParamDef};

#[derive(Debug, Clone, PartialEq)]
pub struct EnumCaseDef {
    pub name: String,
    pub discriminant: u32,
    pub payload_fields: Vec<FieldDef>,
}

impl EnumCaseDef {
    pub fn new(name: impl Into<String>, discriminant: u32) -> Self {
        Self {
            name: name.into(),
            discriminant,
            payload_fields: Vec::new(),
        }
    }

    pub fn with_payload(
        name: impl Into<String>,
        discriminant: u32,
        payload_fields: Vec<FieldDef>,
    ) -> Self {
        Self {
            name: name.into(),
            discriminant,
            payload_fields,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub entity: Entity,
    pub name: String,
    pub type_params: Vec<TypeParamDef>,
    pub cases: Vec<EnumCaseDef>,
    pub type_info: TypeInfo,
}

impl EnumDef {
    pub fn new(entity: Entity, name: impl Into<String>) -> Self {
        Self {
            entity,
            name: name.into(),
            type_params: Vec::new(),
            cases: Vec::new(),
            type_info: TypeInfo::default(),
        }
    }

    pub fn add_case(&mut self, case: EnumCaseDef) {
        self.cases.push(case);
    }
}
