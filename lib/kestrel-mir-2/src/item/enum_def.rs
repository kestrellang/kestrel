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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TyId;

    #[test]
    fn enum_def_new() {
        let def = EnumDef::new(Entity::from_raw(1), "Optional");
        assert_eq!(def.name, "Optional");
        assert!(def.cases.is_empty());
    }

    #[test]
    fn enum_with_cases() {
        let mut def = EnumDef::new(Entity::from_raw(1), "Optional");
        def.add_case(EnumCaseDef::new("None", 0));
        def.add_case(EnumCaseDef::with_payload(
            "Some",
            1,
            vec![FieldDef::new("0", TyId::new(0))],
        ));
        assert_eq!(def.cases.len(), 2);
        assert_eq!(def.cases[0].name, "None");
        assert!(def.cases[0].payload_fields.is_empty());
        assert_eq!(def.cases[1].name, "Some");
        assert_eq!(def.cases[1].payload_fields.len(), 1);
    }

    #[test]
    fn enum_case_discriminant() {
        let case = EnumCaseDef::new("Ok", 0);
        assert_eq!(case.discriminant, 0);
    }
}
