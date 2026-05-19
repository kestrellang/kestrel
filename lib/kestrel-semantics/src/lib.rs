//! Shared semantic queries for conformance and copy semantics.
//!
//! This crate is intentionally query-only. It centralizes facts that several
//! downstream phases need so analyzers, type inference, move tracking, and MIR
//! lowering do not each reinterpret raw conformance syntax differently.

use std::cell::RefCell;
use std::collections::HashSet;

use kestrel_ast::AstType;
use kestrel_ast_builder::{
    ConformanceItem, Conformances, NodeKind, WhereClause as AstWhereClause, WhereConstraint,
};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::builtin::BuiltinKind;
use kestrel_hir::{Builtin, HirTy};
use kestrel_hir_lower::{LowerCallableTypes, LowerTypeAnnotation};
use kestrel_name_res::{
    ConformingProtocols, EntityBuiltin, ResolveBuiltin, ResolveTypePath, TypeResolution,
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

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ProtocolAllowsNegativeConformance {
    pub protocol: Entity,
}

impl QueryFn for ProtocolAllowsNegativeConformance {
    type Output = bool;

    fn execute(&self, ctx: &QueryContext<'_>) -> bool {
        let Some(builtin) = ctx.query(EntityBuiltin {
            entity: self.protocol,
        }) else {
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

    fn describe(&self) -> String {
        format!("ProtocolAllowsNegativeConformance({:?})", self.protocol)
    }
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

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExplicitlyConformsToProtocol {
    pub entity: Entity,
    pub protocol: Entity,
    pub root: Entity,
}

impl QueryFn for ExplicitlyConformsToProtocol {
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
            "ExplicitlyConformsToProtocol({:?}, {:?})",
            self.entity, self.protocol
        )
    }
}

// ===== Copy Semantics =====

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CopySemantics {
    Copyable,
    Cloneable,
    NotCopyable,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CopyRequirement {
    RequiresCopyable,
    RequiresCloneable,
    MayBeNonCopyable,
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

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NominalCopySemantics {
    pub entity: Entity,
    pub root: Entity,
}

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
pub struct NominalTypeConformsToProtocol {
    pub entity: Entity,
    pub protocol: Entity,
    pub root: Entity,
}

impl QueryFn for NominalTypeConformsToProtocol {
    type Output = bool;

    fn execute(&self, ctx: &QueryContext<'_>) -> bool {
        if ctx.query(IsBuiltinProtocol {
            protocol: self.protocol,
            builtin: Builtin::Copyable,
            root: self.root,
        }) {
            return query_nominal_semantics(ctx, self.entity, self.root)
                != CopySemantics::NotCopyable;
        }

        if ctx.query(IsBuiltinProtocol {
            protocol: self.protocol,
            builtin: Builtin::Cloneable,
            root: self.root,
        }) {
            return query_nominal_semantics(ctx, self.entity, self.root)
                == CopySemantics::Cloneable;
        }

        ctx.query(ExplicitlyConformsToProtocol {
            entity: self.entity,
            protocol: self.protocol,
            root: self.root,
        })
    }

    fn describe(&self) -> String {
        format!(
            "NominalTypeConformsToProtocol({:?}, {:?})",
            self.entity, self.protocol
        )
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

pub fn hir_type_copy_semantics(
    ctx: &QueryContext<'_>,
    ty: &HirTy,
    context: Entity,
    root: Entity,
) -> CopySemantics {
    match ty {
        HirTy::Struct { entity, .. } | HirTy::Enum { entity, .. } => {
            query_nominal_semantics(ctx, *entity, root)
        },
        HirTy::Protocol { .. } | HirTy::Opaque { .. } => CopySemantics::Copyable,
        HirTy::Tuple(elems, _) => {
            let mut saw_cloneable = false;
            for elem in elems {
                match hir_type_copy_semantics(ctx, elem, context, root) {
                    CopySemantics::NotCopyable => return CopySemantics::NotCopyable,
                    CopySemantics::Cloneable => saw_cloneable = true,
                    CopySemantics::Copyable => {},
                }
            }
            if saw_cloneable {
                CopySemantics::Cloneable
            } else {
                CopySemantics::Copyable
            }
        },
        HirTy::Function { .. } | HirTy::Never(_) | HirTy::Infer(_) | HirTy::Error(_) => {
            CopySemantics::Copyable
        },
        HirTy::AliasUse { entity, .. } => query_nominal_semantics(ctx, *entity, root),
        HirTy::Param(entity, _) => {
            match ctx.query(TypeParamCopyRequirement {
                param: *entity,
                context,
                root,
            }) {
                CopyRequirement::RequiresCloneable => CopySemantics::Cloneable,
                CopyRequirement::RequiresCopyable => CopySemantics::Copyable,
                CopyRequirement::MayBeNonCopyable => CopySemantics::NotCopyable,
            }
        },
        HirTy::SelfType(entity, _) => query_nominal_semantics(ctx, *entity, root),
        HirTy::AssocProjection { .. } => CopySemantics::NotCopyable,
    }
}

pub fn hir_type_conforms_to_protocol(
    ctx: &QueryContext<'_>,
    ty: &HirTy,
    protocol: Entity,
    context: Entity,
    root: Entity,
) -> bool {
    if ctx.query(IsBuiltinProtocol {
        protocol,
        builtin: Builtin::Copyable,
        root,
    }) {
        return hir_type_copy_semantics(ctx, ty, context, root) != CopySemantics::NotCopyable;
    }

    if ctx.query(IsBuiltinProtocol {
        protocol,
        builtin: Builtin::Cloneable,
        root,
    }) {
        return hir_type_copy_semantics(ctx, ty, context, root) == CopySemantics::Cloneable;
    }

    match ty {
        HirTy::Struct { entity, .. }
        | HirTy::Enum { entity, .. }
        | HirTy::Protocol { entity, .. }
        | HirTy::AliasUse { entity, .. }
        | HirTy::SelfType(entity, _) => ctx.query(ExplicitlyConformsToProtocol {
            entity: *entity,
            protocol,
            root,
        }),
        // Opaque types conform if any of their bounds conform
        HirTy::Opaque { bounds, .. } => bounds
            .iter()
            .any(|b| hir_type_conforms_to_protocol(ctx, b, protocol, context, root)),
        HirTy::Param(entity, _) => {
            let Some(parent) = ctx.parent_of(*entity) else {
                return false;
            };
            type_param_has_bound(ctx, *entity, protocol, parent, root)
                || type_param_has_bound(ctx, *entity, protocol, context, root)
        },
        HirTy::Tuple(_, _)
        | HirTy::Function { .. }
        | HirTy::Never(_)
        | HirTy::Infer(_)
        | HirTy::AssocProjection { .. }
        | HirTy::Error(_) => false,
    }
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
    // negative conformance.
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
        && ctx.query(ExplicitlyConformsToProtocol {
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

    for (child, ty) in &child_types {
        if hir_type_copy_semantics(ctx, ty, entity, root) == CopySemantics::Cloneable {
            return CopySemanticsInfo {
                semantics: CopySemantics::NotCopyable,
                reason: CopySemanticsReason::CloneableChildRequiresConformance(*child),
            };
        }
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

fn type_param_has_bound(
    ctx: &QueryContext<'_>,
    param: Entity,
    protocol: Entity,
    context: Entity,
    root: Entity,
) -> bool {
    let Some(wc) = ctx.get::<AstWhereClause>(context) else {
        return false;
    };
    for constraint in &wc.0 {
        let WhereConstraint::Bound {
            subject, protocols, ..
        } = constraint
        else {
            continue;
        };
        if resolve_type_entity(ctx, subject, context, root) != Some(param) {
            continue;
        }
        for protocol_ty in protocols {
            let Some(bound) = resolve_type_entity(ctx, protocol_ty, context, root) else {
                continue;
            };
            if ctx.query(ProtocolRefines {
                protocol: bound,
                base: protocol,
                root,
            }) {
                return true;
            }
        }
    }
    false
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
        | AstType::Some { span, .. } => span.clone(),
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
    }
}
