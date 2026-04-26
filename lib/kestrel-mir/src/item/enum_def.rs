//! Enum definitions.

use crate::id::StructId;
use crate::item::TypeParamDef;
use indexmap::IndexMap;
use kestrel_hecs::Entity;

/// An enum definition.
#[derive(Debug, Clone)]
pub struct EnumDef {
    /// The ECS entity for this enum.
    pub entity: Entity,
    /// Fully qualified name.
    pub name: String,
    /// Generic type parameters.
    pub type_params: Vec<TypeParamDef>,
    /// Cases in declaration order.
    pub cases: Vec<EnumCaseDef>,
    /// Case lookup by name.
    pub cases_by_name: IndexMap<String, usize>,
}

impl EnumDef {
    pub fn new(entity: Entity, name: impl Into<String>) -> Self {
        Self {
            entity,
            name: name.into(),
            type_params: Vec::new(),
            cases: Vec::new(),
            cases_by_name: IndexMap::new(),
        }
    }

    /// Add a case and return its index.
    pub fn add_case(&mut self, case: EnumCaseDef) -> usize {
        let idx = self.cases.len();
        self.cases_by_name.insert(case.name.clone(), idx);
        self.cases.push(case);
        idx
    }

    /// Look up a case by name.
    pub fn case_by_name(&self, name: &str) -> Option<&EnumCaseDef> {
        self.cases_by_name.get(name).map(|&idx| &self.cases[idx])
    }
}

/// An enum case definition.
///
/// Each case maps to a struct holding the payload fields.
#[derive(Debug, Clone)]
pub struct EnumCaseDef {
    /// Case name (e.g., "Some", "None").
    pub name: String,
    /// The discriminant value for this case.
    pub discriminant: u32,
    /// The struct that holds this case's payload fields.
    pub payload_struct: StructId,
}

impl EnumCaseDef {
    pub fn new(name: impl Into<String>, discriminant: u32, payload_struct: StructId) -> Self {
        Self {
            name: name.into(),
            discriminant,
            payload_struct,
        }
    }
}
