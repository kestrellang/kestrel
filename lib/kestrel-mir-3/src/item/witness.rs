use kestrel_hecs::Entity;

use crate::TyId;

use super::function::WhereConstraint;

#[derive(Debug, Clone, PartialEq)]
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

    pub fn simple(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            labels: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WitnessMethodBinding {
    pub key: WitnessMethodKey,
    pub func: Entity,
    pub type_args: Vec<TyId>,
}

impl WitnessMethodBinding {
    pub fn new(key: WitnessMethodKey, func: Entity, type_args: Vec<TyId>) -> Self {
        Self {
            key,
            func,
            type_args,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WitnessDef {
    pub protocol: Entity,
    pub implementing_type: TyId,
    pub constraints: Vec<WhereConstraint>,
    pub type_bindings: Vec<(Entity, TyId)>,
    pub methods: Vec<WitnessMethodBinding>,
    pub proto_type_args: Vec<TyId>,
}

impl WitnessDef {
    pub fn new(protocol: Entity, implementing_type: TyId) -> Self {
        Self {
            protocol,
            implementing_type,
            constraints: Vec::new(),
            type_bindings: Vec::new(),
            methods: Vec::new(),
            proto_type_args: Vec::new(),
        }
    }

    pub fn add_method(&mut self, binding: WitnessMethodBinding) {
        self.methods.push(binding);
    }

    pub fn add_type_binding(&mut self, assoc_entity: Entity, concrete_type: TyId) {
        self.type_bindings.push((assoc_entity, concrete_type));
    }
}
