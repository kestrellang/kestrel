//! Inference context: solver state, TyVar allocation, and constraint emission.
//!
//! `InferCtx` holds all mutable state for type inference of a single body.
//! It owns the type variable table, pending constraints, and result tables.

use std::collections::{HashMap, HashSet};

use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::body::HirExprId;
use kestrel_hir::res::LocalId;
use kestrel_span::Span;

use kestrel_hir::ty::HirTy;

use crate::constraint::{CallArg, Constraint};
use crate::error::InferError;
use crate::resolve::TypeResolver;
use crate::ty::{LiteralKind, TyKind, TySlot, TyVar};
use kestrel_ast_builder::NodeKind;

/// One level of an in-progress `Indirection` peel, accumulated during solving.
/// `target_tv` is resolved to a concrete `ResolvedTy` in `build_result` to
/// form the final `crate::result::IndirectionPeel`.
pub(crate) struct PendingPeel {
    pub read_method: kestrel_hecs::Entity,
    pub mut_method: Option<kestrel_hecs::Entity>,
    pub target_tv: TyVar,
}

/// Which *projection* a cached `where_clause_assoc_subs` TyVar stands for.
///
/// The base is a `TyVar`, not a `WhereSubject`: this table is a memo over the
/// *inference* world, six of its seven readers hold a TyVar and no subject, and
/// `lower_subject` is not invertible — by the time a reader asks, the base may
/// have unified with a concrete type no `WhereSubject` names.
///
/// The base is stored **raw** — never `resolve()`d at push. `DirectEquality`
/// writes `TySlot::Redirect` straight into a param slot and ordinary
/// unification redirects constantly, so a canonical snapshot taken at push goes
/// stale and yields a false *miss* (the over-rejection failure mode). Both
/// sides are resolved at lookup instead; union-find is monotone, so
/// resolve-at-lookup only ever becomes *more* permissive as inference proceeds.
/// Same convention as `witness_protocol_args`.
#[derive(Clone, Copy, Debug)]
struct AssocSubKey {
    /// `None` only for a genuinely baseless binding — `DirectEquality` on a
    /// TypeAlias entity (`where Item = Int64`), which names no receiver.
    base: Option<TyVar>,
    assoc: Entity,
}

/// What a *strict* `(resolved base, assoc)` key answers at a read site,
/// relative to the base-blind answer. Detection only — see
/// [`InferCtx::audit_assoc_sub`].
#[derive(Clone, Copy, PartialEq, Eq)]
enum SubVerdict {
    /// The memo has no entry for this assoc at all; strict and blind agree.
    None_,
    /// Strict and blind pick the same TyVar.
    Match,
    /// Strict picks a *different* TyVar than blind — a live receiver confusion.
    Mismatch,
    /// Blind found an entry; strict finds none (no entry shares the base).
    Miss,
    /// Baseless query with >1 candidate: strict would bail to the general path.
    Ambiguous,
}

impl SubVerdict {
    fn tag(self) -> &'static str {
        match self {
            SubVerdict::None_ => "NONE",
            SubVerdict::Match => "MATCH",
            SubVerdict::Mismatch => "MISMATCH",
            SubVerdict::Miss => "MISS",
            SubVerdict::Ambiguous => "AMBIGUOUS",
        }
    }
}

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

    /// HirExprIds used as `match` scrutinees. A scrutinee is a VALUE context
    /// (stage-1 transparent place): a ref-returning call here binds its
    /// result var to the POINTEE (decay) in `bind_call_result`, so patterns
    /// never see a ref. Patterns stay wired directly to the scrutinee var.
    pub(crate) scrutinee_exprs: HashSet<HirExprId>,

    /// HirExprIds that are a `Call`'s direct callee. A direct callee Def
    /// legitimately carries `TyKind::Function { ret: Ref }` — exempted from
    /// the E491 function-as-value check in `validate_ref_placement`.
    pub(crate) direct_callee_exprs: HashSet<HirExprId>,

    /// HirExprIds that initialize a `let`/`var` binding. Bindings are VALUE
    /// contexts (decay): a ref-returning call here binds its result to the
    /// POINTEE in `bind_call_result` — order-independently (the binding's
    /// Coerce may unify local ≡ result before the member resolves, which
    /// would defeat the coerce-side decay arm).
    pub(crate) binding_init_exprs: HashSet<HirExprId>,

    /// HirExprIds that are an `Assign`'s TARGET. A `&mutating T`-returning
    /// target (`arr.mutableAt(index: i) = v`) types as the POINTEE so the
    /// value's Coerce just works — order-independently (the value side may
    /// literal-link the target var before the member resolves). Place-ness
    /// for the analyzer and MIR comes from the resolved callee's
    /// `CallableRefReturn`, not from this expression's recorded type.
    pub(crate) assign_target_exprs: HashSet<HirExprId>,

    /// HirExprIds in ALWAYS-DECAY value positions: `if`/`match` ARM VALUES
    /// (refs cannot cross merges) and array/tuple/dict LITERAL ELEMENTS
    /// (aggregates own their elements). A ref-returning call here binds its
    /// result to the POINTEE in `bind_call_result`, so the arm-merge /
    /// element `Equal`s only ever see owned types — order-independently,
    /// like the other value-context sets. The constraints stay `Equal`, so
    /// bidirectional back-flow (annotation → arms/elements, literal
    /// defaulting, ExpressibleByArrayLiteral targeting) is untouched.
    pub(crate) always_decay_exprs: HashSet<HirExprId>,

    /// TyVars allocated for ENUM-PATTERN BINDERS whose `ImplicitPat` may
    /// fire late (the scrutinee can wait on literal defaulting — e.g.
    /// `for x in [1,2,3].refs()`: the array's element literal pins T, which
    /// pins the iterator, which pins `next()`'s `Optional[&T]`). A binder's
    /// type comes from its PATTERN, never its uses (the AssignTarget
    /// principle): while `pattern_binder_gate` is up, a use-site Coerce
    /// FROM one of these still-unresolved vars against a resolved target
    /// defers instead of pinning via plain unify (which manufactured
    /// "expected Int64 got &Int64" when the late payload equate landed).
    pub(crate) pattern_binder_tvs: HashSet<TyVar>,

    /// Drops after the literal-relaxation loop (next to the AssignTarget
    /// stall-breaker): at that point every fireable pattern has fired, so a
    /// binder still unresolved has no pattern-side source and its uses may
    /// pin it (old behavior) rather than deadlock.
    pub(crate) pattern_binder_gate: bool,

    /// HirExprIds of `HirExpr::ProtocolCall` nodes that sit inside a
    /// `HirExpr::Sugar` wrapper (the desugaring's primary call). When the
    /// `ProtocolCall` arm of `gen_expr` sees its own `id` in this set, it
    /// emits a poison-on-failure `Conforms` so a non-conforming receiver
    /// stops the cascade by poisoning downstream Member/ImplicitMember
    /// errors inside the desugared subtree. Populated by Sugar's per-kind
    /// gen helpers before they recurse into `inner`.
    pub(crate) poison_protocol_call_recv_on_failure: HashSet<HirExprId>,

    /// Member-access exprs that came from a desugared `ProtocolCall`
    /// (operators, for-in, try). The lazy `Indirection` peel in `solve_member`
    /// is SKIPPED for these — operators/conformances forward via explicit
    /// `extend`, never through the member peel (the receiver-only rule, R7).
    /// Populated by the `ProtocolCall` arm of `generate.rs`.
    pub(crate) protocol_dispatch_members: HashSet<HirExprId>,

    /// Subset of `protocol_dispatch_members`: exprs desugared from OPERATORS
    /// (HIR `ProtocolCall.from_operator`). These members follow the
    /// `Output = Self`-by-convention operator protocols, which licenses the
    /// solver's literal passes (receiver-from-result inference, operator
    /// shape projection) — for-in/try sugar must NOT get that reasoning.
    pub(crate) operator_members: HashSet<HirExprId>,

    // === Results (populated during solving) ===
    /// Resolved entity for MethodCall/Field expressions.
    pub(crate) resolutions: HashMap<HirExprId, Entity>,

    /// `Indirection`-peel plan per member-access expr (outer→inner chain).
    /// Accumulated by the peel arm in `solve_member`; `build_result` resolves
    /// each `target_tv` and copies the chain to `TypedBody.indirection_peels`.
    pub(crate) indirection_peels: HashMap<HirExprId, Vec<PendingPeel>>,

    /// MethodCall exprs where the resolution went through a field access.
    /// Maps expr → field entity. MIR lowering must interpose a field
    /// projection before the call (the resolution points to the subscript/
    /// method on the field's type, not the receiver's type).
    pub(crate) field_subscripts: HashMap<HirExprId, Entity>,

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
    ///
    /// **Private to this module on purpose** (G17 stage 3a). Every reader goes
    /// through [`InferCtx::assoc_sub`], so the `(base, assoc)` comparison rule
    /// lives in exactly one place — [`InferCtx::lookup_assoc_sub`] — and no
    /// reader can re-derive its own. Every reader is base-aware (C3); the last
    /// base-blind one, the cross-protocol `Name` fallback, was deleted by G25.
    where_clause_assoc_subs: Vec<(AssocSubKey, TyVar)>,

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

    /// Kind-flexible function TyVars — *bare* callable values (named `Def`s and
    /// enum-case constructors, built by [`InferCtx::function`]). A bare function
    /// pointer has no environment, so it satisfies every closure kind: in
    /// `unify` and `solve_coerce` the flex side ADOPTS the other side's kind
    /// instead of forcing equality. Modeled as a TyVar set rather than a fifth
    /// `FnTypeKind` variant (plan D1) — a wildcard kind that survives
    /// unification would poison control-flow merge points.
    pub(crate) kind_flex: HashSet<TyVar>,

    /// Accepted CROSS-kind coercions: `expr → (source kind, target kind)`.
    /// Surfaced on `TypedBody.kind_coercions`. NOTE: currently RECORDED but
    /// UNCONSUMED — mir-lower ended up deriving every representation
    /// conversion from the value/slot types directly (bare→owning widens via
    /// the nop-shim `ApplyPartial` path; escaping→normal/consuming reads the
    /// shared first two words). Kept as the audit trail of the solver's
    /// passing-table decisions and as the channel a future conversion that
    /// *cannot* be type-derived would use.
    pub(crate) kind_coercions:
        HashMap<HirExprId, (kestrel_ast::FnTypeKind, kestrel_ast::FnTypeKind)>,

    /// HirExprIds of closure-*literal* expressions, recorded during constraint
    /// generation. `solve_call` uses this to gate the no-annotation `MutBorrow`
    /// convention upgrade to literals only — a named function value's
    /// conventions reflect its real ABI and must not be rewritten.
    pub(crate) closure_literal_exprs: HashSet<HirExprId>,

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

    /// Bidirectional hint for the key/value types of the next dictionary
    /// literal. Set by `HirStmt::Let` when the annotation is `Dictionary[K, V]`
    /// or the `[K: V]` type operator has already lowered to Dictionary.
    pub(crate) expected_dict_entry: Option<(TyVar, TyVar)>,

    /// Expression ID of the accumulator init call inside a Sugar::StringInterpolation.
    /// Set by `mark_sugar_primary`, consumed by `gen_expr` for Call. Replaces the
    /// concrete DefaultStringInterpolation init with a deferred type variable so
    /// the accumulator type can be resolved from context.
    pub(crate) interpolation_init_expr: Option<kestrel_hir::body::HirExprId>,

    /// The accumulator type variable for the current string interpolation.
    /// Set during the init interception, consumed by the Sugar handler to
    /// emit the InterpolationLink constraint.
    pub(crate) interpolation_acc_tv: Option<TyVar>,

    /// TyVars created from an explicit `_` (HirTy::Infer) in a type-argument
    /// position. These intentionally stay unresolved when the caller doesn't
    /// care about the value (e.g. `lang.cast_ptr[_, T](p)`). They must not
    /// generate "could not infer type" diagnostics. Wildcard status propagates
    /// through unification so that any TyVar unified with a wildcard is also
    /// treated as one.
    pub(crate) wildcard_tvars: HashSet<TyVar>,

    /// Witness protocol args, keyed by `(canonical container TyVar, protocol)`.
    /// Populated when a `where T: Proto[Args]` clause emits its Conforms
    /// constraint — the args lower to TyVars and get cached here. Read by
    /// `solve_associated` to substitute the extension's free TypeParams when
    /// projecting through an `extend ConcreteType: Proto[FreeParams]` binding.
    pub(crate) witness_protocol_args: HashMap<(TyVar, Entity), Vec<TyVar>>,

    /// Per-loop type variable for break targets. `break` unifies `()` with
    /// the innermost (or label-matched) entry; the loop expr returns the
    /// type variable. If no break is reachable the var stays unconstrained
    /// and defaults to Never via never-fallback.
    pub(crate) loop_break_tys: Vec<(Option<String>, TyVar)>,

    /// Metadata for a function with an opaque return type (`some P`).
    /// Set in `create_return_type` when the return annotation is `HirTy::Opaque`.
    /// Used by `build_result` to extract the concrete type for `TypedBody`.
    pub(crate) opaque_return: Option<OpaqueReturnInfo>,

    /// Deferred type-parameter defaults (e.g. `H = DefaultHasher`).
    /// Applied after constraint solving: only type vars still unconstrained
    /// get their default, so generic bodies like `Set.init()` keep `H` free.
    pub(crate) type_param_defaults: Vec<(TyVar, HirTy)>,

    /// De-dup set for TYPE-ARGUMENT conformance FAILURES, keyed by
    /// `(structural type string, protocol)`. The same wellformedness
    /// obligation (`X: Copyable`/`Static`) is emitted from several layers —
    /// the call-site where-clause (`emit_where_clause_constraints_with_subs`),
    /// the annotation formation (`lower_hir_ty_with_subs`), and the
    /// return-position formation (`lower_return_ty_with_opaque`) — so the same
    /// concrete violation can fail more than once. Report each distinct
    /// `(type, protocol)` only once; the first (earliest-solved, best-span)
    /// wins. Keyed structurally, not by TyVar, because the duplicate sites
    /// form independent TyVar trees for the same concrete type.
    pub(crate) reported_typearg_conformance: HashSet<(String, Entity)>,
}

/// Info about a promotion inserted at a Coerce site.
#[derive(Clone, Debug)]
pub struct PromotionInfo {
    /// The `FromValue.from()` method entity to call.
    pub method: Entity,
    /// Target type (what we're promoting to).
    pub target_ty: TyVar,
}

/// Metadata for a function with an opaque return type (`some P`).
/// Stored on `InferCtx` during inference of the defining body.
/// The `concrete_tv` is a fresh TyVar that the body's return expressions
/// unify with; `bounds` are the protocol constraints callers see.
#[derive(Clone, Debug)]
#[allow(dead_code)] // bounds/span used by future phases (external view, diagnostics)
pub(crate) struct OpaqueReturnInfo {
    pub concrete_tv: TyVar,
    pub bounds: Vec<(Entity, Vec<TyVar>)>,
    /// `and not Copyable` on the annotation — the underlier may be move-only,
    /// so the post-solve Copyable check on `concrete_tv` is skipped.
    pub not_copyable: bool,
    pub span: Span,
}

impl<'a> InferCtx<'a> {
    pub fn new(
        resolver: &'a dyn TypeResolver,
        query_ctx: &'a QueryContext<'a>,
        owner: Entity,
        root: Entity,
    ) -> Self {
        // Allocate a dummy TyVar(0) for the return type — will be overwritten
        let types = vec![TySlot::Unresolved { literal: None }];

        Self {
            resolver,
            query_ctx,
            types,
            constraints: Vec::new(),
            errors: Vec::new(),
            error_details: Vec::new(),
            errored_coerce_exprs: HashSet::new(),
            scrutinee_exprs: HashSet::new(),
            assign_target_exprs: HashSet::new(),
            always_decay_exprs: HashSet::new(),
            direct_callee_exprs: HashSet::new(),
            binding_init_exprs: HashSet::new(),
            pattern_binder_tvs: HashSet::new(),
            pattern_binder_gate: true,
            poison_protocol_call_recv_on_failure: HashSet::new(),
            protocol_dispatch_members: HashSet::new(),
            operator_members: HashSet::new(),
            resolutions: HashMap::new(),
            indirection_peels: HashMap::new(),
            field_subscripts: HashMap::new(),
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
            kind_flex: HashSet::new(),
            kind_coercions: HashMap::new(),
            closure_literal_exprs: HashSet::new(),
            never_fallback_targets: HashSet::new(),
            expected_array_elem: None,
            expected_dict_entry: None,
            interpolation_init_expr: None,
            interpolation_acc_tv: None,
            wildcard_tvars: HashSet::new(),
            witness_protocol_args: HashMap::new(),
            loop_break_tys: Vec::new(),
            opaque_return: None,
            type_param_defaults: Vec::new(),
            reported_typearg_conformance: HashSet::new(),
        }
    }

    /// Record protocol args for a `(container, protocol)` witness pair.
    /// Keyed by the container's canonical TyVar (after redirects) so lookups
    /// in `solve_associated` use the same canonical form.
    pub(crate) fn record_witness_args(&mut self, tv: TyVar, protocol: Entity, args: Vec<TyVar>) {
        let key = (self.resolve(tv), protocol);
        self.witness_protocol_args.insert(key, args);
    }

    /// Walk the owner's container chain to find the protocol for which `Self`
    /// refers to the implementer: a `protocol P { … }` or `extend T { … }` whose
    /// target `T` is a protocol. Returns `None` for bodies in concrete-type
    /// scopes (struct/enum methods) — `Self` there lowers to the concrete type
    /// directly, not through `HirTy::SelfType`.
    pub fn owning_self_protocol(&self) -> Option<Entity> {
        let mut current = Some(self.owner);
        while let Some(e) = current {
            match self.query_ctx.get::<kestrel_ast_builder::NodeKind>(e) {
                Some(kestrel_ast_builder::NodeKind::Protocol) => return Some(e),
                Some(kestrel_ast_builder::NodeKind::Extension) => {
                    let target = self
                        .query_ctx
                        .query(kestrel_name_res::ExtensionTargetEntity {
                            extension: e,
                            root: self.root,
                        })?;
                    if matches!(
                        self.query_ctx.get::<kestrel_ast_builder::NodeKind>(target),
                        Some(kestrel_ast_builder::NodeKind::Protocol)
                    ) {
                        return Some(target);
                    }
                    return None;
                },
                _ => current = self.query_ctx.parent_of(e),
            }
        }
        None
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

    /// Allocate a TyVar bound to a reference type (`&T` / `&mutating T`).
    pub fn ref_ty(&mut self, pointee: TyVar, mutating: bool) -> TyVar {
        let idx = self.types.len() as u32;
        self.types
            .push(TySlot::Resolved(TyKind::Ref { pointee, mutating }));
        TyVar(idx)
    }

    /// Allocate a TyVar for abstract `Self` inside `extend P` / `protocol P`.
    /// Behaves like `protocol_ty(P, vec![])` for associated-type / conformance
    /// lookups but is distinguished at output so MIR sees `MirTy::SelfType`.
    pub fn self_type_ty(&mut self, entity: Entity) -> TyVar {
        let idx = self.types.len() as u32;
        self.types
            .push(TySlot::Resolved(TyKind::SelfType { entity }));
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

    /// Allocate a TyVar directly resolved to an AssocProjection.
    /// Unlike `project_associated`, this does NOT emit an Associated constraint —
    /// use when the projection must survive as-is (e.g. cycle-breaking in
    /// `solve_associated` where re-emitting the constraint would loop).
    pub fn assoc_projection(&mut self, base: TyVar, assoc: Entity) -> TyVar {
        let idx = self.types.len() as u32;
        self.types
            .push(TySlot::Resolved(TyKind::AssocProjection { base, assoc }));
        TyVar(idx)
    }

    // === where-clause associated-type memo (G17 stage 3a) ===

    /// Record `base.assoc → tv`. The base is stored **raw**; see [`AssocSubKey`].
    pub(crate) fn push_assoc_sub(&mut self, base: Option<TyVar>, assoc: Entity, tv: TyVar) {
        self.where_clause_assoc_subs
            .push((AssocSubKey { base, assoc }, tv));
    }

    /// Base-aware memo lookup (G17 C3): an entry answers only if its recorded
    /// base and the query's base resolve to the same TyVar.
    ///
    /// `base` is the receiver the caller is projecting off — `None` only where
    /// the read site genuinely has no receiver in scope, which then resolves by
    /// unambiguity. A miss is never an error: the memo is a shortcut, so every
    /// caller falls through to its general path (`project_associated`, an
    /// `Associated` constraint, a protocol-bound search) exactly as it does for
    /// a `None` answer today.
    pub(crate) fn assoc_sub(
        &self,
        site: &'static str,
        base: Option<TyVar>,
        assoc: Entity,
    ) -> Option<TyVar> {
        self.lookup_assoc_sub(site, base, assoc, &|e| e == assoc)
    }

    /// Audit-only probe: report the strict verdict a read site *would* get for
    /// a hypothetical base, without performing a lookup. Used at R6, whose base
    /// (`recv_tv`) is identified rather than passed through, so the C2 and C3
    /// deltas can be sized separately. Inert unless the audit category is on.
    pub(crate) fn probe_assoc_sub(&self, site: &'static str, base: Option<TyVar>, assoc: Entity) {
        if !kestrel_debug::is_enabled("audit-subject") {
            return;
        }
        self.audit_assoc_sub(site, base, assoc, &|e: Entity| e == assoc);
    }

    /// The one place the memo is scanned. `matches` selects candidate entries
    /// by assoc entity; the receiver then filters them. There is no base-blind
    /// reader: the cross-protocol `Name` fallback that was one is gone (G25).
    fn lookup_assoc_sub(
        &self,
        site: &'static str,
        base: Option<TyVar>,
        assoc: Entity,
        matches: &dyn Fn(Entity) -> bool,
    ) -> Option<TyVar> {
        if kestrel_debug::is_enabled("audit-subject") {
            self.audit_assoc_sub(site, base, assoc, matches);
        }
        self.strict_assoc_sub(base, matches)
    }

    /// Candidate entries in push order — the scan every lookup shares.
    fn assoc_sub_candidates<'m>(
        &'m self,
        matches: &'m dyn Fn(Entity) -> bool,
    ) -> impl Iterator<Item = &'m (AssocSubKey, TyVar)> {
        self.where_clause_assoc_subs
            .iter()
            .filter(move |(k, _)| matches(k.assoc))
    }

    /// The `(resolved base, assoc)` answer. Both sides are resolved **here**,
    /// never at push: raw indices differ after any redirect, and union-find is
    /// monotone so resolving at lookup only ever becomes more permissive.
    fn strict_assoc_sub(
        &self,
        base: Option<TyVar>,
        matches: &dyn Fn(Entity) -> bool,
    ) -> Option<TyVar> {
        let mut cands = self.assoc_sub_candidates(matches);
        let Some(q) = base else {
            // Baseless query: resolve by unambiguity — a lone candidate is the
            // answer, two or more bail to the general path rather than picking
            // arbitrarily (the `witness_protocol_args` rule).
            let first = cands.next()?;
            return cands.next().is_none().then_some(first.1);
        };
        // A baseless *entry* matches only a baseless query; widening it to
        // "matches anything" would reintroduce the receiver confusion.
        cands
            .find(|(k, _)| k.base.is_some_and(|s| self.resolve(s) == self.resolve(q)))
            .map(|&(_, tv)| tv)
    }

    /// Log how the base-aware key differs from the base-blind one at a read
    /// site. Detection only: never influences either answer.
    fn audit_assoc_sub(
        &self,
        site: &'static str,
        base: Option<TyVar>,
        assoc: Entity,
        matches: &dyn Fn(Entity) -> bool,
    ) {
        let cands: Vec<(AssocSubKey, TyVar)> =
            self.assoc_sub_candidates(matches).copied().collect();
        let blind = cands.first().map(|&(_, tv)| tv);
        let strict = self.strict_assoc_sub(base, matches);
        // Two distinct indices that already resolve to the same canonical are
        // behaviourally the same answer, so compare resolved, not raw.
        let verdict = match (blind, strict) {
            (None, _) => SubVerdict::None_,
            (Some(b), Some(s)) if self.resolve(b) == self.resolve(s) => SubVerdict::Match,
            (Some(_), Some(_)) => SubVerdict::Mismatch,
            (Some(_), None) if base.is_none() => SubVerdict::Ambiguous,
            (Some(_), None) => SubVerdict::Miss,
        };
        let tv = |t: Option<TyVar>| t.map_or(-1i64, |t| i64::from(self.resolve(t).0));
        // `cands[0]` *is* the entry the base-blind read picked, so reporting its
        // own assoc + base says exactly which receiver the answer leaked from.
        let via = cands.first().map_or_else(
            || "-".to_string(),
            |(k, _)| format!("{}@{}", self.assoc_path(k.assoc), tv(k.base)),
        );
        kestrel_debug::ktrace!(
            "audit-subject",
            "{} site={site} assoc={} base={} blind={} strict={} cands={} via={via}",
            verdict.tag(),
            self.assoc_path(assoc),
            tv(base),
            tv(blind),
            tv(strict),
            cands.len(),
        );
    }

    /// `Owner.Name` for an associated-type entity — audit output only.
    fn assoc_path(&self, assoc: Entity) -> String {
        let name = |e: Entity| {
            self.query_ctx
                .get::<kestrel_ast_builder::Name>(e)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| "?".to_string())
        };
        match self.query_ctx.parent_of(assoc) {
            Some(p) => format!("{}.{}", name(p), name(assoc)),
            None => name(assoc),
        }
    }

    /// Allocate a TyVar bound to a Tuple type.
    pub fn tuple(&mut self, elements: Vec<TyVar>) -> TyVar {
        let idx = self.types.len() as u32;
        self.types.push(TySlot::Resolved(TyKind::Tuple(elements)));
        TyVar(idx)
    }

    /// Allocate a TyVar for a *bare* callable value — a named `Def` or an
    /// enum-case constructor. Every param defaults to `Consuming` (the pre-#106
    /// convention) and the kind is built at `Normal`, but the fresh TyVar is
    /// also registered in [`Self::kind_flex`]: a capture-free function pointer
    /// "has no environment, satisfies every kind, and escapes freely"
    /// (closures.md), so it ADOPTS whatever kind it is checked against rather
    /// than carrying a wildcard kind through unification (plan D1 — a wildcard
    /// kind poisons merge points). Each use site instantiates a fresh TyVar, so
    /// in-place adoption never retroactively re-kinds another site.
    ///
    /// Use [`Self::function_conv`] for types whose kind and conventions are
    /// fixed by an annotation or a closure literal.
    pub fn function(&mut self, params: Vec<TyVar>, ret: TyVar) -> TyVar {
        let conventions = vec![kestrel_ast::ParamConvention::Consuming; params.len()];
        let tv = self.function_conv(kestrel_ast::FnTypeKind::Normal, params, conventions, ret);
        self.kind_flex.insert(tv);
        tv
    }

    /// Allocate a TyVar bound to a Function type with an explicit closure
    /// `kind` and explicit per-param conventions (parallel to `params`).
    pub fn function_conv(
        &mut self,
        kind: kestrel_ast::FnTypeKind,
        params: Vec<TyVar>,
        conventions: Vec<kestrel_ast::ParamConvention>,
        ret: TyVar,
    ) -> TyVar {
        let idx = self.types.len() as u32;
        self.types.push(TySlot::Resolved(TyKind::Function {
            kind,
            params,
            conventions,
            ret,
        }));
        TyVar(idx)
    }

    /// Overwrite the resolved `TyKind::Function` *kind* of `tv` in place — the
    /// closure-literal retrofit (`{ … }` is always built at `Normal`; the
    /// expected type selects the real kind) and the `kind_flex` adoption.
    /// No-op if `tv` does not resolve to a function type (mirrors
    /// [`Self::set_function_conventions`]).
    pub fn set_function_kind(&mut self, tv: TyVar, kind: kestrel_ast::FnTypeKind) {
        let root = self.resolve(tv);
        if let TySlot::Resolved(TyKind::Function { kind: k, .. }) = &mut self.types[root.0 as usize]
        {
            *k = kind;
        }
    }

    /// Overwrite the resolved `TyKind::Function` conventions of `tv` in place.
    /// Used by `solve_call` to upgrade a closure literal's inferred param
    /// convention (e.g. `Consuming` → `MutBorrow`) once the expected parameter
    /// type is known. No-op if `tv` does not resolve to a function type.
    pub fn set_function_conventions(
        &mut self,
        tv: TyVar,
        conventions: Vec<kestrel_ast::ParamConvention>,
    ) {
        let root = self.resolve(tv);
        if let TySlot::Resolved(TyKind::Function { conventions: c, .. }) =
            &mut self.types[root.0 as usize]
        {
            *c = conventions;
        }
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

    /// Mark `tv` as a wildcard (created from explicit `_` in a type-arg position).
    /// Wildcard TyVars that stay Unresolved don't generate "could not infer type"
    /// diagnostics. Call after creating the TyVar; propagation to unified vars
    /// is handled in unify::unify.
    pub fn mark_wildcard(&mut self, tv: TyVar) {
        self.wildcard_tvars.insert(tv);
    }

    /// Returns true if `tv`'s resolved root is marked as a wildcard.
    pub fn is_wildcard(&self, tv: TyVar) -> bool {
        self.wildcard_tvars.contains(&self.resolve(tv))
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

    pub fn borrow_pointee(&mut self, inner: TyVar, pointee: TyVar, span: Span) {
        self.constraints.push(Constraint::BorrowPointee {
            inner,
            pointee,
            span,
        });
    }

    /// Equal with the ref-decay dimension — see `Constraint::EqualDecayed`.
    pub fn equal_decayed(&mut self, value: TyVar, target: TyVar, span: Span) {
        self.constraints.push(Constraint::EqualDecayed {
            value,
            target,
            span,
        });
    }

    /// Assignment into a local target — see `Constraint::AssignTarget`.
    pub fn assign_target(&mut self, value: TyVar, target: TyVar, expr: HirExprId, span: Span) {
        self.constraints.push(Constraint::AssignTarget {
            value,
            target,
            expr,
            span,
        });
    }

    pub fn conforms(&mut self, ty: TyVar, protocol: Entity, span: Span) {
        self.constraints.push(Constraint::Conforms {
            ty,
            protocol,
            span,
            poison_ty_on_failure: false,
            origin: crate::constraint::ConformsOrigin::Expr,
        });
    }

    /// Conforms variant for TYPE-ARGUMENT obligations (where-clause bounds,
    /// formation wellformedness, alias bounds): a ref judged here is the
    /// ref ITSELF, never its pointee — see `ConformsOrigin`.
    pub fn conforms_typearg(&mut self, ty: TyVar, protocol: Entity, span: Span) {
        self.constraints.push(Constraint::Conforms {
            ty,
            protocol,
            span,
            poison_ty_on_failure: false,
            origin: crate::constraint::ConformsOrigin::TypeArg,
        });
    }

    /// Conforms variant that poisons `ty` on failure. Used by Sugar's
    /// primary-constraint emission so cascading Member/ImplicitMember errors
    /// inside the desugared subtree absorb silently when the receiver type
    /// doesn't conform to the expected protocol.
    pub fn conforms_poisoning(&mut self, ty: TyVar, protocol: Entity, span: Span) {
        self.constraints.push(Constraint::Conforms {
            ty,
            protocol,
            span,
            poison_ty_on_failure: true,
            origin: crate::constraint::ConformsOrigin::Expr,
        });
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
    /// (e.g., `Counter.method()` or `T.method()`). Optionally carries
    /// explicit type args for cases like `Pointer[UInt8].nullPointer()`.
    pub fn member_static(
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
            is_static_context: true,
            explicit_type_args,
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

    pub fn interpolation_link(&mut self, result_tv: TyVar, acc_tv: TyVar, span: Span) {
        self.constraints.push(Constraint::InterpolationLink {
            result_tv,
            acc_tv,
            span,
        });
    }
}
