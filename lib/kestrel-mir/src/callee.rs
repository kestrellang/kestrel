use kestrel_hecs::Entity;

use crate::item::witness::WitnessMethodKey;
use crate::{MonoFuncId, TyId, ValueId};

#[derive(Debug, Clone, PartialEq)]
pub enum Callee {
    Direct {
        func: Entity,
        type_args: Vec<TyId>,
        self_type: Option<TyId>,
    },
    Resolved(MonoFuncId),
    Thin(ValueId),
    Thick(ValueId),
    Witness {
        protocol: Entity,
        method: WitnessMethodKey,
        self_type: TyId,
        method_type_args: Vec<TyId>,
    },
}

impl Callee {
    pub fn direct(func: Entity) -> Self {
        Self::Direct {
            func,
            type_args: vec![],
            self_type: None,
        }
    }

    pub fn direct_with_args(func: Entity, type_args: Vec<TyId>, self_type: Option<TyId>) -> Self {
        Self::Direct {
            func,
            type_args,
            self_type,
        }
    }

    /// Extract the ValueId for indirect callees (Thin/Thick).
    pub fn value(&self) -> Option<ValueId> {
        match self {
            Self::Thin(v) | Self::Thick(v) => Some(*v),
            _ => None,
        }
    }
}
