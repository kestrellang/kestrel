//! Shared semantic queries for conformance and copy semantics.
//!
//! This crate is intentionally query-only. It centralizes facts that several
//! downstream phases need so analyzers, type inference, move tracking, and MIR
//! lowering do not each reinterpret raw conformance syntax differently.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use kestrel_ast::AstType;
use kestrel_copy_fold::{CopyLayer, fold_members, instance_semantics};
use kestrel_ast_builder::{
    Computed, ConformanceItem, Conformances, NodeKind, WhereClause as AstWhereClause,
    WhereConstraint,
};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::builtin::BuiltinKind;
use kestrel_hir::{Builtin, HirTy};
use kestrel_hir_lower::{LowerCallableTypes, LowerExtensionTargetTypeArgs, LowerTypeAnnotation};
use kestrel_name_res::{
    ConformingProtocolInstantiations, ConformingProtocols, EntityBuiltin, ResolveBuiltin,
    ResolveTypePath, TypeResolution,
};
use kestrel_span::Span;

// ===== Direct Conformance Facts =====

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConformancePolarity {
    Positive,
    Negative,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResolvedConformanceTarget {
    Protocol(Entity),
    NonProtocol(Entity),
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResolvedConformance {
    pub polarity: ConformancePolarity,
    pub target: ResolvedConformanceTarget,
    pub ast_ty: AstType,
    pub span: Span,
}

impl ResolvedConformance {
    pub fn protocol(&self) -> Option<Entity> {
        match self.target {
            ResolvedConformanceTarget::Protocol(entity) => Some(entity),
            _ => None,
        }
    }

    pub fn type_name(&self) -> String {
        ast_type_name(&self.ast_ty)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct ResolvedConformanceSet {
    pub items: Vec<ResolvedConformance>,
}

impl ResolvedConformanceSet {
    pub fn positives(&self) -> impl Iterator<Item = &ResolvedConformance> {
        self.items
            .iter()
            .filter(|item| item.polarity == ConformancePolarity::Positive)
    }

    pub fn negatives(&self) -> impl Iterator<Item = &ResolvedConformance> {
        self.items
            .iter()
            .filter(|item| item.polarity == ConformancePolarity::Negative)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ResolvedConformances {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for ResolvedConformances {
    type Output = ResolvedConformanceSet;

    fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output {
        let Some(conformances) = ctx.get::<Conformances>(self.entity) else {
            return ResolvedConformanceSet::default();
        };

        let mut items = Vec::new();
        for item in &conformances.0 {
            let (polarity, ast_ty) = match item {
                ConformanceItem::Positive(ast_ty, _) => {
                    (ConformancePolarity::Positive, ast_ty.clone())
                },
                ConformanceItem::Negative(ast_ty, _) => {
                    (ConformancePolarity::Negative, ast_ty.clone())
                },
            };
            let span = ast_type_span(&ast_ty);
            let target = resolve_protocol_target(ctx, &ast_ty, self.entity, self.root);
            items.push(ResolvedConformance {
                polarity,
                target,
                ast_ty,
                span,
            });
        }

        ResolvedConformanceSet { items }
    }

    fn describe(&self) -> String {
        format!("ResolvedConformances({:?})", self.entity)
    }
}

// ===== Protocol Refinement =====

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ProtocolRefines {
    pub protocol: Entity,
    pub base: Entity,
    pub root: Entity,
}

impl QueryFn for ProtocolRefines {
    type Output = bool;

    fn execute(&self, ctx: &QueryContext<'_>) -> bool {
        if self.protocol == self.base {
            return true;
        }
        ctx.query(ConformingProtocols {
            entity: self.protocol,
            root: self.root,
        })
        .contains(&self.base)
    }

    fn describe(&self) -> String {
        format!("ProtocolRefines({:?}, base={:?})", self.protocol, self.base)
    }
}

/// Whether `: not P` is permitted on `protocol` — true only for builtin
/// language-feature protocols with implicit conformance (e.g. Copyable).
/// A plain function, not a query: the body is one `EntityBuiltin` lookup,
/// cheaper to recompute than a memo slot.
pub fn protocol_allows_negative_conformance(ctx: &QueryContext<'_>, protocol: Entity) -> bool {
    let Some(builtin) = ctx.query(EntityBuiltin { entity: protocol }) else {
        return false;
    };

    matches!(
        builtin.kind(),
        BuiltinKind::Protocol {
            implicit_conformance: true,
            ..
        }
    )
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExplicitlyNegatesProtocol {
    pub entity: Entity,
    pub protocol: Entity,
    pub root: Entity,
}

impl QueryFn for ExplicitlyNegatesProtocol {
    type Output = bool;

    fn execute(&self, ctx: &QueryContext<'_>) -> bool {
        ctx.query(ResolvedConformances {
            entity: self.entity,
            root: self.root,
        })
        .negatives()
        .any(|item| item.protocol() == Some(self.protocol))
    }

    fn describe(&self) -> String {
        format!(
            "ExplicitlyNegatesProtocol({:?}, {:?})",
            self.entity, self.protocol
        )
    }
}

/// Does `entity` declare conformance to `protocol` by any route — direct
/// declaration, `extend entity: protocol`, or protocol refinement (membership
/// in `ConformingProtocols`)? "Declares" as opposed to *satisfies*: conditional
/// `where`-gated conformance is the bound-aware `type_satisfies` check.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct DeclaresConformanceTo {
    pub entity: Entity,
    pub protocol: Entity,
    pub root: Entity,
}

impl QueryFn for DeclaresConformanceTo {
    type Output = bool;

    fn execute(&self, ctx: &QueryContext<'_>) -> bool {
        ctx.query(ConformingProtocols {
            entity: self.entity,
            root: self.root,
        })
        .contains(&self.protocol)
    }

    fn describe(&self) -> String {
        format!(
            "DeclaresConformanceTo({:?}, {:?})",
            self.entity, self.protocol
        )
    }
}

// ===== Copy Semantics =====

// The tri-state vocabulary and the shared decision tree live in the leaf
// crate kestrel-copy-fold; re-exported here so all existing importers
// (solver, resolve, where_clauses, mir-lower, analyze) keep their
// `kestrel_semantics::` paths. Only kestrel-mir imports kestrel-copy-fold
// directly (it has no dependency path to this crate).
pub use kestrel_copy_fold::{CopyRequirement, CopySemantics};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CopySemanticsReason {
    Default,
    ExplicitNotCopyable,
    NonCopyableChild(Entity),
    ExplicitCloneable,
    CloneableChildRequiresConformance(Entity),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CopySemanticsInfo {
    pub semantics: CopySemantics,
    pub reason: CopySemanticsReason,
}

impl CopySemanticsInfo {
    pub fn copyable() -> Self {
        Self {
            semantics: CopySemantics::Copyable,
            reason: CopySemanticsReason::Default,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypeParamCopyRequirement {
    pub param: Entity,
    pub context: Entity,
    pub root: Entity,
}

impl QueryFn for TypeParamCopyRequirement {
    type Output = CopyRequirement;

    fn execute(&self, ctx: &QueryContext<'_>) -> CopyRequirement {
        let copyable = ctx.query(ResolveBuiltin {
            builtin: Builtin::Copyable,
            root: self.root,
        });
        let cloneable = ctx.query(ResolveBuiltin {
            builtin: Builtin::Cloneable,
            root: self.root,
        });

        let mut checked = HashSet::new();
        let mut contexts = Vec::new();
        if let Some(parent) = ctx.parent_of(self.param) {
            contexts.push(parent);
        }
        let mut current = Some(self.context);
        while let Some(entity) = current {
            contexts.push(entity);
            current = ctx.parent_of(entity);
        }

        let mut requires_cloneable = false;
        for context in contexts {
            if !checked.insert(context) {
                continue;
            }
            let Some(wc) = ctx.get::<AstWhereClause>(context) else {
                continue;
            };
            for constraint in &wc.0 {
                match constraint {
                    WhereConstraint::NegativeBound {
                        subject, protocol, ..
                    } => {
                        if resolve_type_entity(ctx, subject, context, self.root) != Some(self.param)
                        {
                            continue;
                        }
                        if let Some(copyable) = copyable
                            && resolve_type_entity(ctx, protocol, context, self.root)
                                == Some(copyable)
                        {
                            return CopyRequirement::MayBeNonCopyable;
                        }
                    },
                    WhereConstraint::Bound {
                        subject, protocols, ..
                    } => {
                        if resolve_type_entity(ctx, subject, context, self.root) != Some(self.param)
                        {
                            continue;
                        }
                        if let Some(cloneable) = cloneable {
                            for protocol_ty in protocols {
                                let Some(protocol) =
                                    resolve_type_entity(ctx, protocol_ty, context, self.root)
                                else {
                                    continue;
                                };
                                if ctx.query(ProtocolRefines {
                                    protocol,
                                    base: cloneable,
                                    root: self.root,
                                }) {
                                    requires_cloneable = true;
                                }
                            }
                        }
                    },
                    WhereConstraint::Equality { .. } => {},
                }
            }
        }

        if requires_cloneable {
            CopyRequirement::RequiresCloneable
        } else {
            CopyRequirement::RequiresCopyable
        }
    }

    fn describe(&self) -> String {
        format!("TypeParamCopyRequirement({:?})", self.param)
    }
}

/// The type-param positions that gate a `not Copyable` type's *conditional*
/// Copyable conformance. `struct X[A, B]: not Copyable` + `extend X[A, B]:
/// Copyable where A: Copyable` returns `[0]`. Empty when the type isn't
/// conditionally copyable — i.e. it's unconditionally Copyable/Cloneable, or
/// `not Copyable` with no `extend …: Copyable`. Used to compute
/// per-instantiation copyability: `X[args]` is Copyable iff every gating
/// `args[i]` is itself Copyable. Invariant relied on by the MIR mono layer:
/// non-empty ONLY when the base is `NotCopyable` (checked first below). The
/// fold over these positions is `kestrel_copy_fold::instance_semantics` — the
/// single source of truth for every layer (semantics, solver, analyze, MIR).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ConditionalCopyableParams {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for ConditionalCopyableParams {
    type Output = Vec<usize>;

    fn describe(&self) -> String {
        format!("ConditionalCopyableParams({:?})", self.entity)
    }

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<usize> {
        // Only a `not Copyable`-base type can be *conditionally* Copyable.
        if ctx
            .query(NominalCopySemantics {
                entity: self.entity,
                root: self.root,
            })
            .semantics
            != CopySemantics::NotCopyable
        {
            return Vec::new();
        }
        let Some(copyable) = ctx.query(ResolveBuiltin {
            builtin: Builtin::Copyable,
            root: self.root,
        }) else {
            return Vec::new();
        };

        // Find an extension adding a Copyable(-refining) conformance.
        let insts = ctx.query(ConformingProtocolInstantiations {
            entity: self.entity,
            root: self.root,
        });
        // Memoize per-protocol refinement: both the conformance scan and the
        // where-clause scan below can probe the same protocol; RefCell keeps
        // the closure `Fn` so it can be shared by reference between them.
        let refines_cache: RefCell<HashMap<Entity, bool>> = RefCell::new(HashMap::new());
        let refines_copyable = |proto: Entity| {
            if proto == copyable {
                return true;
            }
            if let Some(&hit) = refines_cache.borrow().get(&proto) {
                return hit;
            }
            let refines = ctx.query(ProtocolRefines {
                protocol: proto,
                base: copyable,
                root: self.root,
            });
            refines_cache.borrow_mut().insert(proto, refines);
            refines
        };
        let Some(ext) = insts
            .iter()
            .find(|(proto, source, _)| *source != self.entity && refines_copyable(*proto))
            .map(|(_, source, _)| *source)
        else {
            return Vec::new();
        };

        // Map the extension's Copyable where-clause bounds to target-arg
        // positions: `extend X[A, B]` lowers its target to `X[Param(A), Param(B)]`,
        // so a bound `A: Copyable` gates position 0.
        let target_args = ctx
            .query(LowerExtensionTargetTypeArgs {
                extension: ext,
                root: self.root,
            })
            .unwrap_or_default();
        let Some(wc) = ctx.get::<AstWhereClause>(ext) else {
            return Vec::new();
        };
        let mut positions = Vec::new();
        for constraint in &wc.0 {
            let WhereConstraint::Bound {
                subject, protocols, ..
            } = constraint
            else {
                continue;
            };
            let gates_copyable = protocols.iter().any(|p| {
                resolve_type_entity(ctx, p, ext, self.root).is_some_and(&refines_copyable)
            });
            if !gates_copyable {
                continue;
            }
            let Some(param) = resolve_type_entity(ctx, subject, ext, self.root) else {
                continue;
            };
            if let Some(idx) = target_args
                .iter()
                .position(|t| matches!(t, HirTy::Param(p, _) if *p == param))
                && !positions.contains(&idx)
            {
                positions.push(idx);
            }
        }
        positions
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NominalCopySemantics {
    pub entity: Entity,
    pub root: Entity,
}

// WARNING: side-channel state invisible to the query framework's dependency
// tracker — memoized results that consulted it are not invalidated when it
// changes. It exists only because the framework panics on re-entrant queries
// (recursive types). Any future recursive query needs the same treatment, or
// a framework-level cycle-recovery mechanism.
thread_local! {
    static COMPUTING_COPY_SEMANTICS: RefCell<Vec<(Entity, Entity)>> = const { RefCell::new(Vec::new()) };
}

impl QueryFn for NominalCopySemantics {
    type Output = CopySemanticsInfo;

    fn execute(&self, ctx: &QueryContext<'_>) -> CopySemanticsInfo {
        // The query framework panics on re-entry for the same key, so we
        // can't rely on a thread_local check *inside* execute — by the
        // time we'd see the cycle, the framework has already panicked.
        // Callers guard against re-entry by consulting `computing_contains`
        // before invoking the query (see `hir_type_copy_semantics`).
        let key = (self.entity, self.root);
        COMPUTING_COPY_SEMANTICS.with(|stack| stack.borrow_mut().push(key));
        let result = nominal_copy_semantics_impl(ctx, self.entity, self.root);
        COMPUTING_COPY_SEMANTICS.with(|stack| {
            stack.borrow_mut().retain(|entry| *entry != key);
        });
        result
    }

    fn describe(&self) -> String {
        format!("NominalCopySemantics({:?})", self.entity)
    }
}

fn computing_contains(entity: Entity, root: Entity) -> bool {
    COMPUTING_COPY_SEMANTICS.with(|stack| stack.borrow().contains(&(entity, root)))
}

fn query_nominal_semantics(ctx: &QueryContext<'_>, entity: Entity, root: Entity) -> CopySemantics {
    if computing_contains(entity, root) {
        // Self-reference (direct or transitive). Fall back to Copyable —
        // the definition can't make itself non-copyable without another
        // child already doing so, which is handled on its own branch.
        CopySemantics::Copyable
    } else {
        ctx.query(NominalCopySemantics { entity, root }).semantics
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct IsBuiltinProtocol {
    pub protocol: Entity,
    pub builtin: Builtin,
    pub root: Entity,
}

impl QueryFn for IsBuiltinProtocol {
    type Output = bool;

    fn execute(&self, ctx: &QueryContext<'_>) -> bool {
        ctx.query(ResolveBuiltin {
            builtin: self.builtin,
            root: self.root,
        }) == Some(self.protocol)
    }

    fn describe(&self) -> String {
        format!("IsBuiltinProtocol({:?}, {:?})", self.protocol, self.builtin)
    }
}

/// `CopyLayer` over `HirTy` — the semantics layer's hooks into the shared
/// decision tree (`kestrel_copy_fold::instance_semantics`, the single source
/// of truth for per-instantiation copy semantics across semantics / solver /
/// analyze / MIR). Layer-specific plumbing: the `NominalCopySemantics`
/// re-entrancy guard in `base_semantics` and the caller-supplied `context`
/// scoping type-param bound lookups.
struct HirCopyLayer<'a, 'q> {
    ctx: &'a QueryContext<'q>,
    context: Entity,
    root: Entity,
}

impl CopyLayer for HirCopyLayer<'_, '_> {
    type Ty = HirTy;
    type Sem = CopySemantics;

    fn base_semantics(&self, entity: Entity) -> CopySemantics {
        // Keeps the COMPUTING_COPY_SEMANTICS cycle guard — layer-1-only plumbing.
        query_nominal_semantics(self.ctx, entity, self.root)
    }

    fn gating_positions(&self, entity: Entity) -> Cow<'_, [usize]> {
        Cow::Owned(self.ctx.query(ConditionalCopyableParams {
            entity,
            root: self.root,
        }))
    }

    fn sem_from_class(&self, _: Entity, class: CopySemantics) -> CopySemantics {
        class
    }

    fn member_semantics(&self, ty: &HirTy) -> CopySemantics {
        match ty {
            HirTy::Struct { entity, args, .. } | HirTy::Enum { entity, args, .. } => {
                instance_semantics(self, *entity, args)
            },
            HirTy::Tuple(elems, _) => fold_members(elems.iter().map(|e| self.member_semantics(e))),
            // HOOK: caller-supplied `context` scopes the bound lookup.
            HirTy::Param(entity, _) => self
                .ctx
                .query(TypeParamCopyRequirement {
                    param: *entity,
                    context: self.context,
                    root: self.root,
                })
                .into(),
            // No per-instantiation refinement for Self/alias uses (current behavior).
            HirTy::SelfType(entity, _) => self.base_semantics(*entity),
            HirTy::AliasUse { entity, .. } => self.base_semantics(*entity),
            HirTy::Protocol { .. } | HirTy::Opaque { .. } => CopySemantics::Copyable,
            // Ref is rejected (rewritten to Error) at HIR lowering and should
            // never reach here; treat it exactly like Error if it does.
            // Infer/Error are recovery.
            HirTy::Function { .. }
            | HirTy::Never(_)
            | HirTy::Infer(_)
            | HirTy::Error(_)
            | HirTy::Ref { .. } => CopySemantics::Copyable,
            // An associated projection (`I.Item`) is Copyable-by-default, exactly
            // like a type param: the model gives every associated type an implicit
            // `Copyable` bound unless it's declared `: not Copyable`, and only
            // Copyable concretes are substituted at mono. Treating it as
            // `NotCopyable` here wrongly poisoned every container of an assoc type
            // (e.g. an iterator's `pendingItem: I.Item?`). MIR `ty_query` already
            // classifies `AssociatedProjection` as `Bitwise`; the solver agrees via
            // `type_conforms_copyable`. (A `type Item: not Copyable` associated type
            // would need per-assoc requirement tracking — not modeled yet.)
            HirTy::AssocProjection { .. } => CopySemantics::Copyable,
        }
    }
}

pub fn hir_type_copy_semantics(
    ctx: &QueryContext<'_>,
    ty: &HirTy,
    context: Entity,
    root: Entity,
) -> CopySemantics {
    HirCopyLayer { ctx, context, root }.member_semantics(ty)
}

fn nominal_copy_semantics_impl(
    ctx: &QueryContext<'_>,
    entity: Entity,
    root: Entity,
) -> CopySemanticsInfo {
    let copyable = ctx.query(ResolveBuiltin {
        builtin: Builtin::Copyable,
        root,
    });
    let cloneable = ctx.query(ResolveBuiltin {
        builtin: Builtin::Cloneable,
        root,
    });

    if let Some(copyable) = copyable
        && ctx.query(ExplicitlyNegatesProtocol {
            entity,
            protocol: copyable,
            root,
        })
    {
        return CopySemanticsInfo {
            semantics: CopySemantics::NotCopyable,
            reason: CopySemanticsReason::ExplicitNotCopyable,
        };
    }

    // Fallback: `: not Copyable` is meaningful syntactically even when the
    // protocol path did not resolve to the builtin (for example, fixtures or
    // modules that did not import std.core.Copyable). Match the last segment by
    // name so the semantic copy classifier still agrees with the parser-level
    // negative conformance. This string match exists ONLY for stdlib-less test
    // fixtures where the builtin entity isn't registered; the ResolveBuiltin
    // entity path above is the source of truth — don't extend this pattern.
    if let Some(conf) = ctx.get::<Conformances>(entity)
        && conf.0.iter().any(|item| {
            matches!(
                item,
                ConformanceItem::Negative(ast_ty, _) if ast_type_last_segment_is(ast_ty, "Copyable")
            )
        })
    {
        return CopySemanticsInfo {
            semantics: CopySemantics::NotCopyable,
            reason: CopySemanticsReason::ExplicitNotCopyable,
        };
    }

    let child_types = collect_child_types(ctx, entity, root);
    for (child, ty) in &child_types {
        if hir_type_copy_semantics(ctx, ty, entity, root) == CopySemantics::NotCopyable {
            return CopySemanticsInfo {
                semantics: CopySemantics::NotCopyable,
                reason: CopySemanticsReason::NonCopyableChild(*child),
            };
        }
    }

    if let Some(cloneable) = cloneable
        && ctx.query(DeclaresConformanceTo {
            entity,
            protocol: cloneable,
            root,
        })
    {
        return CopySemanticsInfo {
            semantics: CopySemantics::Cloneable,
            reason: CopySemanticsReason::ExplicitCloneable,
        };
    }

    CopySemanticsInfo::copyable()
}

/// `Copyable`-name fallback for stdlib-less fixtures. Returns true when
/// `ast_ty` is a single-segment named type whose last segment matches
/// `name`. Matches the previous HIR-tracker behavior so tests like
/// `struct Handle: not Copyable {}` (without `import std.core`) still
/// pick up their non-copyable semantics.
fn ast_type_last_segment_is(ast_ty: &AstType, name: &str) -> bool {
    let AstType::Named { segments, .. } = ast_ty else {
        return false;
    };
    segments.last().is_some_and(|s| s.name == name)
}

fn collect_child_types(
    ctx: &QueryContext<'_>,
    entity: Entity,
    root: Entity,
) -> Vec<(Entity, HirTy)> {
    let mut out = Vec::new();
    for &child in ctx.children_of(entity) {
        match ctx.get::<NodeKind>(child) {
            Some(NodeKind::Field) => {
                // Computed properties (`var x: T { get … }`) store nothing —
                // they read/write through accessors — so they never affect
                // whether the containing type is bit-copyable. Only stored
                // fields contribute to copy semantics.
                if ctx.get::<Computed>(child).is_some() {
                    continue;
                }
                if let Some(ty) = ctx.query(LowerTypeAnnotation {
                    entity: child,
                    root,
                }) {
                    out.push((child, ty));
                }
            },
            Some(NodeKind::EnumCase) => {
                // Enum case payloads live in the Callable component's params,
                // not as child Field entities. Lower them via LowerCallableTypes.
                if let Some(tys) = ctx.query(LowerCallableTypes {
                    entity: child,
                    root,
                }) {
                    for ty in tys.into_iter().flatten() {
                        out.push((child, ty));
                    }
                }
            },
            _ => {},
        }
    }
    out
}

fn resolve_protocol_target(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    context: Entity,
    root: Entity,
) -> ResolvedConformanceTarget {
    let Some(entity) = resolve_type_entity(ctx, ast_ty, context, root) else {
        return ResolvedConformanceTarget::Unresolved;
    };
    if ctx.get::<NodeKind>(entity) == Some(&NodeKind::Protocol) {
        ResolvedConformanceTarget::Protocol(entity)
    } else {
        ResolvedConformanceTarget::NonProtocol(entity)
    }
}

fn resolve_type_entity(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    context: Entity,
    root: Entity,
) -> Option<Entity> {
    let AstType::Named { segments, .. } = ast_ty else {
        return None;
    };
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    match ctx.query(ResolveTypePath {
        segments: seg_names,
        context,
        root,
    }) {
        TypeResolution::Found(entity) | TypeResolution::NotAType(entity) => Some(entity),
        TypeResolution::SelfType | TypeResolution::NotFound(_) => None,
    }
}

fn ast_type_span(ast_ty: &AstType) -> Span {
    match ast_ty {
        AstType::Named { span, .. }
        | AstType::Tuple(_, span)
        | AstType::Function { span, .. }
        | AstType::Array(_, span)
        | AstType::Dictionary(_, _, span)
        | AstType::Optional(_, span)
        | AstType::Result { span, .. }
        | AstType::Unit(span)
        | AstType::Never(span)
        | AstType::Inferred(span)
        | AstType::Some { span, .. }
        | AstType::Ref { span, .. } => span.clone(),
    }
}

fn ast_type_name(ast_ty: &AstType) -> String {
    match ast_ty {
        AstType::Named { segments, .. } => segments
            .iter()
            .map(|seg| seg.name.as_str())
            .collect::<Vec<_>>()
            .join("."),
        AstType::Tuple(_, _) => "tuple".into(),
        AstType::Function { .. } => "function".into(),
        AstType::Array(_, _) => "array".into(),
        AstType::Dictionary(_, _, _) => "dictionary".into(),
        AstType::Optional(_, _) => "optional".into(),
        AstType::Result { .. } => "result".into(),
        AstType::Unit(_) => "()".into(),
        AstType::Never(_) => "Never".into(),
        AstType::Inferred(_) => "_".into(),
        AstType::Some { .. } => "some".into(),
        AstType::Ref { .. } => "reference".into(),
    }
}
