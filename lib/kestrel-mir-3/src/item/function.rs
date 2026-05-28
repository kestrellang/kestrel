use kestrel_hecs::Entity;

use crate::body::OssaBody;
use crate::ty::ParamConvention;
use crate::{TyId, ValueId};

use super::TypeParamDef;

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionKind {
    Free,
    Method {
        parent: Entity,
        receiver: ParamConvention,
    },
    StaticMethod {
        parent: Entity,
    },
    Initializer {
        parent: Entity,
    },
    Deinit {
        parent: Entity,
    },
    ClosureCall {
        env_struct: Entity,
    },
    Closure {
        parent_func: Entity,
    },
    Thunk {
        original: Entity,
    },
    DropShim {
        nominal: Entity,
    },
    CloneShim {
        nominal: Entity,
    },
    ModuleInit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamDef {
    pub name: String,
    pub value: ValueId,
    pub ty: TyId,
    pub convention: ParamConvention,
    pub external_label: Option<String>,
}

impl ParamDef {
    pub fn new(
        name: impl Into<String>,
        value: ValueId,
        ty: TyId,
        convention: ParamConvention,
    ) -> Self {
        Self {
            name: name.into(),
            value,
            ty,
            convention,
            external_label: None,
        }
    }

    pub fn with_label(
        name: impl Into<String>,
        value: ValueId,
        ty: TyId,
        convention: ParamConvention,
        label: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            value,
            ty,
            convention,
            external_label: label,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallingConvention {
    C,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternInfo {
    pub calling_convention: CallingConvention,
    pub symbol_name: String,
}

impl ExternInfo {
    pub fn c(symbol_name: impl Into<String>) -> Self {
        Self {
            calling_convention: CallingConvention::C,
            symbol_name: symbol_name.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WhereConstraint {
    Implements {
        type_param: Entity,
        protocol: Entity,
        protocol_type_args: Vec<Entity>,
    },
    NotImplements {
        type_param: Entity,
        protocol: Entity,
    },
}

impl WhereConstraint {
    pub fn implements(type_param: Entity, protocol: Entity) -> Self {
        Self::Implements {
            type_param,
            protocol,
            protocol_type_args: Vec::new(),
        }
    }

    pub fn implements_with_args(
        type_param: Entity,
        protocol: Entity,
        protocol_type_args: Vec<Entity>,
    ) -> Self {
        Self::Implements {
            type_param,
            protocol,
            protocol_type_args,
        }
    }

    pub fn not_implements(type_param: Entity, protocol: Entity) -> Self {
        Self::NotImplements {
            type_param,
            protocol,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhereClause {
    pub constraints: Vec<WhereConstraint>,
}

impl WhereClause {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
        }
    }

    pub fn add_constraint(&mut self, constraint: WhereConstraint) {
        self.constraints.push(constraint);
    }
}

impl Default for WhereClause {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub entity: Entity,
    pub name: String,
    pub kind: FunctionKind,
    pub type_params: Vec<TypeParamDef>,
    pub params: Vec<ParamDef>,
    pub ret: TyId,
    pub where_clause: Option<WhereClause>,
    pub body: Option<OssaBody>,
    pub extern_info: Option<ExternInfo>,
}

impl FunctionDef {
    pub fn new(entity: Entity, name: impl Into<String>, ret: TyId) -> Self {
        Self {
            entity,
            name: name.into(),
            kind: FunctionKind::Free,
            type_params: Vec::new(),
            params: Vec::new(),
            ret,
            where_clause: None,
            body: None,
            extern_info: None,
        }
    }
}
