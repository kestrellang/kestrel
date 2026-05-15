//! Struct definitions.

use crate::id::FieldId;
use crate::item::{CopyBehavior, DeinitBehavior, TypeParamDef};
use crate::ty::MirTy;
use indexmap::IndexMap;
use kestrel_hecs::Entity;

/// A struct definition.
#[derive(Debug, Clone)]
pub struct StructDef {
    /// The ECS entity for this struct.
    pub entity: Entity,
    /// Fully qualified name.
    pub name: String,
    /// Generic type parameters.
    pub type_params: Vec<TypeParamDef>,
    /// Fields in declaration order.
    pub fields: Vec<FieldDef>,
    /// Field lookup by name.
    pub fields_by_name: IndexMap<String, FieldId>,
    /// Precomputed layout (filled by the layout pass).
    pub layout: Option<StructLayout>,
    /// How this struct is duplicated. Populated by `kestrel-mir-lower` from
    /// `kestrel_semantics::NominalCopySemantics`.
    pub copy_behavior: CopyBehavior,
    /// How this struct is destroyed. Populated by `kestrel-mir-lower` from
    /// the `deinit` method (if any) plus structural field drops.
    pub deinit_behavior: DeinitBehavior,
}

impl StructDef {
    pub fn new(entity: Entity, name: impl Into<String>) -> Self {
        Self {
            entity,
            name: name.into(),
            type_params: Vec::new(),
            fields: Vec::new(),
            fields_by_name: IndexMap::new(),
            layout: None,
            // Default to `None` (affine) until lowering populates the real
            // behavior. Primitives and types built directly by the MIR test
            // helpers can override after `new`.
            copy_behavior: CopyBehavior::None,
            deinit_behavior: DeinitBehavior::default(),
        }
    }

    /// Add a field and return its ID.
    pub fn add_field(&mut self, field: FieldDef) -> FieldId {
        let id = FieldId::new(self.fields.len());
        self.fields_by_name.insert(field.name.clone(), id);
        self.fields.push(field);
        id
    }

    /// Look up a field by name.
    pub fn field_by_name(&self, name: &str) -> Option<FieldId> {
        self.fields_by_name.get(name).copied()
    }
}

/// A field in a struct or enum case.
#[derive(Debug, Clone)]
pub struct FieldDef {
    /// Field name (or numeric index for tuple-like fields).
    pub name: String,
    /// The type of this field.
    pub ty: MirTy,
}

impl FieldDef {
    pub fn new(name: impl Into<String>, ty: MirTy) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

/// Precomputed struct memory layout.
#[derive(Debug, Clone)]
pub struct StructLayout {
    /// Total size in bytes.
    pub size: u64,
    /// Alignment in bytes.
    pub align: u64,
    /// Byte offset of each field, in field declaration order.
    pub field_offsets: Vec<u64>,
}
