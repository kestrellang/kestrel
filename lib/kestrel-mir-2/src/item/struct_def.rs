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
}

impl StructDef {
    pub fn new(entity: Entity, name: impl Into<String>) -> Self {
        Self {
            entity,
            name: name.into(),
            type_params: Vec::new(),
            fields: Vec::new(),
            type_info: TypeInfo::default(),
        }
    }

    pub fn add_field(&mut self, field: FieldDef) {
        self.fields.push(field);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struct_def_new() {
        let def = StructDef::new(Entity::from_raw(1), "std.Point");
        assert_eq!(def.name, "std.Point");
        assert!(def.fields.is_empty());
        assert!(def.type_info.layout.is_none());
    }

    #[test]
    fn struct_def_with_fields() {
        let mut def = StructDef::new(Entity::from_raw(1), "std.Point");
        def.add_field(FieldDef::new("x", TyId::new(0)));
        def.add_field(FieldDef::new("y", TyId::new(0)));
        assert_eq!(def.fields.len(), 2);
        assert_eq!(def.fields[0].name, "x");
        assert_eq!(def.fields[1].name, "y");
    }
}
