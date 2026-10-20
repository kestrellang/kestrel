//! The solver's view for the `kestrel-entail` kernel (bidi design §7.2,
//! "solver adapter").
//!
//! Converts `TyVar`s into kernel types, where an open var is [`ETy::Var`] (or
//! [`ETy::LitVar`] for an open literal) — so every applicability or
//! conformance question touching it answers `Unknown`, and the solver keeps
//! the goal as an obligation instead of permitting it (N1). This replaces the
//! old `reify_tv` → `HirTy::Error` → permit route.

use std::cell::RefCell;

use kestrel_entail::{ETy, EntailLayer, Goal, Supply, Tri};
use kestrel_hecs::Entity;

use crate::ctx::InferCtx;
use crate::entail_layer::FrontEndLayer;
use crate::ty::{LiteralKind, TyKind, TySlot, TyVar};

pub(crate) struct SolverLayer<'a, 'i> {
    fe: FrontEndLayer<'a, 'i>,
    ctx: &'a InferCtx<'i>,
    /// Literal kinds behind [`ETy::LitVar`] payloads.
    lits: RefCell<Vec<LiteralKind>>,
}

impl<'a, 'i> SolverLayer<'a, 'i> {
    pub(crate) fn new(ctx: &'a InferCtx<'i>) -> Self {
        Self {
            fe: FrontEndLayer::new(ctx.query_ctx, ctx.root),
            ctx,
            lits: RefCell::new(Vec::new()),
        }
    }

    /// Kernel view of a solver type var.
    pub(crate) fn ety(&self, tv: TyVar) -> ETy {
        match self.ctx.slot(self.ctx.resolve(tv)) {
            TySlot::Resolved(kind) => self.kind_ety(&kind.clone()),
            TySlot::Unresolved { literal: Some(lit) } => {
                let mut lits = self.lits.borrow_mut();
                lits.push(*lit);
                ETy::LitVar((lits.len() - 1) as u32)
            },
            _ => ETy::Var,
        }
    }

    /// Kernel view of a resolved solver type.
    pub(crate) fn kind_ety(&self, kind: &TyKind) -> ETy {
        match kind {
            TyKind::Struct { entity, args }
            | TyKind::Enum { entity, args }
            | TyKind::Protocol { entity, args } => ETy::Nominal {
                entity: *entity,
                args: args.iter().map(|&a| self.ety(a)).collect(),
            },
            TyKind::Tuple(elems) => ETy::Tuple(elems.iter().map(|&a| self.ety(a)).collect()),
            TyKind::Never => ETy::Never,
            TyKind::Param { entity } => ETy::Param(*entity),
            TyKind::AssocProjection { base, assoc } => ETy::Proj {
                base: Box::new(self.ety(*base)),
                assoc: *assoc,
            },
            TyKind::Ref { pointee, mutating } => ETy::Ref {
                inner: Box::new(self.ety(*pointee)),
                mutating: *mutating,
            },
            TyKind::Error => ETy::Poison,
            TyKind::SelfType { entity } => self
                .fe
                .abstract_of(kestrel_hir::ty::HirTy::SelfType(*entity, kestrel_span::Span::synthetic(0))),
            // Functions, opaques and unreduced aliases: nothing for the
            // kernel to evaluate; the layer decides them structurally.
            TyKind::Function { .. } | TyKind::Opaque { .. } | TyKind::TypeAlias { .. } => self
                .fe
                .abstract_of(kestrel_hir::ty::HirTy::Error(kestrel_span::Span::synthetic(0))),
        }
    }
}

impl EntailLayer for SolverLayer<'_, '_> {
    fn supply(&self, ty: &ETy, protocol: Entity) -> Supply {
        self.fe.supply(ty, protocol)
    }

    fn assoc_binding(&self, base: &ETy, assoc: Entity) -> Option<ETy> {
        self.fe.assoc_binding(base, assoc)
    }

    fn literal_admits(&self, kind: u32, entity: Entity) -> bool {
        let lit = self.lits.borrow()[kind as usize];
        let ty = TyKind::Struct {
            entity,
            args: Vec::new(),
        };
        crate::unify::conforms_to_literal_protocol(self.ctx, &ty, lit)
    }
}

/// Do `extension`'s target pattern and where clauses hold for a receiver of
/// kind `recv`? Member applicability, bidi §5.1 step 4.
pub(crate) fn extension_applicability(ctx: &InferCtx<'_>, extension: Entity, recv: &TyKind) -> Tri {
    let layer = SolverLayer::new(ctx);
    let recv = layer.kind_ety(recv);
    let target = layer.fe.extension_target(extension);
    let clauses = layer.fe.extension_clauses(extension);
    kestrel_entail::extension_applies(&layer, &target, &clauses, &recv)
}

/// Does `ty` satisfy `protocol`, conditional conformances included?
pub(crate) fn conformance(ctx: &InferCtx<'_>, ty: TyVar, protocol: Entity) -> Tri {
    let layer = SolverLayer::new(ctx);
    let goal = Goal::Conforms {
        ty: layer.ety(ty),
        protocol,
    };
    kestrel_entail::entails(&layer, &goal)
}

/// [`conformance`] for an already-resolved receiver kind.
pub(crate) fn kind_conformance(ctx: &InferCtx<'_>, recv: &TyKind, protocol: Entity) -> Tri {
    let layer = SolverLayer::new(ctx);
    let goal = Goal::Conforms {
        ty: layer.kind_ety(recv),
        protocol,
    };
    kestrel_entail::entails(&layer, &goal)
}

/// Does the type still mention an open var (so an Unknown verdict is
/// "not determined yet" rather than an abstract position)?
pub(crate) fn has_open_var(ctx: &InferCtx<'_>, ty: TyVar) -> bool {
    SolverLayer::new(ctx).ety(ty).has_var()
}
