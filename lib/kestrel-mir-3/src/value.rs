use crate::{TyId, ValueId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ownership {
    Owned,
    Guaranteed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueDef {
    pub ty: TyId,
    pub ownership: Ownership,
    /// For @guaranteed values: which @owned value is frozen by this borrow.
    /// Propagates through block args and forwarding extractions.
    pub borrow_source: Option<ValueId>,
}

impl ValueDef {
    pub fn owned(ty: TyId) -> Self {
        Self { ty, ownership: Ownership::Owned, borrow_source: None }
    }

    pub fn guaranteed(ty: TyId, source: ValueId) -> Self {
        Self { ty, ownership: Ownership::Guaranteed, borrow_source: Some(source) }
    }

}
