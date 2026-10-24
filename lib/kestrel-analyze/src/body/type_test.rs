//! # Type Test Analyzer
//!
//! Checks `is` patterns — and through them `x is T`, `x as? T` and
//! `consuming x as? T`, which desugar onto them (existentials design §8,
//! §8b). Types come from `TypedBody::type_tests` (the scrutinee at each `is`
//! pattern) and `expr_types`.
//!
//! ## Diagnostics
//!
//! ### E152 — `type_test_not_existential` (Error, Correctness)
//!
//! **Message:** "'is' needs an existential on the left: '{ty}' is known at
//! compile time" (concrete) or "…: '{T}' is a generic parameter" (generic)
//!
//! **Labels:**
//! - Primary: the `is` pattern (the whole `x is T` / `x as? T` when desugared)
//!   - Span source: `util::pat_span` on the `HirPat::TypeTest`
//!   - Message: "not an existential"
//!
//! **Notes:** (none)
//!
//! ### E149 — `type_test_always_fails` (Warning, Correctness)
//!
//! **Message:** "cast from '{existential}' to '{T}' always fails: '{T}' does
//! not conform to '{P}'"
//!
//! **Labels:**
//! - Primary: the `is` pattern
//!   - Span source: `util::pat_span` on the `HirPat::TypeTest`
//!   - Message: "never matches"
//!
//! **Notes:** (none)
//!
//! ### E148 — `consuming_cast_shared_box` (Error, Correctness)
//!
//! **Message:** "cannot consume the payload of '{existential}': '{B}' shares
//! its payload, so it cannot give it up; use 'any[OwnedBox] {bounds}'"
//!
//! **Labels:**
//! - Primary: the `consuming x as? T` expression
//!   - Span source: `util::expr_span` on the `HirExpr::TakePayload`
//!   - Message: "the box does not allow this"
//!
//! **Notes:** (none)

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use crate::util;
use kestrel_ast_builder::Name;
use kestrel_hecs::Entity;
use kestrel_hir::Builtin;
use kestrel_hir::body::*;
use kestrel_name_res::{ConformingProtocols, ResolveBuiltin};
use kestrel_type_infer::result::ResolvedTy;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E152",
        name: "type_test_not_existential",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E149",
        name: "type_test_always_fails",
        default_severity: Severity::Warning,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E148",
        name: "consuming_cast_shared_box",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct TypeTestAnalyzer;

impl Describe for TypeTestAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::TypeTest
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for TypeTestAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        for (pat_id, pat) in cx.hir.pats.iter() {
            let HirPat::TypeTest { target, .. } = pat else {
                continue;
            };
            let Some(scrutinee) = cx.typed.type_tests.get(&pat_id) else {
                continue;
            };
            check_type_test(cx, pat_id, target, scrutinee, &mut diags);
        }
        for (_, expr) in cx.hir.exprs.iter() {
            if let HirExpr::TakePayload { expr, span, .. } = expr {
                check_consuming_cast(cx, *expr, span, &mut diags);
            }
        }
        diags
    }
}

fn diag(index: usize, message: String, span: kestrel_span::Span, label: &str) -> AnalyzeDiagnostic {
    AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[index].id,
        severity: DESCRIPTORS[index].default_severity,
        message,
        labels: vec![DiagLabel {
            span,
            message: label.into(),
            is_primary: true,
        }],
        notes: vec![],
    }
}

fn check_type_test(
    cx: &BodyContext<'_>,
    pat_id: HirPatId,
    target: &kestrel_hir::ty::HirTy,
    scrutinee: &ResolvedTy,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let scrutinee = peel_ref(scrutinee);
    let span = util::pat_span(cx.hir, pat_id);
    let Some(bounds) = existential_bounds(scrutinee) else {
        let why = match scrutinee {
            ResolvedTy::Error | ResolvedTy::Never => return,
            ResolvedTy::Param { .. } => "is a generic parameter",
            _ => "is known at compile time",
        };
        let message = format!(
            "'is' needs an existential on the left: '{}' {why}",
            describe(cx, scrutinee)
        );
        diags.push(diag(0, message, span, "not an existential"));
        return;
    };
    let target = kestrel_type_infer::compare::resolve_hir_type(cx.query, cx.root, target);
    let Some(blocker) = kestrel_type_infer::type_test_blocker(cx.query, &target, &bounds, cx.root)
    else {
        return;
    };
    let shown = describe(cx, &target);
    let message = format!(
        "cast from '{}' to '{shown}' always fails: '{shown}' does not conform to '{}'",
        describe(cx, scrutinee),
        name_of(cx, blocker)
    );
    diags.push(diag(1, message, span, "never matches"));
}

/// `consuming x as? T` gives up the payload: the box must be a
/// `ConsumingIndirection`.
fn check_consuming_cast(
    cx: &BodyContext<'_>,
    operand: HirExprId,
    span: &kestrel_span::Span,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let Some(boxed) = cx.typed.expr_types.get(&operand).map(peel_ref) else {
        return;
    };
    let ResolvedTy::Named {
        entity: box_entity, ..
    } = boxed
    else {
        return;
    };
    if existential_bounds(boxed).is_none() {
        return;
    }
    let builtin = |b| {
        cx.query.query(ResolveBuiltin {
            builtin: b,
            root: cx.root,
        })
    };
    let conforms = |protocol: Option<Entity>| {
        protocol.is_some_and(|p| {
            cx.query
                .query(ConformingProtocols {
                    entity: *box_entity,
                    root: cx.root,
                })
                .contains(&p)
        })
    };
    if conforms(builtin(Builtin::ConsumingIndirection)) {
        return;
    }
    let box_name = name_of(cx, *box_entity);
    // A Cloneable box hands out copies that share the payload.
    let reason = if conforms(builtin(Builtin::Cloneable)) {
        format!("'{box_name}' shares its payload, so it cannot give it up")
    } else {
        format!("'{box_name}' cannot give up its payload")
    };
    let hint = builtin(Builtin::OwnedBox)
        .filter(|owned| owned != box_entity)
        .and_then(|owned| match boxed {
            ResolvedTy::Named { args, .. } => args.first().map(|raw| {
                format!(
                    "; use 'any[{}] {}'",
                    name_of(cx, owned),
                    describe_bounds(cx, raw)
                )
            }),
            _ => None,
        })
        .unwrap_or_default();
    let message = format!(
        "cannot consume the payload of '{}': {reason}{hint}",
        describe(cx, boxed)
    );
    diags.push(diag(
        2,
        message,
        span.clone(),
        "the box does not allow this",
    ));
}

fn peel_ref(ty: &ResolvedTy) -> &ResolvedTy {
    match ty {
        ResolvedTy::Ref { pointee, .. } => pointee,
        other => other,
    }
}

/// The protocol bounds of an existential box (`B[rawany P and Q]`).
fn existential_bounds(ty: &ResolvedTy) -> Option<Vec<Entity>> {
    let ResolvedTy::Named { args, .. } = ty else {
        return None;
    };
    match args.first() {
        Some(ResolvedTy::RawExistential { bounds, .. }) => {
            Some(bounds.iter().map(|(p, _)| *p).collect())
        },
        _ => None,
    }
}

fn name_of(cx: &BodyContext<'_>, entity: Entity) -> String {
    cx.query
        .get::<Name>(entity)
        .map_or_else(|| "?".into(), |n| n.0.clone())
}

/// `P and Q`.
fn describe_bounds(cx: &BodyContext<'_>, raw: &ResolvedTy) -> String {
    let ResolvedTy::RawExistential { bounds, .. } = raw else {
        return describe(cx, raw);
    };
    let names: Vec<String> = bounds.iter().map(|(p, _)| name_of(cx, *p)).collect();
    names.join(" and ")
}

/// A type as the user wrote it: `any P` for the default box, `any[B] P`
/// otherwise, a generic parameter by name.
fn describe(cx: &BodyContext<'_>, ty: &ResolvedTy) -> String {
    match ty {
        ResolvedTy::Named { entity, args } => {
            if let Some(raw) = args
                .first()
                .filter(|a| matches!(a, ResolvedTy::RawExistential { .. }))
            {
                let default_box = cx.query.query(ResolveBuiltin {
                    builtin: Builtin::DefaultBoxType,
                    root: cx.root,
                });
                // `DefaultBox` is an alias of the default box struct.
                let is_default = default_box.is_some_and(|alias| {
                    alias == *entity || alias_target(cx, alias) == Some(*entity)
                });
                return if is_default {
                    format!("any {}", describe_bounds(cx, raw))
                } else {
                    format!("any[{}] {}", name_of(cx, *entity), describe_bounds(cx, raw))
                };
            }
            name_of(cx, *entity)
        },
        ResolvedTy::Param { entity } => name_of(cx, *entity),
        _ => "?".into(),
    }
}

/// The struct an alias names (`DefaultBox[T] = RcBox[T]`).
fn alias_target(cx: &BodyContext<'_>, alias: Entity) -> Option<Entity> {
    match cx.query.query(kestrel_hir_lower::LowerTypeAnnotation {
        entity: alias,
        root: cx.root,
    })? {
        kestrel_hir::ty::HirTy::Struct { entity, .. } => Some(entity),
        _ => None,
    }
}
