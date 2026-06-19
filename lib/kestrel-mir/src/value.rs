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

    /// Join two provenance roots into the MOST RESTRICTIVE one (stage 2b:
    /// an aggregate built from several refs carries a single root — the one
    /// that gives the strictest verdict at every return-rule dimension).
    /// `param_convs` is the owning function's entry-param convention list
    /// (a `Param(i)` root's behavior is convention-derived, single source
    /// of truth). Ties keep `self` so diagnostics anchor on the first
    /// component encountered.
    ///
    /// The linear rank bakes in the three check dimensions:
    /// - escape: `Local` beats everything (E494 on return);
    /// - consuming: a consuming param beats borrow roots (E496);
    /// - mutability: every immutable root (borrow param, immutable
    ///   pointer-derived, static) beats every mutable one, so a
    ///   `&mutating`-carrying return demands ALL components be
    ///   mutable-rooted (E495).
    pub fn join(self, other: Self, param_convs: &[crate::ty::ParamConvention]) -> Self {
        let rank = |root: RootProvenance| -> u8 {
            use crate::ty::ParamConvention as PC;
            match root {
                // Includes the derived placeholder ("Local, no known def").
                RootProvenance::Local(_) => 6,
                RootProvenance::Param(i) => match param_convs.get(i as usize) {
                    // Out-of-range index = builder bug; rank conservatively.
                    Some(PC::Consuming) | None => 5,
                    Some(PC::Borrow) => 4,
                    Some(PC::MutBorrow) => 1,
                },
                RootProvenance::PointerDerived { mutable: false } => 3,
                RootProvenance::Static => 2,
                RootProvenance::PointerDerived { mutable: true } => 0,
            }
        };
        if rank(other) > rank(self) { other } else { self }
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

    // join picks the most restrictive root per the return-rule dimensions.
    #[test]
    fn join_rank_order() {
        use crate::ty::ParamConvention as PC;
        let convs = [PC::Consuming, PC::Borrow, PC::MutBorrow];
        let local = RootProvenance::Local(ValueId::new(3));
        let consuming = RootProvenance::Param(0);
        let borrow = RootProvenance::Param(1);
        let mut_borrow = RootProvenance::Param(2);
        let pd_imm = RootProvenance::PointerDerived { mutable: false };
        let pd_mut = RootProvenance::PointerDerived { mutable: true };
        let stat = RootProvenance::Static;

        // Escape dimension: Local beats everything, either side.
        assert_eq!(consuming.join(local, &convs), local);
        assert_eq!(local.join(stat, &convs), local);
        // Consuming beats borrow roots.
        assert_eq!(borrow.join(consuming, &convs), consuming);
        // Mutability dimension: every immutable root beats every mutable one
        // (a &mutating-carrying return needs ALL components mutable-rooted).
        assert_eq!(mut_borrow.join(borrow, &convs), borrow);
        assert_eq!(pd_mut.join(stat, &convs), stat);
        assert_eq!(mut_borrow.join(pd_imm, &convs), pd_imm);
        // Ties keep self (stable diagnostic anchor).
        let borrow2 = RootProvenance::Param(1);
        assert_eq!(borrow.join(borrow2, &convs), borrow);
        // Placeholder ranks as Local.
        assert_eq!(stat.join(RootProvenance::derived(), &convs), RootProvenance::derived());
        // Out-of-range param index ranks conservatively (consuming).
        let oob = RootProvenance::Param(9);
        assert_eq!(borrow.join(oob, &convs), oob);
    }
}
