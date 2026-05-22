use kestrel_hecs::Entity;

use crate::statement::WitnessMethodKey;
use crate::TyId;

use super::function::WhereConstraint;

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
    /// Protocol type args: maps protocol type params (by position) to their
    /// expressions in the witness context. For `extend Int64: SeqIndex[T]`,
    /// this would be `[TypeParam(T_ext)]`. Used by the monomorphizer to
    /// connect call-site type args to extension free params.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn witness_def_new() {
        let proto = Entity::from_raw(1);
        let impl_ty = TyId::new(0);
        let def = WitnessDef::new(proto, impl_ty);
        assert_eq!(def.protocol, proto);
        assert_eq!(def.implementing_type, impl_ty);
        assert!(def.methods.is_empty());
        assert!(def.type_bindings.is_empty());
    }

    #[test]
    fn witness_with_method() {
        let mut def = WitnessDef::new(Entity::from_raw(1), TyId::new(0));
        def.add_method(WitnessMethodBinding::new(
            WitnessMethodKey::simple("equals"),
            Entity::from_raw(10),
            vec![],
        ));
        assert_eq!(def.methods.len(), 1);
        assert_eq!(def.methods[0].key.name, "equals");
    }

    #[test]
    fn witness_with_type_binding() {
        let mut def = WitnessDef::new(Entity::from_raw(1), TyId::new(0));
        def.add_type_binding(Entity::from_raw(5), TyId::new(2));
        assert_eq!(def.type_bindings.len(), 1);
        assert_eq!(def.type_bindings[0].0, Entity::from_raw(5));
        assert_eq!(def.type_bindings[0].1, TyId::new(2));
    }
}
