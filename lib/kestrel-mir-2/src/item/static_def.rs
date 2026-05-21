use std::path::PathBuf;

use kestrel_hecs::Entity;

use crate::immediate::Immediate;
use crate::TyId;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_def_new() {
        let def = StaticDef::new(Entity::from_raw(1), "std.VERSION", TyId::new(0));
        assert_eq!(def.name, "std.VERSION");
        assert!(!def.is_mutable);
        assert!(def.initializer.is_none());
        assert!(def.file_constant_data.is_none());
    }

    #[test]
    fn static_def_with_initializer() {
        let mut def = StaticDef::new(Entity::from_raw(1), "MAX", TyId::new(0));
        def.initializer = Some(Immediate::i64(100));
        assert!(def.initializer.is_some());
    }
}
