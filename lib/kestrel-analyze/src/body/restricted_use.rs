//! # Restricted Use Analyzer (body half)
//!
//! Reports every use of a `@deprecated` (warning) or `@unavailable` (error)
//! declaration inside a body: calls, references, member accesses, enum cases,
//! initializers, subscripts, and types written in the body (`let` and closure
//! annotations, explicit type arguments, `is T` patterns). Whether a
//! declaration is restricted comes from `UseRestrictionOf`, the one query the
//! signature half (`decl/restricted_signature.rs`) and the LSP also read.
//!
//! A use inside the restricted member's own type (or an extension of it)
//! is not reported: its initializers must still set a deprecated field, and
//! a protocol extension may still call a deprecated requirement.
//!
//! A deprecated use is not reported inside a declaration that is itself
//! deprecated or unavailable, or nested in one (an extension of a restricted
//! type counts as nested in it), so deprecated APIs can call each other during
//! a migration. An unavailable use is reported even there.
//!
//! ## Diagnostics
//!
//! ### E908 — `deprecated_use` (Warning, Usage)
//!
//! **Message:** "'{name}' is deprecated[ since {since}][: {message}][; use '{replacement}' instead]"
//!
//! **Labels:**
//! - Primary: the use
//!   - Span source: `util::expr_span` on the referencing `HirExprId`,
//!     `util::pat_span` on the `HirPatId`, or the `HirTy` span of the type
//!   - Message: "deprecated"
//! - Secondary: the declaration
//!   - Span source: `util::entity_span` on the restricted entity
//!   - Message: "declared here"
//!
//! **Notes:**
//! - (none)
//!
//! ### E913 — `unavailable_use` (Error, Usage)
//!
//! **Message:** "'{name}' is unavailable[: {message}][; use '{replacement}' instead]"
//!
//! **Labels:** as E908, with the primary message "unavailable".
//!
//! **Notes:**
//! - (none)

use std::collections::HashSet;

use kestrel_ast_builder::attrs::{RestrictionKind, UseRestrictionOf};
use kestrel_ast_builder::{AstType, ExtensionTarget, Name, NodeKind};
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::body::*;
use kestrel_hir::ty::HirTy;
use kestrel_span::Span;

use crate::body::match_pattern;
use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use crate::util;

pub(crate) static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E908",
        name: "deprecated_use",
        default_severity: Severity::Warning,
        category: Category::Usage,
    },
    DiagnosticDescriptor {
        id: "E913",
        name: "unavailable_use",
        default_severity: Severity::Error,
        category: Category::Usage,
    },
];

fn descriptor(id: &str) -> &'static DiagnosticDescriptor {
    DESCRIPTORS
        .iter()
        .find(|d| d.id == id)
        .expect("restricted_use descriptor")
}

pub struct RestrictedUseAnalyzer;

impl Describe for RestrictedUseAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::RestrictedUse
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for RestrictedUseAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut uses = UseSink::new(cx.query, cx.entity, cx.root);
        for (id, expr) in cx.hir.exprs.iter() {
            let span = util::expr_span(cx.hir, id);
            if let Some(&target) = cx.typed.resolutions.get(&id) {
                uses.entity(target, &span);
            }
            match expr {
                HirExpr::Def(entity, type_args, _) => {
                    uses.entity(*entity, &span);
                    type_args.iter().for_each(|t| uses.hir_ty(t));
                },
                HirExpr::OverloadSet { type_args, .. } => {
                    type_args.iter().for_each(|t| uses.hir_ty(t));
                },
                HirExpr::TypeRef { ty, .. } | HirExpr::TakePayload { target: ty, .. } => {
                    uses.hir_ty(ty)
                },
                HirExpr::MethodCall { type_args, .. } | HirExpr::ProtocolCall { type_args, .. } => {
                    type_args.iter().flatten().for_each(|t| uses.hir_ty(t));
                },
                HirExpr::Match {
                    scrutinee, arms, ..
                } => {
                    if let Some(ty) = cx.typed.expr_types.get(scrutinee) {
                        arms.iter()
                            .for_each(|arm| implicit_cases(cx, arm.pattern, ty, &mut uses));
                    }
                },
                HirExpr::Closure { params, .. } => {
                    params
                        .iter()
                        .filter_map(|p| p.ty.as_ref())
                        .for_each(|t| uses.hir_ty(t));
                },
                _ => {},
            }
        }
        for (id, pat) in cx.hir.pats.iter() {
            match pat {
                HirPat::Variant { entity, .. } | HirPat::Struct { entity, .. } => {
                    uses.entity(*entity, &util::pat_span(cx.hir, id));
                },
                HirPat::TypeTest { target, .. } => uses.hir_ty(target),
                _ => {},
            }
        }
        for (_, stmt) in cx.hir.stmts.iter() {
            if let HirStmt::Let { ty: Some(ty), .. } = stmt {
                uses.hir_ty(ty);
            }
        }
        uses.finish()
    }
}

/// `.Case` patterns at the top of a match arm, resolved against the
/// scrutinee's enum. Inference records no entity for an implicit case
/// pattern, so this is the one place it can be found; payload sub-patterns
/// would need their payload types and are not walked.
fn implicit_cases(
    cx: &BodyContext<'_>,
    pat: HirPatId,
    scrutinee: &kestrel_type_infer::result::ResolvedTy,
    uses: &mut UseSink<'_>,
) {
    match &cx.hir.pats[pat] {
        HirPat::ImplicitVariant { name, .. } => {
            let case = name
                .as_str()
                .and_then(|n| match_pattern::lookup_enum_case(cx, scrutinee, n));
            if let Some(case) = case {
                uses.entity(case, &util::pat_span(cx.hir, pat));
            }
        },
        HirPat::Or { alternatives, .. } => alternatives
            .iter()
            .for_each(|&a| implicit_cases(cx, a, scrutinee, uses)),
        HirPat::At { subpattern, .. } => implicit_cases(cx, *subpattern, scrutinee, uses),
        _ => {},
    }
}

/// Collects restricted uses found from one declaration or body and turns
/// them into diagnostics. Shared by both halves so the context exemption, the
/// wording and the de-duplication are decided once.
pub(crate) struct UseSink<'a> {
    ctx: &'a QueryContext<'a>,
    root: Entity,
    context: UseContext,
    /// `(entity, span start)` pairs already reported: one expression can be
    /// both a `Def` and a resolution of the same entity.
    seen: HashSet<(Entity, usize, usize)>,
    diags: Vec<AnalyzeDiagnostic>,
}

impl<'a> UseSink<'a> {
    /// A sink for uses written inside `owner` (a declaration or a body's
    /// entity).
    pub(crate) fn new(ctx: &'a QueryContext<'a>, owner: Entity, root: Entity) -> Self {
        Self {
            ctx,
            root,
            context: use_context(ctx, owner, root),
            seen: HashSet::new(),
            diags: Vec::new(),
        }
    }

    /// A use of `entity` at `span`.
    pub(crate) fn entity(&mut self, entity: Entity, span: &Span) {
        let Some(restriction) = self.ctx.query(UseRestrictionOf { entity }) else {
            return;
        };
        // Deprecated APIs may use each other; unavailable code is never usable.
        if restriction.kind == RestrictionKind::Deprecated && self.context.restriction.is_some() {
            return;
        }
        // A type's own code may use its restricted members: its initializers
        // must still set a deprecated field, and a protocol extension may
        // still call a deprecated requirement.
        let member_of = self
            .ctx
            .parent_of(entity)
            .and_then(|p| enclosed_type(self.ctx, p, self.root));
        if member_of.is_some_and(|t| self.context.types.contains(&t)) {
            return;
        }
        if !self.seen.insert((entity, span.file_id, span.start)) {
            return;
        }
        let (code, label) = match restriction.kind {
            RestrictionKind::Deprecated => ("E908", "deprecated"),
            RestrictionKind::Unavailable => ("E913", "unavailable"),
        };
        let d = descriptor(code);
        let name = format!("'{}'", display_name(self.ctx, entity));
        self.diags.push(AnalyzeDiagnostic {
            descriptor_id: d.id,
            severity: d.default_severity,
            message: restriction.describe(&name),
            labels: vec![
                DiagLabel {
                    span: span.clone(),
                    message: label.into(),
                    is_primary: true,
                },
                DiagLabel {
                    span: util::entity_span(self.ctx, entity),
                    message: "declared here".into(),
                    is_primary: false,
                },
            ],
            notes: vec![],
        });
    }

    /// Every named type in a lowered type.
    pub(crate) fn hir_ty(&mut self, ty: &HirTy) {
        match ty {
            HirTy::Struct { entity, args, span }
            | HirTy::Enum { entity, args, span }
            | HirTy::Protocol { entity, args, span }
            | HirTy::AliasUse { entity, args, span } => {
                self.entity(*entity, span);
                args.iter().for_each(|a| self.hir_ty(a));
            },
            HirTy::AssocProjection { base, assoc, span } => {
                self.hir_ty(base);
                self.entity(*assoc, span);
            },
            HirTy::Tuple(elems, _) => elems.iter().for_each(|e| self.hir_ty(e)),
            HirTy::Function { params, ret, .. } => {
                params.iter().for_each(|p| self.hir_ty(p));
                self.hir_ty(ret);
            },
            HirTy::Opaque { bounds, .. } => bounds.iter().for_each(|b| self.hir_ty(b)),
            HirTy::RawExistential {
                bounds, bindings, ..
            } => {
                bounds.iter().for_each(|b| self.hir_ty(b));
                bindings.iter().for_each(|b| self.hir_ty(&b.ty));
            },
            HirTy::Ref { inner, .. } => self.hir_ty(inner),
            HirTy::Param(..)
            | HirTy::SelfType(..)
            | HirTy::Never(_)
            | HirTy::Infer(_)
            | HirTy::Error(_) => {},
        }
    }

    pub(crate) fn root(&self) -> Entity {
        self.root
    }

    pub(crate) fn finish(self) -> Vec<AnalyzeDiagnostic> {
        self.diags
    }
}

/// What the declarations enclosing a use say about it.
struct UseContext {
    /// The strongest restriction on the use's declaration or one enclosing
    /// it.
    restriction: Option<RestrictionKind>,
    /// The types the use sits in: enclosing structs, enums and protocols,
    /// and the types extended by enclosing extensions.
    types: Vec<Entity>,
}

/// Walk out from `entity`. An extension sits inside the type it extends, so
/// the members of `extend Old { ... }` are in `Old`'s context.
fn use_context(ctx: &QueryContext<'_>, entity: Entity, root: Entity) -> UseContext {
    let mut out = UseContext {
        restriction: None,
        types: Vec::new(),
    };
    let mut current = Some(entity);
    while let Some(e) = current {
        let own = ctx.query(UseRestrictionOf { entity: e }).map(|r| r.kind);
        out.restriction = out.restriction.max(own);
        if let Some(t) = enclosed_type(ctx, e, root) {
            let extended = ctx.query(UseRestrictionOf { entity: t }).map(|r| r.kind);
            out.restriction = out.restriction.max(extended);
            out.types.push(t);
        }
        current = ctx.parent_of(e);
    }
    out
}

/// The type `entity` stands for as a container: itself for a struct, enum
/// or protocol, the extended type for an extension.
fn enclosed_type(ctx: &QueryContext<'_>, entity: Entity, root: Entity) -> Option<Entity> {
    match ctx.get::<NodeKind>(entity)? {
        NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol => Some(entity),
        NodeKind::Extension => ctx.query(kestrel_name_res::ExtensionTargetEntity {
            extension: entity,
            root,
        }),
        _ => None,
    }
}

/// How a restricted declaration is named in the message: `parse`,
/// `Shape.Circle`, `Point.init`, `Table.subscript`.
fn display_name(ctx: &QueryContext<'_>, entity: Entity) -> String {
    let owner = || {
        ctx.parent_of(entity)
            .map(|p| owner_name(ctx, p))
            .unwrap_or_default()
    };
    match ctx.get::<NodeKind>(entity) {
        Some(NodeKind::Initializer) => format!("{}.init", owner()),
        Some(NodeKind::Subscript) => format!("{}.subscript", owner()),
        Some(NodeKind::EnumCase) => format!("{}.{}", owner(), util::entity_name(ctx, entity)),
        _ => util::entity_name(ctx, entity),
    }
}

/// A type's name, or for an extension the name of the type it extends.
fn owner_name(ctx: &QueryContext<'_>, entity: Entity) -> String {
    if let Some(name) = ctx.get::<Name>(entity) {
        return name.0.clone();
    }
    match ctx.get::<ExtensionTarget>(entity).map(|t| &t.0) {
        Some(AstType::Named { segments, .. }) => {
            segments.last().map(|s| s.name.clone()).unwrap_or_default()
        },
        _ => String::new(),
    }
}
