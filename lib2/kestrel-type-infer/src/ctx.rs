//! Inference context: solver state, TyVar allocation, and constraint emission.
//!
//! `InferCtx` holds all mutable state for type inference of a single body.
//! It owns the type variable table, pending constraints, and result tables.

use std::collections::HashMap;

use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::body::HirExprId;
use kestrel_hir::res::LocalId;
use kestrel_span2::Span;

use crate::constraint::{CallArg, Constraint};
use crate::error::InferError;
use crate::resolve::TypeResolver;
use crate::ty::{LiteralKind, TyKind, TySlot, TyVar};

/// Mutable state for type inference of a single function/init/getter body.
pub struct InferCtx<'a> {
    /// Type resolver for querying the world (members, conformances, builtins).
    pub(crate) resolver: &'a dyn TypeResolver,

    /// Direct ECS access for reading entity structure (TypeParams, Callable, etc.)
    pub(crate) query_ctx: &'a QueryContext<'a>,

    /// All type variables. Index = TyVar(n).
    pub(crate) types: Vec<TySlot>,

    /// Pending constraints to solve.
    pub(crate) constraints: Vec<Constraint>,

    /// Accumulated errors (each produces an Error TyVar).
    pub(crate) errors: Vec<InferError>,

    // === Results (populated during solving) ===
    /// Resolved entity for MethodCall/Field expressions.
    pub(crate) resolutions: HashMap<HirExprId, Entity>,

    /// Promotion info for Coerce sites that needed wrapping.
    pub(crate) promotions: HashMap<HirExprId, PromotionInfo>,

    /// Inferred type arguments for generic calls.
    pub(crate) type_args: HashMap<HirExprId, Vec<TyVar>>,

    // === Bookkeeping ===
    /// Type assigned to each HirExpr during constraint generation.
    pub(crate) expr_types: HashMap<HirExprId, TyVar>,

    /// Type assigned to each Local during constraint generation.
    pub(crate) local_types: HashMap<LocalId, TyVar>,

    /// The function's declared return type TyVar.
    pub(crate) return_ty: TyVar,

    /// Entity being inferred (function/init/getter).
    #[allow(dead_code)]
    pub(crate) owner: Entity,
    pub(crate) root: Entity,
}

/// Info about a promotion inserted at a Coerce site.
#[derive(Clone, Debug)]
pub struct PromotionInfo {
    /// The `FromValue.from()` method entity to call.
    pub method: Entity,
    /// Target type (what we're promoting to).
    pub target_ty: TyVar,
}

impl<'a> InferCtx<'a> {
    pub fn new(
        resolver: &'a dyn TypeResolver,
        query_ctx: &'a QueryContext<'a>,
        owner: Entity,
        root: Entity,
    ) -> Self {
        // Allocate a dummy TyVar(0) for the return type — will be overwritten
        let mut types = Vec::new();
        types.push(TySlot::Unresolved { literal: None });

        Self {
            resolver,
            query_ctx,
            types,
            constraints: Vec::new(),
            errors: Vec::new(),
            resolutions: HashMap::new(),
            promotions: HashMap::new(),
            type_args: HashMap::new(),
            expr_types: HashMap::new(),
            local_types: HashMap::new(),
            return_ty: TyVar(0),
            owner,
            root,
        }
    }

    // ===== TyVar creation =====

    /// Allocate a fresh unconstrained type variable.
    pub fn fresh(&mut self) -> TyVar {
        let idx = self.types.len() as u32;
        self.types.push(TySlot::Unresolved { literal: None });
        TyVar(idx)
    }

    /// Allocate a fresh type variable with a literal marker.
    pub fn fresh_literal(&mut self, kind: LiteralKind) -> TyVar {
        let idx = self.types.len() as u32;
        self.types.push(TySlot::Unresolved {
            literal: Some(kind),
        });
        TyVar(idx)
    }

    /// Allocate a TyVar bound to a Named type.
    pub fn named(&mut self, entity: Entity, args: Vec<TyVar>) -> TyVar {
        let idx = self.types.len() as u32;
        self.types
            .push(TySlot::Resolved(TyKind::Named { entity, args }));
        TyVar(idx)
    }

    /// Allocate a TyVar bound to a Tuple type.
    pub fn tuple(&mut self, elements: Vec<TyVar>) -> TyVar {
        let idx = self.types.len() as u32;
        self.types
            .push(TySlot::Resolved(TyKind::Tuple(elements)));
        TyVar(idx)
    }

    /// Allocate a TyVar bound to a Function type.
    pub fn function(&mut self, params: Vec<TyVar>, ret: TyVar) -> TyVar {
        let idx = self.types.len() as u32;
        self.types
            .push(TySlot::Resolved(TyKind::Function { params, ret }));
        TyVar(idx)
    }

    /// Allocate a TyVar bound to Never.
    pub fn never(&mut self) -> TyVar {
        let idx = self.types.len() as u32;
        self.types.push(TySlot::Resolved(TyKind::Never));
        TyVar(idx)
    }

    /// Allocate a TyVar bound to a type parameter.
    pub fn param(&mut self, entity: Entity) -> TyVar {
        let idx = self.types.len() as u32;
        self.types
            .push(TySlot::Resolved(TyKind::Param { entity }));
        TyVar(idx)
    }

    // ===== Error reporting =====

    /// Report an error and return an Error TyVar.
    /// Guarantees every Error TyVar has a corresponding diagnostic.
    pub fn report_error(&mut self, err: InferError) -> TyVar {
        self.errors.push(err);
        let idx = self.types.len() as u32;
        self.types.push(TySlot::Resolved(TyKind::Error));
        TyVar(idx)
    }

    // ===== Resolution =====

    /// Follow redirect chains to find the root TyVar.
    pub fn resolve(&self, tv: TyVar) -> TyVar {
        match &self.types[tv.0 as usize] {
            TySlot::Redirect(target) => self.resolve(*target),
            _ => tv,
        }
    }

    /// Resolve and return a reference to the slot.
    pub fn slot(&self, tv: TyVar) -> &TySlot {
        let resolved = self.resolve(tv);
        &self.types[resolved.0 as usize]
    }

    /// Check if a TyVar is resolved to a concrete type (not Unresolved).
    pub fn is_concrete(&self, tv: TyVar) -> bool {
        matches!(self.slot(tv), TySlot::Resolved(_))
    }

    /// Check if a TyVar is resolved to Error.
    pub fn is_error(&self, tv: TyVar) -> bool {
        matches!(self.slot(tv), TySlot::Resolved(TyKind::Error))
    }

    // ===== Constraint emission =====

    pub fn equal(&mut self, a: TyVar, b: TyVar, span: Span) {
        self.constraints.push(Constraint::Equal { a, b, span });
    }

    pub fn coerce(&mut self, from: TyVar, to: TyVar, expr: HirExprId, span: Span) {
        self.constraints
            .push(Constraint::Coerce { from, to, expr, span });
    }

    pub fn conforms(&mut self, ty: TyVar, protocol: Entity, span: Span) {
        self.constraints
            .push(Constraint::Conforms { ty, protocol, span });
    }

    pub fn associated(&mut self, container: TyVar, name: &str, result: TyVar, span: Span) {
        self.constraints.push(Constraint::Associated {
            container,
            name: name.to_string(),
            result,
            span,
        });
    }

    pub fn member(
        &mut self,
        receiver: TyVar,
        name: &str,
        args: Vec<CallArg>,
        result: TyVar,
        expr: HirExprId,
        span: Span,
    ) {
        self.constraints.push(Constraint::Member {
            receiver,
            name: name.to_string(),
            args,
            result,
            expr,
            span,
        });
    }

    pub fn call(
        &mut self,
        callee: TyVar,
        args: Vec<CallArg>,
        result: TyVar,
        expr: HirExprId,
        span: Span,
    ) {
        self.constraints.push(Constraint::Call {
            callee,
            args,
            result,
            expr,
            span,
        });
    }

    pub fn implicit(
        &mut self,
        expected: TyVar,
        name: &str,
        args: Vec<CallArg>,
        result: TyVar,
        expr: HirExprId,
        span: Span,
    ) {
        self.constraints.push(Constraint::Implicit {
            expected,
            name: name.to_string(),
            args,
            result,
            expr,
            span,
        });
    }
}
