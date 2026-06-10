use crate::{TyId, ValueId};
use kestrel_span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ownership {
    Owned,
    Guaranteed,
}

/// Where a value's storage ultimately roots, for the stage-1 reference
/// escape check: a `ret_borrow` function may only return a borrow whose
/// root is `Param` (a mutable one for `&mutating`), `Static`, or
/// `PointerDerived` — `Local` is the escape error (E494).
///
/// Stamped at value creation and copied O(1) through borrows/projections
/// by `OssaBody::alloc_value` (never walked at check time). Meaningful
/// pre-mono only: the escape check runs at `Stage::Verify`, so mono passes
/// don't maintain it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RootProvenance {
    /// Rooted at the function parameter with this entry-param index.
    /// Mutability is derived from the param's convention at check time
    /// (single source of truth — not duplicated here).
    Param(u32),
    /// Rooted at a global/static — outlives every call.
    Static,
    /// Rooted at a function-local value; the payload is the rooting value,
    /// so escape diagnostics can point at its definition span.
    Local(ValueId),
    /// Fabricated from a `Pointer[T]` via the `ptr_ref`/`ptr_mut_ref`
    /// intrinsics — inherits the pointer's safety contract and may escape.
    /// `mutable` records which accessor fired (feeds the mutable-root rule).
    PointerDerived { mutable: bool },
}

impl RootProvenance {
    /// Placeholder meaning "derive at allocation": `OssaBody::alloc_value`
    /// replaces it — inherit `borrow_source`'s root when present, else
    /// self-root as `Local(id)`. Never observable after `alloc_value`;
    /// hand-built bodies that bypass it (test helpers) may still carry it,
    /// so readers must treat it as `Local` with no known definition.
    pub fn derived() -> Self {
        RootProvenance::Local(ValueId::new(u32::MAX as usize))
    }

    pub fn is_derived_placeholder(self) -> bool {
        matches!(self, RootProvenance::Local(v) if v.index() == u32::MAX as usize)
    }
}

#[derive(Debug, Clone)]
pub struct ValueDef {
    pub ty: TyId,
    pub ownership: Ownership,
    /// For @guaranteed values: which @owned value is frozen by this borrow.
    /// Propagates through block args and forwarding extractions.
    pub borrow_source: Option<ValueId>,
    /// Escape-check provenance root; see `RootProvenance`.
    pub root: RootProvenance,
    /// Source location of the instruction/expression that defined this value,
    /// when known. Metadata only — used to give verifier ICEs a precise span;
    /// excluded from `PartialEq` so it never affects value identity. Synthetic
    /// values (shims, thunks) carry `None`.
    pub span: Option<Span>,
}

// Hand-written so `span` (metadata) is excluded from value identity: two values
// that agree on type/ownership/borrow_source/root are equal regardless of span.
impl PartialEq for ValueDef {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty
            && self.ownership == other.ownership
            && self.borrow_source == other.borrow_source
            && self.root == other.root
    }
}

impl ValueDef {
    pub fn owned(ty: TyId) -> Self {
        Self {
            ty,
            ownership: Ownership::Owned,
            borrow_source: None,
            root: RootProvenance::derived(),
            span: None,
        }
    }

    pub fn guaranteed(ty: TyId, source: ValueId) -> Self {
        Self {
            ty,
            ownership: Ownership::Guaranteed,
            borrow_source: Some(source),
            root: RootProvenance::derived(),
            span: None,
        }
    }

    /// Attach a defining span, builder-style.
    pub fn with_span(mut self, span: Option<Span>) -> Self {
        self.span = span;
        self
    }

    /// Override the provenance root, builder-style (params, globals,
    /// pointer-derived intrinsic results).
    pub fn with_root(mut self, root: RootProvenance) -> Self {
        self.root = root;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `span` is metadata, not identity: two values that agree on
    // ty/ownership/borrow_source/root must compare equal regardless of span.
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

    // `root` IS identity: two values differing only in provenance behave
    // differently at a ret_borrow Return, so they must not compare equal.
    #[test]
    fn root_included_in_equality() {
        let ty = TyId::new(0);
        let a = ValueDef::owned(ty);
        let b = ValueDef::owned(ty).with_root(RootProvenance::Param(0));
        assert_ne!(a, b);
        assert_eq!(a, ValueDef::owned(ty));
    }

    #[test]
    fn derived_placeholder_roundtrip() {
        assert!(RootProvenance::derived().is_derived_placeholder());
        assert!(!RootProvenance::Local(ValueId::new(0)).is_derived_placeholder());
        assert!(!RootProvenance::Static.is_derived_placeholder());
    }
}
