use kestrel_hecs::Entity;

use super::struct_def::FieldDef;
use super::{TypeInfo, TypeParamDef};
use crate::TyId;

#[derive(Debug, Clone, PartialEq)]
pub struct EnumCaseDef {
    pub name: String,
    pub discriminant: u32,
    /// PHYSICAL payload — what layout, drop/clone shims, mono and codegen
    /// see. For a boxed case this is the single `OwnedBox` field.
    pub payload_fields: Vec<FieldDef>,
    /// Set for a boxed (recursive) case of an `indirect enum`. Only body
    /// lowering of construction and patterns reads it, translating the
    /// declared (logical) payload to and from the box.
    pub indirect: Option<IndirectPayload>,
}

/// The logical shape of a boxed `indirect enum` case: its declared fields,
/// stored as ONE heap block `OwnedBox[payload_ty]`, one allocation per value.
#[derive(Debug, Clone, PartialEq)]
pub struct IndirectPayload {
    /// The declared fields (names + types), in declaration order.
    pub logical_fields: Vec<FieldDef>,
    /// What the box holds: the lone field's type, or the tuple of all of
    /// them (`(f0, f1, …)`).
    pub payload_ty: TyId,
    /// `OwnedBox[payload_ty]` — the type of the one physical field.
    pub box_ty: TyId,
}

impl IndirectPayload {
    /// The payload is a tuple (two or more fields), not the lone field.
    pub fn is_tuple(&self) -> bool {
        self.logical_fields.len() != 1
    }
}

impl EnumCaseDef {
    pub fn new(name: impl Into<String>, discriminant: u32) -> Self {
        Self {
            name: name.into(),
            discriminant,
            payload_fields: Vec::new(),
            indirect: None,
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
            indirect: None,
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
    /// Type-param positions that gate this type's *conditional* Copyable
    /// conformance (`enum X: not Copyable` + `extend X: Copyable where
    /// T: Copyable`). Empty unless conditionally copyable. See
    /// `kestrel_semantics::ConditionalCopyableParams` and `StructDef`.
    pub conditionally_copyable: kestrel_copy_fold::ConditionalCopy,
    /// `@layout` / `@align`, the input to every layout of this type.
    pub layout_attrs: crate::layout::LayoutAttrs,
}

impl EnumDef {
    pub fn new(entity: Entity, name: impl Into<String>) -> Self {
        Self {
            entity,
            name: name.into(),
            type_params: Vec::new(),
            cases: Vec::new(),
            type_info: TypeInfo::default(),
            conditionally_copyable: kestrel_copy_fold::ConditionalCopy::default(),
            layout_attrs: crate::layout::LayoutAttrs::default(),
        }
    }

    /// The declared payload of `case`: the logical fields of a boxed case,
    /// else its (physical = logical) payload fields.
    pub fn logical_fields(&self, case: usize) -> &[FieldDef] {
        self.cases.get(case).map_or(&[], |c| {
            c.indirect
                .as_ref()
                .map_or(c.payload_fields.as_slice(), |ind| &ind.logical_fields)
        })
    }

    pub fn add_case(&mut self, case: EnumCaseDef) {
        self.cases.push(case);
    }
}
