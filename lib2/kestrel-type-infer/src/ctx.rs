//! Inference context: solver state, TyVar allocation, and constraint emission.
//!
//! `InferCtx` holds all mutable state for type inference of a single body.
//! It owns the type variable table, pending constraints, and result tables.

use std::collections::{HashMap, HashSet};

use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::body::HirExprId;
use kestrel_hir::res::LocalId;
use kestrel_span2::Span;

use crate::constraint::{CallArg, Constraint};
use crate::error::InferError;
use crate::resolve::TypeResolver;
use crate::ty::{LiteralKind, TyKind, TySlot, TyVar};
use kestrel_ast_builder::NodeKind;

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

    /// Error description strings, computed at report-time before any
    /// cascade-suppression poisoning alters the referenced TyVars.
    /// Parallel to `errors`.
    pub(crate) error_details: Vec<String>,

    /// HirExprIds that have had a Coerce-derived error reported. Used to
    /// suppress duplicate errors on subsequent args of the same call (e.g.
    /// `Point(x: "a", y: "b")` — emit once, not per field).
    pub(crate) errored_coerce_exprs: HashSet<HirExprId>,

    // === Results (populated during solving) ===
    /// Resolved entity for MethodCall/Field expressions.
    pub(crate) resolutions: HashMap<HirExprId, Entity>,

    /// Promotion info for Coerce sites that needed wrapping.
    pub(crate) promotions: HashMap<HirExprId, PromotionInfo>,

    /// Inferred type arguments for generic calls.
    pub(crate) type_args: HashMap<HirExprId, Vec<TyVar>>,

    /// Span of the call/ref expression for each `type_args` entry.
    /// Used by the phase-4 unresolved-type-param diagnostic so we can
    /// point at the right call site without threading HirBody through
    /// the solver.
    pub(crate) type_arg_spans: HashMap<HirExprId, Span>,

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

    /// Where clause associated type substitutions (e.g., Output_entity → Item_tv
    /// from `Item.Output = Item`). Used by lower_hir_ty_sub to substitute
    /// associated type entities found in protocol member signatures.
    pub(crate) where_clause_assoc_subs: Vec<(Entity, TyVar)>,

    /// Maps type parameter entities to their canonical TyVars.
    /// Ensures all references to the same type param share one TyVar,
    /// even after the TyVar is redirected by DirectEquality.
    pub(crate) param_tyvars: HashMap<Entity, TyVar>,

    /// Tracks Def(TypeParameter) expressions that haven't been consumed
    /// by a MethodCall or Call. After constraint generation, remaining
    /// entries are reported as "type parameter used as value" errors.
    pub(crate) type_param_defs: HashMap<HirExprId, Span>,

    /// Flex closure TyVars: 0 explicit params, adapts to any expected arity.
    pub(crate) closure_flex: HashSet<TyVar>,
    /// Implicit-it closure TyVars: 1 param named "it", requires exactly 1-param context.
    pub(crate) closure_it: HashSet<TyVar>,

    /// TyVars that were unified with `Never` while still unresolved.
    /// `unify(Never, Unresolved)` is intentionally a no-op — Never is
    /// the bottom type and shouldn't pin a TyVar that a sibling arm
    /// might still make concrete. But if fixpoint ends and no other
    /// constraint has touched the var, Rust's never-fallback rule says
    /// "it's Never": the entries here get defaulted to Never in phase
    /// 4.25. Populated by `unify::unify`; drained by
    /// `default_never_fallback`.
    pub(crate) never_fallback_targets: HashSet<TyVar>,

    /// Bidirectional hint for the *element* type of the next array literal to be
    /// lowered. Set by `HirStmt::Let` when the annotation is `Array[E]`; read and
    /// cleared by `HirExpr::Array`. Pre-seeding `elem_tv` with the annotated
    /// element type stops the first element from dictating `elem_tv`'s literal
    /// kind and surfacing confusing "expected bool literal got integer literal"
    /// errors for mixed-type arrays.
    pub(crate) expected_array_elem: Option<TyVar>,
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
            error_details: Vec::new(),
            errored_coerce_exprs: HashSet::new(),
            resolutions: HashMap::new(),
            promotions: HashMap::new(),
            type_args: HashMap::new(),
            type_arg_spans: HashMap::new(),
            expr_types: HashMap::new(),
            local_types: HashMap::new(),
            return_ty: TyVar(0),
            owner,
            root,
            where_clause_assoc_subs: Vec::new(),
            param_tyvars: HashMap::new(),
            type_param_defs: HashMap::new(),
            closure_flex: HashSet::new(),
            closure_it: HashSet::new(),
            never_fallback_targets: HashSet::new(),
            expected_array_elem: None,
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

    /// Allocate a TyVar bound to a nominal type, dispatching on the entity's
    /// NodeKind to pick the right variant (Struct / Enum / Protocol / TypeAlias).
    ///
    /// Callers that know the kind should prefer the explicit builders
    /// (`struct_ty`, `enum_ty`, `protocol_ty`, `type_alias`) for clarity.
    pub fn named(&mut self, entity: Entity, args: Vec<TyVar>) -> TyVar {
        let kind = match self.query_ctx.get::<NodeKind>(entity).cloned() {
            Some(NodeKind::Enum) => TyKind::Enum { entity, args },
            Some(NodeKind::Protocol) => TyKind::Protocol { entity, args },
            Some(NodeKind::TypeAlias) => TyKind::TypeAlias { entity, args },
            Some(NodeKind::TypeParameter) => {
                // A type-parameter used as a Named slot: fall back to Param.
                debug_assert!(args.is_empty(), "TypeParameter should not have args");
                return self.param(entity);
            },
            // Struct is the default for Typed entities without a more specific kind
            // (covers Struct, lang.* primitives seeded as leaf types, etc.).
            _ => TyKind::Struct { entity, args },
        };
        let idx = self.types.len() as u32;
        self.types.push(TySlot::Resolved(kind));
        TyVar(idx)
    }

    /// Allocate a TyVar bound to a Struct type.
    pub fn struct_ty(&mut self, entity: Entity, args: Vec<TyVar>) -> TyVar {
        let idx = self.types.len() as u32;
        self.types
            .push(TySlot::Resolved(TyKind::Struct { entity, args }));
        TyVar(idx)
    }

    /// Allocate a TyVar bound to an Enum type.
    pub fn enum_ty(&mut self, entity: Entity, args: Vec<TyVar>) -> TyVar {
        let idx = self.types.len() as u32;
        self.types
            .push(TySlot::Resolved(TyKind::Enum { entity, args }));
        TyVar(idx)
    }

    /// Allocate a TyVar bound to a Protocol type.
    pub fn protocol_ty(&mut self, entity: Entity, args: Vec<TyVar>) -> TyVar {
        let idx = self.types.len() as u32;
        self.types
            .push(TySlot::Resolved(TyKind::Protocol { entity, args }));
        TyVar(idx)
    }

    /// Allocate a TyVar bound to a TypeAlias. Inference will `Reduce` this to
    /// the substituted definition (or leave it for protocol-bound lookup if
    /// the alias is abstract — no `TypeAnnotation`).
    pub fn type_alias(&mut self, entity: Entity, args: Vec<TyVar>) -> TyVar {
        let idx = self.types.len() as u32;
        self.types
            .push(TySlot::Resolved(TyKind::TypeAlias { entity, args }));
        TyVar(idx)
    }

    /// Project an associated type on a base TyVar, emitting the constraint
    /// that drives the solver to resolve it.
    ///
    /// Returns a fresh TyVar that `solve_associated` will unify with the
    /// concrete projected type once `base` is known. Pairs allocation +
    /// constraint emission in a single call — the previous two-step API
    /// (`assoc_projection` raw, then `associated` separately) was easy to
    /// misuse: callers who forgot the constraint caused abstract `Item`
    /// names to leak into diagnostics. Do not add a raw variant back.
    pub fn project_associated(&mut self, base: TyVar, assoc: Entity, span: Span) -> TyVar {
        let name = self
            .query_ctx
            .get::<kestrel_ast_builder::Name>(assoc)
            .map(|n| n.0.clone())
            .unwrap_or_default();
        let result = self.fresh();
        self.associated(base, &name, result, span);
        result
    }

    /// Allocate a TyVar bound to a Tuple type.
    pub fn tuple(&mut self, elements: Vec<TyVar>) -> TyVar {
        let idx = self.types.len() as u32;
        self.types.push(TySlot::Resolved(TyKind::Tuple(elements)));
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
    /// Get or create a Param TyVar for a type parameter entity.
    /// Reuses an existing TyVar if one already exists for this entity,
    /// ensuring all references to the same type param share one TyVar
    /// (so redirects from where clause equalities are visible everywhere).
    pub fn param(&mut self, entity: Entity) -> TyVar {
        if let Some(&tv) = self.param_tyvars.get(&entity) {
            return tv;
        }
        let idx = self.types.len() as u32;
        let tv = TyVar(idx);
        self.types.push(TySlot::Resolved(TyKind::Param { entity }));
        self.param_tyvars.insert(entity, tv);
        tv
    }

    // ===== Error reporting =====

    /// Report an error and return an Error TyVar.
    /// Guarantees every Error TyVar has a corresponding diagnostic.
    ///
    /// The error's description is computed immediately so it reflects TyVar
    /// state *before* any cascade-suppression poisoning rewrites the
    /// referenced TyVars to `TyKind::Error`.
    pub fn report_error(&mut self, err: InferError) -> TyVar {
        let detail = crate::result::describe_error(self, &err);
        self.errors.push(err);
        self.error_details.push(detail);
        let idx = self.types.len() as u32;
        self.types.push(TySlot::Resolved(TyKind::Error));
        TyVar(idx)
    }

    /// Record the instantiated type args for a call/ref expression along
    /// with its source span. Both maps must stay in sync so phase-4 can
    /// report unresolved type parameters at the right site.
    pub fn record_type_args(&mut self, expr: HirExprId, tvs: Vec<TyVar>, span: Span) {
        self.type_args.insert(expr, tvs);
        self.type_arg_spans.insert(expr, span);
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

    /// Overwrite `tv`'s resolved root with `TyKind::Error` so downstream
    /// constraints referencing it absorb silently (cascade suppression).
    pub fn poison(&mut self, tv: TyVar) {
        let root = self.resolve(tv);
        self.types[root.0 as usize] = TySlot::Resolved(TyKind::Error);
    }

    // ===== Constraint emission =====

    pub fn equal(&mut self, a: TyVar, b: TyVar, span: Span) {
        self.constraints.push(Constraint::Equal { a, b, span });
    }

    pub fn coerce(&mut self, from: TyVar, to: TyVar, expr: HirExprId, span: Span) {
        self.constraints.push(Constraint::Coerce {
            from,
            to,
            expr,
            span,
        });
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
        is_call: bool,
        span: Span,
    ) {
        self.constraints.push(Constraint::Member {
            receiver,
            name: name.to_string(),
            args,
            result,
            expr,
            is_call,
            is_static_context: false,
            explicit_type_args: Vec::new(),
            span,
        });
    }

    /// Like `member` but carries explicit type args from the call site
    /// (e.g., `x.flatMap[Int](...)`).
    pub fn member_with_type_args(
        &mut self,
        receiver: TyVar,
        name: &str,
        args: Vec<CallArg>,
        result: TyVar,
        expr: HirExprId,
        is_call: bool,
        explicit_type_args: Vec<kestrel_hir::ty::HirTy>,
        span: Span,
    ) {
        self.constraints.push(Constraint::Member {
            receiver,
            name: name.to_string(),
            args,
            result,
            expr,
            is_call,
            is_static_context: false,
            explicit_type_args,
            span,
        });
    }

    /// Like `member` but marks the constraint as a static context call
    /// (e.g., `T.method()` where T is a type parameter).
    pub fn member_static(
        &mut self,
        receiver: TyVar,
        name: &str,
        args: Vec<CallArg>,
        result: TyVar,
        expr: HirExprId,
        is_call: bool,
        span: Span,
    ) {
        self.constraints.push(Constraint::Member {
            receiver,
            name: name.to_string(),
            args,
            result,
            expr,
            is_call,
            is_static_context: true,
            explicit_type_args: Vec::new(),
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

    pub fn overloaded_call(
        &mut self,
        candidates: Vec<Entity>,
        type_args: Vec<kestrel_hir::ty::HirTy>,
        args: Vec<CallArg>,
        result: TyVar,
        expr: HirExprId,
        span: Span,
    ) {
        self.constraints.push(Constraint::OverloadedCall {
            candidates,
            type_args,
            args,
            result,
            expr,
            span,
        });
    }

    /// Emit a constraint that reduces a TypeAlias TyVar to its substituted
    /// definition (and emits bound obligations).
    pub fn reduce(&mut self, alias: TyVar, result: TyVar, span: Span) {
        self.constraints.push(Constraint::Reduce {
            alias,
            result,
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
