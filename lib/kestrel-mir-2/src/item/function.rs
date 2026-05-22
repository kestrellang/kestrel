use kestrel_hecs::Entity;

use crate::body::MirBody;
use crate::ty::ParamConvention;
use crate::{LocalId, TyId};

use super::TypeParamDef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReceiverConvention {
    Borrow,
    MutBorrow,
    Consuming,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionKind {
    Free,
    Method {
        parent: Entity,
        receiver: ReceiverConvention,
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
    ModuleInit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamDef {
    pub name: String,
    pub local: LocalId,
    pub ty: TyId,
    pub convention: ParamConvention,
    pub external_label: Option<String>,
}

impl ParamDef {
    pub fn new(
        name: impl Into<String>,
        local: LocalId,
        ty: TyId,
        convention: ParamConvention,
    ) -> Self {
        Self {
            name: name.into(),
            local,
            ty,
            convention,
            external_label: None,
        }
    }

    pub fn with_label(
        name: impl Into<String>,
        local: LocalId,
        ty: TyId,
        convention: ParamConvention,
        label: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            local,
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
        /// Protocol type arguments as entities from the function's scope.
        /// For `I: SeqIndex[T]`, this would be `[T_entity]`.
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
    pub body: Option<MirBody>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_def_new() {
        let func = FunctionDef::new(Entity::from_raw(1), "test.main", TyId::new(0));
        assert_eq!(func.entity, Entity::from_raw(1));
        assert_eq!(func.name, "test.main");
        assert_eq!(func.kind, FunctionKind::Free);
        assert!(func.params.is_empty());
        assert!(func.body.is_none());
        assert!(func.extern_info.is_none());
    }

    #[test]
    fn param_def() {
        let param = ParamDef::new("x", LocalId::new(0), TyId::new(0), ParamConvention::Consuming);
        assert_eq!(param.name, "x");
        assert_eq!(param.convention, ParamConvention::Consuming);
        assert!(param.external_label.is_none());
    }

    #[test]
    fn param_def_with_label() {
        let param = ParamDef::with_label(
            "index",
            LocalId::new(0),
            TyId::new(0),
            ParamConvention::Consuming,
            Some("at".into()),
        );
        assert_eq!(param.external_label, Some("at".into()));
    }

    #[test]
    fn function_kind_variants() {
        let method = FunctionKind::Method {
            parent: Entity::from_raw(1),
            receiver: ReceiverConvention::Borrow,
        };
        let drop_shim = FunctionKind::DropShim {
            nominal: Entity::from_raw(2),
        };
        assert_ne!(method, drop_shim);
    }

    #[test]
    fn extern_info_c() {
        let info = ExternInfo::c("malloc");
        assert_eq!(info.calling_convention, CallingConvention::C);
        assert_eq!(info.symbol_name, "malloc");
    }

    #[test]
    fn where_clause() {
        let mut wc = WhereClause::new();
        let tp = Entity::from_raw(1);
        let proto = Entity::from_raw(2);
        wc.add_constraint(WhereConstraint::implements(tp, proto));
        wc.add_constraint(WhereConstraint::not_implements(tp, Entity::from_raw(3)));
        assert_eq!(wc.constraints.len(), 2);
    }
}
