//! Static variable definitions.

use crate::immediate::Immediate;
use crate::ty::MirTy;
use kestrel_hecs::Entity;
use std::path::PathBuf;

/// A static variable (global constant or mutable static).
#[derive(Debug, Clone)]
pub struct StaticDef {
    /// The ECS entity for this static.
    pub entity: Entity,
    /// Fully qualified name.
    pub name: String,
    /// Type of this static.
    pub ty: MirTy,
    /// Whether this static is mutable (`var`).
    pub is_mutable: bool,
    /// Initial value (if known at compile time).
    pub initializer: Option<Immediate>,
    /// Topologically sorted initialization order.
    pub init_order: u32,
    /// Embedded file constant data (if this is a @fileconstant static).
    pub file_constant_data: Option<FileConstantData>,
}

impl StaticDef {
    pub fn new(entity: Entity, name: impl Into<String>, ty: MirTy) -> Self {
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

    pub fn mutable(mut self) -> Self {
        self.is_mutable = true;
        self
    }

    pub fn with_initializer(mut self, init: Immediate) -> Self {
        self.initializer = Some(init);
        self
    }

    pub fn with_file_constant(mut self, data: FileConstantData) -> Self {
        self.file_constant_data = Some(data);
        self
    }
}

/// Data for a file constant static (embedded binary data).
#[derive(Debug, Clone)]
pub struct FileConstantData {
    /// Relative path to the file (as specified in @fileconstant).
    pub relative_path: String,
    /// Element type for the LiteralSlice.
    pub element_ty: MirTy,
    /// Base directory to resolve the relative path against.
    pub base_path: Option<PathBuf>,
}
