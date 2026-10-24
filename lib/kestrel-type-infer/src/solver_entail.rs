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
            TyKind::Function { .. }
            | TyKind::Opaque { .. }
            | TyKind::RawExistential { .. }
            | TyKind::TypeAlias { .. } => self.fe.abstract_of(kestrel_hir::ty::HirTy::Error(
                kestrel_span::Span::synthetic(0),
            )),
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
    let open = recv.has_var();
    let target = layer.fe.extension_target(extension);
    let clauses = layer.fe.extension_clauses(extension);
    settle_abstract(kestrel_entail::extension_applies(&layer, &target, &clauses, &recv), open)
}

/// Does `ty` satisfy `protocol`, conditional conformances included?
///
/// Callers have already confirmed the conformance is *declared*
/// (`TypeResolver::conforms_to`, which consults a param's bounds). So an
/// `Unknown` caused only by an abstract position (a param, an unreduced
/// projection) is that declaration's answer and holds; only an open var
/// keeps the goal `Unknown` (re-queued until it resolves).
pub(crate) fn conformance(ctx: &InferCtx<'_>, ty: TyVar, protocol: Entity) -> Tri {
    let layer = SolverLayer::new(ctx);
    let ety = layer.ety(ty);
    let open = ety.has_var();
    let verdict = kestrel_entail::entails(&layer, &Goal::Conforms { ty: ety, protocol });
    settle_abstract(verdict, open)
}

/// `Unknown` with no open var anywhere in the question can never become
/// known by solving further: it is an abstract position the caller's own
/// declared-conformance check already vouched for.
fn settle_abstract(verdict: Tri, open: bool) -> Tri {
    if verdict == Tri::Unknown && !open { Tri::Holds } else { verdict }
}

/// [`conformance`] for an already-resolved receiver kind.
pub(crate) fn kind_conformance(ctx: &InferCtx<'_>, recv: &TyKind, protocol: Entity) -> Tri {
    let layer = SolverLayer::new(ctx);
    let ety = layer.kind_ety(recv);
    let open = ety.has_var();
    settle_abstract(kestrel_entail::entails(&layer, &Goal::Conforms { ty: ety, protocol }), open)
}

/// Does the type still mention an open var (so an Unknown verdict is
/// "not determined yet" rather than an abstract position)?
pub(crate) fn has_open_var(ctx: &InferCtx<'_>, ty: TyVar) -> bool {
    SolverLayer::new(ctx).ety(ty).has_var()
}

/// Rule C (bidi design §4): the type an open literal receiver of an operator
/// takes from the operator's *expected result*. Candidates are the
/// argument-free nominal types mentioned in the result, plus the literal's
/// default; a candidate survives when it can be written as the literal, has
/// the operator method, and that method's return type fits the result. One
/// survivor wins; several pick the default; none decides nothing (the
/// literal defaults later and the mismatch is reported against the
/// expectation). The candidate set is syntactic and local (§11.2): no
/// enumeration of every conformer of the literal protocol.
pub(crate) fn rule_c_pick(
    ctx: &InferCtx<'_>,
    literal: LiteralKind,
    result: TyVar,
    method: &str,
    args: &[crate::constraint::CallArg],
) -> Option<Entity> {
    use kestrel_hir::builtin::Builtin;
    let layer = SolverLayer::new(ctx);
    let expected = layer.ety(result);
    if matches!(expected, ETy::Var | ETy::LitVar(_)) {
        return None;
    }
    let default = match literal {
        LiteralKind::Integer => ctx.resolver.builtin(Builtin::DefaultIntegerLiteralType),
        LiteralKind::Float => ctx.resolver.builtin(Builtin::DefaultFloatLiteralType),
        _ => None,
    }?;
    let mut candidates = Vec::new();
    collect_plain_nominals(ctx, &expected, &mut candidates);
    if !candidates.contains(&default) {
        candidates.push(default);
    }
    let survivors: Vec<Entity> = candidates
        .into_iter()
        .filter(|&c| {
            let kind = TyKind::Struct {
                entity: c,
                args: Vec::new(),
            };
            if !crate::unify::conforms_to_literal_protocol(ctx, &kind, literal) {
                return false;
            }
            let Ok(resolution) = ctx.resolver.resolve_member(&kind, method, args) else {
                return false;
            };
            let output = layer.fe.hir_to_ety(&resolution.return_type);
            !kestrel_entail::entails(&layer, &Goal::Equal(output, expected.clone())).fails()
        })
        .collect();
    match survivors.as_slice() {
        [one] => Some(*one),
        [] => None,
        many if many.contains(&default) => Some(default),
        _ => None,
    }
}

/// Argument-free struct/enum entities mentioned anywhere in `ty`.
fn collect_plain_nominals(ctx: &InferCtx<'_>, ty: &ETy, out: &mut Vec<Entity>) {
    match ty {
        ETy::Nominal { entity, args } => {
            let generic = ctx
                .query_ctx
                .get::<kestrel_ast_builder::TypeParams>(*entity)
                .is_some_and(|tp| !tp.0.is_empty());
            if args.is_empty() && !generic && !out.contains(entity) {
                out.push(*entity);
            }
            for a in args {
                collect_plain_nominals(ctx, a, out);
            }
        },
        ETy::Tuple(elems) => elems.iter().for_each(|e| collect_plain_nominals(ctx, e, out)),
        ETy::Ref { inner, .. } => collect_plain_nominals(ctx, inner, out),
        _ => {},
    }
}
