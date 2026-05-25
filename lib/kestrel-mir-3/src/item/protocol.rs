use kestrel_hecs::Entity;

use crate::TyId;

use super::TypeParamDef;

#[derive(Debug, Clone, PartialEq)]
pub struct AssociatedTypeDef {
    pub entity: Entity,
    pub name: String,
    pub default: Option<TyId>,
}

impl AssociatedTypeDef {
    pub fn new(entity: Entity, name: impl Into<String>) -> Self {
        Self { entity, name: name.into(), default: None }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolMethodDef {
    pub name: String,
    pub type_params: Vec<TypeParamDef>,
    pub params: Vec<(String, TyId)>,
    pub ret: TyId,
    pub has_default: bool,
}

impl ProtocolMethodDef {
    pub fn new(name: impl Into<String>, params: Vec<(String, TyId)>, ret: TyId) -> Self {
        Self {
            name: name.into(),
            type_params: Vec::new(),
            params,
            ret,
            has_default: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolDef {
    pub entity: Entity,
    pub name: String,
    pub type_params: Vec<TypeParamDef>,
    pub parent_protocols: Vec<Entity>,
    pub associated_types: Vec<AssociatedTypeDef>,
    pub methods: Vec<ProtocolMethodDef>,
}

impl ProtocolDef {
    pub fn new(entity: Entity, name: impl Into<String>) -> Self {
        Self {
            entity,
            name: name.into(),
            type_params: Vec::new(),
            parent_protocols: Vec::new(),
            associated_types: Vec::new(),
            methods: Vec::new(),
        }
    }
}
