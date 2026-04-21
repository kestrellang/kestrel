//! # Condition Type Analyzer
//!
//! Checks that if/while conditions are Bool (or conform to BooleanConditional).
//! Like lib1, this runs as a post-inference analyzer rather than a solver
//! constraint — primitive `lang.i1` doesn't implement protocols, so a
//! Conforms constraint would fail for direct `i1` usage in conditions.
//!
//! ## Diagnostics
//!
//! ### E101 — `condition_not_bool` (Error, Correctness)
//!
//! **Message:** "{kind} condition must be Bool"
//!
//! **Labels:**
//! - Primary: the condition expression
//!   - Message: "expected Bool, found {type}"

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Intrinsic, Name, NodeKind};
use kestrel_hir::Builtin;
use kestrel_hir::body::*;
use kestrel_name_res::{ConformingProtocols, ResolveBuiltin};
use kestrel_type_infer::result::ResolvedTy;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E101",
    name: "condition_not_bool",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ConditionCheckAnalyzer;

impl Describe for ConditionCheckAnalyzer {
    fn id(&self) -> &'static str {
        "condition_check"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for ConditionCheckAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        // Resolve BooleanConditional protocol (may not exist without stdlib)
        let bool_cond_protocol = cx.query.query(ResolveBuiltin {
            builtin: Builtin::BooleanConditional,
            root: cx.root,
        });

        // Check all if-expression conditions
        for (_expr_id, expr) in cx.hir.exprs.iter() {
            if let HirExpr::If { condition, .. } = expr {
                // Skip desugared while conditions — checked separately below
                if cx.hir.while_conditions.contains(condition) {
                    continue;
                }
                check_condition(cx, *condition, "if", bool_cond_protocol, &mut diags);
            }
        }

        // Check while-loop conditions (tracked during desugaring)
        for &cond_id in &cx.hir.while_conditions {
            check_condition(cx, cond_id, "while", bool_cond_protocol, &mut diags);
        }

        diags
    }
}

/// Check that a condition expression has Bool type.
fn check_condition(
    cx: &BodyContext<'_>,
    cond_id: HirExprId,
    kind: &str,
    bool_cond_protocol: Option<kestrel_hecs::Entity>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let Some(cond_ty) = cx.typed.expr_types.get(&cond_id) else {
        return;
    };

    // Skip error/never types (already broken — don't cascade)
    if matches!(cond_ty, ResolvedTy::Error | ResolvedTy::Never) {
        return;
    }

    // Check if the type is Bool (lang.i1)
    if is_bool(cx, cond_ty) {
        return;
    }

    // Check BooleanConditional protocol conformance
    if let Some(protocol) = bool_cond_protocol {
        if conforms_to_protocol(cx, cond_ty, protocol) {
            return;
        }
    }

    let span = util::expr_span(cx.hir, cond_id);
    let type_name = describe_type(cx, cond_ty);

    diags.push(AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[0].id,
        severity: DESCRIPTORS[0].default_severity,
        message: format!("{} condition must be Bool", kind),
        labels: vec![DiagLabel {
            span,
            message: format!("expected Bool, found {}", type_name),
            is_primary: true,
        }],
        notes: vec![],
    });
}

/// Check if a resolved type is the Bool type (lang.i1 intrinsic).
fn is_bool(cx: &BodyContext<'_>, ty: &ResolvedTy) -> bool {
    let ResolvedTy::Named { entity, .. } = ty else {
        return false;
    };
    if cx.query.get::<Intrinsic>(*entity).is_none() {
        return false;
    }
    if cx.query.get::<NodeKind>(*entity) != Some(&NodeKind::Struct) {
        return false;
    }
    cx.query.get::<Name>(*entity).is_some_and(|n| n.0 == "i1")
}

/// Check if a resolved type conforms to a given protocol.
fn conforms_to_protocol(
    cx: &BodyContext<'_>,
    ty: &ResolvedTy,
    protocol: kestrel_hecs::Entity,
) -> bool {
    let ResolvedTy::Named { entity, .. } = ty else {
        return false;
    };
    let conforming = cx.query.query(ConformingProtocols {
        entity: *entity,
        root: cx.root,
    });
    conforming.contains(&protocol)
}

/// Human-readable description of a resolved type for error messages.
fn describe_type(cx: &BodyContext<'_>, ty: &ResolvedTy) -> String {
    match ty {
        ResolvedTy::Named { entity, .. } => cx
            .query
            .get::<Name>(*entity)
            .map(|n| n.0.clone())
            .unwrap_or_else(|| "?".into()),
        ResolvedTy::Tuple(elems) => {
            let inner: Vec<String> = elems.iter().map(|e| describe_type(cx, e)).collect();
            format!("({})", inner.join(", "))
        },
        ResolvedTy::Never => "Never".into(),
        ResolvedTy::Error => "?".into(),
        ResolvedTy::Param { .. } => "type parameter".into(),
        ResolvedTy::Function { .. } => "function type".into(),
    }
}
