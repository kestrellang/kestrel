use crate::{TyId, ValueId};
use kestrel_span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ownership {
    Owned,
    Guaranteed,
}

#[derive(Debug, Clone)]
pub struct ValueDef {
    pub ty: TyId,
    pub ownership: Ownership,
    /// For @guaranteed values: which @owned value is frozen by this borrow.
    /// Propagates through block args and forwarding extractions.
    pub borrow_source: Option<ValueId>,
    /// Source location of the instruction/expression that defined this value,
    /// when known. Metadata only — used to give verifier ICEs a precise span;
    /// excluded from `PartialEq` so it never affects value identity. Synthetic
    /// values (shims, thunks) carry `None`.
    pub span: Option<Span>,
}

// Hand-written so `span` (metadata) is excluded from value identity: two values
// that agree on type/ownership/borrow_source are equal regardless of span.
impl PartialEq for ValueDef {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty
            && self.ownership == other.ownership
            && self.borrow_source == other.borrow_source
    }
}

impl ValueDef {
    pub fn owned(ty: TyId) -> Self {
        Self {
            ty,
            ownership: Ownership::Owned,
            borrow_source: None,
            span: None,
        }
    }

    pub fn guaranteed(ty: TyId, source: ValueId) -> Self {
        Self {
            ty,
            ownership: Ownership::Guaranteed,
            borrow_source: Some(source),
            span: None,
        }
    }

    /// Attach a defining span, builder-style.
    pub fn with_span(mut self, span: Option<Span>) -> Self {
        self.span = span;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `span` is metadata, not identity: two values that agree on
    // ty/ownership/borrow_source must compare equal regardless of span.
    #[test]
    fn span_excluded_from_equality() {
        let ty = TyId::new(0);
        let a = ValueDef::owned(ty);
        let b = ValueDef::owned(ty).with_span(Some(Span::synthetic(0)));
        assert_eq!(a, b);
        assert_eq!(a.with_span(Some(Span::new(0, 1..2))), b);
    }

    #[test]
    fn with_span_sets_field() {
        let v = ValueDef::owned(TyId::new(0)).with_span(Some(Span::synthetic(0)));
        assert!(v.span.is_some());
    }
}
