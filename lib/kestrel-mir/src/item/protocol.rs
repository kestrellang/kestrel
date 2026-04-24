//! Protocol definitions.

use crate::item::TypeParamDef;
use crate::ty::MirTy;
use indexmap::IndexMap;
use kestrel_hecs::Entity;

/// A protocol definition.
#[derive(Debug, Clone)]
pub struct ProtocolDef {
    /// The ECS entity for this protocol.
    pub entity: Entity,
    /// Fully qualified name.
    pub name: String,
    /// Generic type parameters.
    pub type_params: Vec<TypeParamDef>,
    /// Parent protocols (for protocol inheritance).
    pub parent_protocols: Vec<Entity>,
    /// Associated types in declaration order.
    pub associated_types: Vec<AssociatedTypeDef>,
    /// Associated type lookup by name.
    pub associated_types_by_name: IndexMap<String, usize>,
    /// Methods in declaration order.
    pub methods: Vec<ProtocolMethodDef>,
    /// Method lookup by name.
    pub methods_by_name: IndexMap<String, usize>,
}

impl ProtocolDef {
    pub fn new(entity: Entity, name: impl Into<String>) -> Self {
        Self {
            entity,
            name: name.into(),
            type_params: Vec::new(),
            parent_protocols: Vec::new(),
            associated_types: Vec::new(),
            associated_types_by_name: IndexMap::new(),
            methods: Vec::new(),
            methods_by_name: IndexMap::new(),
        }
    }

    /// Add a parent protocol.
    pub fn add_parent(&mut self, parent: Entity) {
        self.parent_protocols.push(parent);
    }

    /// Add an associated type.
    pub fn add_associated_type(&mut self, assoc: AssociatedTypeDef) {
        let idx = self.associated_types.len();
        self.associated_types_by_name
            .insert(assoc.name.clone(), idx);
        self.associated_types.push(assoc);
    }

    /// Add a method.
    pub fn add_method(&mut self, method: ProtocolMethodDef) {
        let idx = self.methods.len();
        self.methods_by_name.insert(method.name.clone(), idx);
        self.methods.push(method);
    }

    /// Look up an associated type by name.
    pub fn associated_type_by_name(&self, name: &str) -> Option<&AssociatedTypeDef> {
        self.associated_types_by_name
            .get(name)
            .map(|&idx| &self.associated_types[idx])
    }

    /// Look up a method by name.
    pub fn method_by_name(&self, name: &str) -> Option<&ProtocolMethodDef> {
        self.methods_by_name
            .get(name)
            .map(|&idx| &self.methods[idx])
    }
}

/// An associated type in a protocol.
#[derive(Debug, Clone)]
pub struct AssociatedTypeDef {
    /// Name (e.g., "Item").
    pub name: String,
    /// Optional default type.
    pub default: Option<MirTy>,
}

impl AssociatedTypeDef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            default: None,
        }
    }

    pub fn with_default(mut self, default: MirTy) -> Self {
        self.default = Some(default);
        self
    }
}

/// A method signature in a protocol.
#[derive(Debug, Clone)]
pub struct ProtocolMethodDef {
    /// Method name.
    pub name: String,
    /// Type parameters for this method.
    pub type_params: Vec<TypeParamDef>,
    /// Parameters as (name, type) pairs.
    pub params: Vec<(String, MirTy)>,
    /// Return type.
    pub ret: MirTy,
    /// Whether this method has a default implementation.
    pub has_default: bool,
}

impl ProtocolMethodDef {
    pub fn new(name: impl Into<String>, ret: MirTy) -> Self {
        Self {
            name: name.into(),
            type_params: Vec::new(),
            params: Vec::new(),
            ret,
            has_default: false,
        }
    }

    pub fn add_param(&mut self, name: impl Into<String>, ty: MirTy) {
        self.params.push((name.into(), ty));
    }

    pub fn with_default(mut self) -> Self {
        self.has_default = true;
        self
    }
}
