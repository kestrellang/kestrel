use std::path::PathBuf;

use kestrel_hecs::Entity;

use crate::TyId;
use crate::immediate::Immediate;

#[derive(Debug, Clone, PartialEq)]
pub struct FileConstantData {
    pub relative_path: String,
    pub element_ty: TyId,
    pub base_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StaticDef {
    pub entity: Entity,
    pub name: String,
    pub ty: TyId,
    pub is_mutable: bool,
    pub initializer: Option<Immediate>,
    pub init_order: u32,
    pub file_constant_data: Option<FileConstantData>,
}

impl StaticDef {
    pub fn new(entity: Entity, name: impl Into<String>, ty: TyId) -> Self {
        Self {
            entity,
            name: name.into(),
            ty,
            is_mutable: false,
            initializer: None,
            init_order: 0,
            file_constant_data: None,
        }
    }
}
