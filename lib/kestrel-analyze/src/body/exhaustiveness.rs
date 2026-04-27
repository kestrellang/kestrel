//! # Exhaustiveness Analyzer
//!
//! The single analyzer for match correctness. Walks every `HirExpr::Match`
//! and, based on its `MatchSource`, emits:
//!
//! - **`UserMatch`**: full exhaustiveness / redundancy / overlap analysis
//!   (`E303`–`E307`). Uses the pattern matrix algorithm (Maranget 2007).
//! - **`IfLet`**: `E302` when the user pattern is irrefutable (the else
//!   branch is dead code).
//! - **`WhileLet`**: `E308` when the user pattern is irrefutable (loop
//!   never terminates via a failed bind).
//! - **`GuardLet`**: `E309` when the user pattern is irrefutable (the
//!   `else { ... }` branch is dead code).
//! - **`ForLoop` / `LetDestructure` / `TryOp` / `UnwrapOp`**: nothing —
//!   these desugared shapes are always exhaustive by construction and
//!   their irrefutability is checked by dedicated analyzers (e.g.
//!   `for_loop_pattern`, `refutable_pattern`).
//!
//! ## Diagnostics
//!
//! | Code | Severity | Meaning |
//! |------|----------|---------|
//! | E302 | Warning  | irrefutable pattern in if-let condition |
//! | E303 | Warning  | irrefutable pattern in match arm makes subsequent arms unreachable |
//! | E304 | Error    | empty match on inhabited type |
//! | E305 | Error    | non-exhaustive match: missing {witnesses} |
//! | E306 | Warning  | unreachable pattern |
//! | E307 | Warning  | overlapping range patterns |
//! | E308 | Warning  | irrefutable pattern in while-let condition |
//! | E309 | Warning  | irrefutable pattern in guard-let condition |

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;
use kestrel_hir::body::*;
use kestrel_type_infer::result::ResolvedTy;

use kestrel_pattern_matching as pattern;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E302",
        name: "irrefutable_if_let",
        default_severity: Severity::Warning,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E303",
        name: "irrefutable_match_arm",
        default_severity: Severity::Warning,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E304",
        name: "empty_match",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E305",
        name: "non_exhaustive_match",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E306",
        name: "unreachable_pattern",
        default_severity: Severity::Warning,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E307",
        name: "overlapping_range",
        default_severity: Severity::Warning,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E308",
        name: "irrefutable_while_let",
        default_severity: Severity::Warning,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E309",
        name: "irrefutable_guard_let",
        default_severity: Severity::Warning,
        category: Category::Correctness,
    },
];

fn descriptor(id: &str) -> &'static DiagnosticDescriptor {
    DESCRIPTORS
        .iter()
        .find(|d| d.id == id)
        .expect("descriptor id must exist")
}

pub struct ExhaustivenessAnalyzer;

impl Describe for ExhaustivenessAnalyzer {
    fn id(&self) -> &'static str {
        "exhaustiveness"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for ExhaustivenessAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        for (expr_id, expr) in cx.hir.exprs.iter() {
            let HirExpr::Match {
                scrutinee,
                arms,
                source,
                ..
            } = expr
            else {
                continue;
            };

            match source {
                MatchSource::UserMatch => {
                    check_user_match(cx, expr_id, *scrutinee, arms, &mut diags);
                },
                MatchSource::IfLet => {
                    check_irrefutable_let(cx, *scrutinee, arms, "E302", &mut diags);
                },
                MatchSource::WhileLet => {
                    check_irrefutable_let(cx, *scrutinee, arms, "E308", &mut diags);
                },
                MatchSource::GuardLet => {
                    check_irrefutable_let(cx, *scrutinee, arms, "E309", &mut diags);
                },
                // Desugared matches whose arm shape is synthetic and always
                // exhaustive by construction. Dedicated analyzers handle
                // refutability concerns (for_loop_pattern, refutable_pattern).
                MatchSource::ForLoop
                | MatchSource::LetDestructure
                | MatchSource::ParamDestructure
                | MatchSource::TryOp
                | MatchSource::UnwrapOp => {},
            }
        }

        diags
    }
}

/// Full analysis for a user-written `match`: empty-match, non-exhaustive,
/// redundant arms, irrefutable non-last arms, overlapping ranges.
fn check_user_match(
    cx: &BodyContext<'_>,
    expr_id: HirExprId,
    scrutinee: HirExprId,
    arms: &[HirMatchArm],
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let Some(scrutinee_ty) = cx.typed.expr_types.get(&scrutinee) else {
        return;
    };

    // Skip analysis when the scrutinee type couldn't be inferred — a
    // type-inference error will already be reported and any exhaustiveness
    // finding would be based on `Unknown` constructors, producing a
    // misleading "missing _" diagnostic on valid code.
    if matches!(scrutinee_ty, ResolvedTy::Error) {
        return;
    }

    // Skip when any arm pattern contains an HirPat::Error (e.g. a malformed
    // pattern that hir-lower already diagnosed), or when `match_pattern`
    // will flag the pattern as structurally invalid (unknown case, bad
    // arity, inconsistent or-binding, float-in-pattern). The flattener
    // treats those as wildcards, which would falsely declare the match
    // exhaustive or mark subsequent arms unreachable.
    if arms.iter().any(|a| {
        pat_has_error(cx.hir, a.pattern)
            || crate::body::match_pattern::is_invalid(cx, a.pattern, Some(scrutinee_ty))
    }) {
        return;
    }

    // E304: empty match on inhabited type.
    if arms.is_empty() {
        if !matches!(scrutinee_ty, ResolvedTy::Never) {
            let d = descriptor("E304");
            diags.push(AnalyzeDiagnostic {
                descriptor_id: d.id,
                severity: d.default_severity,
                message: "empty match on inhabited type".into(),
                labels: vec![DiagLabel {
                    span: util::expr_span(cx.hir, expr_id),
                    message: "match has no arms".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        }
        return;
    }

    let result = pattern::check_match(cx.hir, cx.query, cx.root, scrutinee_ty, arms);

    // E305: non-exhaustive match.
    if !result.is_exhaustive {
        let witnesses: Vec<String> = result
            .missing_patterns
            .iter()
            .map(|w| w.to_string())
            .collect();
        let d = descriptor("E305");
        diags.push(AnalyzeDiagnostic {
            descriptor_id: d.id,
            severity: d.default_severity,
            message: format!("non-exhaustive match: missing {}", witnesses.join(", ")),
            labels: vec![DiagLabel {
                span: util::expr_span(cx.hir, expr_id),
                message: "not all cases covered".into(),
                is_primary: true,
            }],
            notes: vec!["help: add a wildcard pattern '_' or cover the missing cases".into()],
        });
    }

    // E307: overlapping ranges (partial overlap with a prior range).
    for &arm_idx in &result.overlapping_arms {
        if let Some(arm) = arms.get(arm_idx) {
            let d = descriptor("E307");
            diags.push(AnalyzeDiagnostic {
                descriptor_id: d.id,
                severity: d.default_severity,
                message: "overlapping range patterns".into(),
                labels: vec![DiagLabel {
                    span: util::pat_span(cx.hir, arm.pattern),
                    message: "this range overlaps with a previous pattern".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        }
    }

    // E303 vs E306: an arm is redundant either because a PRIOR arm was
    // irrefutable (the "cause") or because its individual coverage was
    // subsumed (the "effect"). Both facts describe the same unreachable
    // code — we emit at most one diagnostic per arm, preferring E306
    // (labels the unreachable code itself, which is what the user needs
    // to fix). E303 is reserved for the degenerate case where no E306
    // would fire but an arm is still irrefutable — vanishingly rare in
    // practice and currently unreached.
    //
    // Overlap-flagged arms are partial overlaps (not fully covered), so
    // they shouldn't also be in redundant_arms, but we filter defensively.
    for &arm_idx in &result.redundant_arms {
        if result.overlapping_arms.contains(&arm_idx) {
            continue;
        }
        if let Some(arm) = arms.get(arm_idx) {
            let d = descriptor("E306");
            diags.push(AnalyzeDiagnostic {
                descriptor_id: d.id,
                severity: d.default_severity,
                message: "unreachable pattern".into(),
                labels: vec![DiagLabel {
                    span: util::pat_span(cx.hir, arm.pattern),
                    message: "this pattern is unreachable".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        }
    }
}

/// Desugared `if let` / `while let` / `guard let` all produce the same
/// 2-arm match shape: `{ user_pattern => true, _ => false }`. If the user's
/// pattern is irrefutable for the scrutinee type, the second arm is dead
/// and we emit the source-specific warning.
fn check_irrefutable_let(
    cx: &BodyContext<'_>,
    scrutinee: HirExprId,
    arms: &[HirMatchArm],
    code: &'static str,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let Some(user_arm) = arms.first() else {
        return;
    };
    if user_arm.guard.is_some() {
        return;
    }
    let Some(scrutinee_ty) = cx.typed.expr_types.get(&scrutinee) else {
        return;
    };
    let is_irrefutable = kestrel_pattern_matching::is_irrefutable(
        cx.hir,
        cx.query,
        cx.root,
        user_arm.pattern,
        scrutinee_ty,
    );
    if !is_irrefutable {
        return;
    }
    let d = descriptor(code);
    let (message, note) = match code {
        "E302" => (
            "irrefutable pattern in if-let condition",
            "help: use a plain 'let' binding instead",
        ),
        "E308" => (
            "irrefutable pattern in while-let condition",
            "help: use a plain 'while' with a 'let' binding inside instead",
        ),
        "E309" => (
            "irrefutable pattern in guard-let condition",
            "help: use a plain 'let' binding — the else branch is dead code",
        ),
        _ => unreachable!("check_irrefutable_let called with unknown code {}", code),
    };
    diags.push(AnalyzeDiagnostic {
        descriptor_id: d.id,
        severity: d.default_severity,
        message: message.into(),
        labels: vec![DiagLabel {
            span: util::pat_span(cx.hir, user_arm.pattern),
            message: "this pattern always matches".into(),
            is_primary: true,
        }],
        notes: vec![note.into()],
    });
}

/// Walk a HIR pattern tree looking for an `Error` node left behind by
/// hir-lower when it couldn't lower a malformed pattern.
fn pat_has_error(hir: &HirBody, pat: HirPatId) -> bool {
    match &hir.pats[pat] {
        HirPat::Error { .. } => true,
        HirPat::Wildcard { .. }
        | HirPat::Binding { .. }
        | HirPat::Literal { .. }
        | HirPat::Range { .. } => false,
        HirPat::Tuple { prefix, suffix, .. } | HirPat::Array { prefix, suffix, .. } => {
            prefix.iter().any(|&p| pat_has_error(hir, p))
                || suffix.iter().any(|&p| pat_has_error(hir, p))
        },
        HirPat::Variant { args, .. } | HirPat::ImplicitVariant { args, .. } => {
            args.iter().any(|a| pat_has_error(hir, a.pattern))
        },
        HirPat::Struct { fields, .. } => fields
            .iter()
            .any(|f| f.pattern.is_some_and(|p| pat_has_error(hir, p))),
        HirPat::Or { alternatives, .. } => alternatives.iter().any(|&p| pat_has_error(hir, p)),
        HirPat::At { subpattern, .. } => pat_has_error(hir, *subpattern),
    }
}
