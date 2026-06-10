//! # Refutable Pattern Analyzer
//!
//! Checks that `let <pat> = value` bindings use irrefutable patterns. HIR
//! lowering desugars any non-trivial let pattern into a match with
//! `MatchSource::LetDestructure` (see `hir-lower/src/stmt.rs`). This
//! analyzer walks those matches and flags the arm pattern when it is not
//! guaranteed to cover every value of the scrutinee's type.
//!
//! Uses `kestrel_pattern_matching::is_irrefutable` for the type-aware
//! check (so e.g. a single-variant enum destructure is irrefutable,
//! but `Option.Some(x)` over `Option[T]` is not).
//!
//! ## Diagnostics
//!
//! ### E300 — `refutable_pattern_in_binding` (Error, Correctness)
//!
//! **Message:** "refutable pattern in let binding"
//!
//! **Labels:**
//! - Primary: the pattern in the let statement
//!   - Span source: `util::pat_span` on the `HirPatId`
//!   - Message: "this pattern may not match all values"
//!
//! **Notes:** "help: use 'if let' or 'match' for refutable patterns"

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use crate::util;
use kestrel_hir::body::*;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E300",
    name: "refutable_pattern_in_binding",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct RefutablePatternAnalyzer;

impl Describe for RefutablePatternAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::RefutablePattern
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for RefutablePatternAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        for (_, expr) in cx.hir.exprs.iter() {
            let HirExpr::Match {
                scrutinee,
                arms,
                source: MatchSource::LetDestructure,
                ..
            } = expr
            else {
                continue;
            };
            let Some(arm) = arms.first() else { continue };

            // Skip if the scrutinee failed to type — the diagnostic is
            // already reported and a refutability finding would be noise.
            let Some(scrutinee_ty) = cx.typed.expr_types.get(scrutinee) else {
                continue;
            };
            if matches!(scrutinee_ty, kestrel_type_infer::result::ResolvedTy::Error) {
                continue;
            }

            if kestrel_pattern_matching::is_irrefutable(
                cx.hir,
                cx.query,
                cx.root,
                arm.pattern,
                scrutinee_ty,
            ) {
                continue;
            }

            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: "refutable pattern in let binding".into(),
                labels: vec![DiagLabel {
                    span: util::pat_span(cx.hir, arm.pattern),
                    message: "this pattern may not match all values".into(),
                    is_primary: true,
                }],
                notes: vec!["help: use 'if let' or 'match' for refutable patterns".into()],
            });
        }

        diags
    }
}

// ===== Irrefutability checks =====

/// Check if a pattern is irrefutable (always matches).
/// Used by other analyzers in this module.
pub(crate) fn is_pattern_irrefutable(hir: &HirBody, pat_id: HirPatId) -> bool {
    match &hir.pats[pat_id] {
        HirPat::Wildcard { .. } => true,
        HirPat::Binding { .. } => true,
        HirPat::Tuple { prefix, suffix, .. } => prefix
            .iter()
            .chain(suffix.iter())
            .all(|&e| is_pattern_irrefutable(hir, e)),
        HirPat::Array { .. } => false, // Array patterns are always refutable (unknown length)
        HirPat::Literal { .. } => false,
        HirPat::Range { .. } => false,
        HirPat::Variant { .. } => false,
        HirPat::ImplicitVariant { .. } => false,
        // Struct patterns are irrefutable if all field patterns are irrefutable
        HirPat::Struct { fields, .. } => fields
            .iter()
            .all(|f| f.pattern.is_none_or(|p| is_pattern_irrefutable(hir, p))),
        // Or-pattern is irrefutable if ANY alternative is irrefutable
        HirPat::Or { alternatives, .. } => {
            alternatives.iter().any(|&a| is_pattern_irrefutable(hir, a))
        },
        // At-pattern is irrefutable if the subpattern is irrefutable
        HirPat::At { subpattern, .. } => is_pattern_irrefutable(hir, *subpattern),
        // Error patterns: treat as irrefutable to avoid cascading
        HirPat::Error { .. } => true,
    }
}

/// Generate a human-readable description of a pattern for error messages.
#[allow(dead_code)]
pub(crate) fn describe_pattern(hir: &HirBody, pat_id: HirPatId) -> String {
    match &hir.pats[pat_id] {
        HirPat::Wildcard { .. } => "_".into(),
        HirPat::Binding { local, .. } => hir.locals[*local].name.clone(),
        HirPat::Tuple {
            prefix,
            has_rest,
            suffix,
            ..
        } => {
            let mut parts: Vec<String> = prefix.iter().map(|&e| describe_pattern(hir, e)).collect();
            if *has_rest {
                parts.push("..".into());
                parts.extend(suffix.iter().map(|&e| describe_pattern(hir, e)));
            }
            format!("({})", parts.join(", "))
        },
        HirPat::Literal { value, .. } => match value {
            HirLiteral::Integer(i) => i.to_string(),
            HirLiteral::Float(f) => f.to_string(),
            HirLiteral::String { value, .. } => format!("\"{}\"", value),
            HirLiteral::Char(c) => {
                if let Some(ch) = char::from_u32(*c) {
                    format!("'{}'", ch)
                } else {
                    format!("'\\u{{{:X}}}'", c)
                }
            },
            HirLiteral::Bool(b) => b.to_string(),
            HirLiteral::Null => "null".into(),
        },
        HirPat::Variant { args, .. } | HirPat::ImplicitVariant { args, .. } => {
            let name = match &hir.pats[pat_id] {
                HirPat::ImplicitVariant { name, .. } => format!(".{}", name),
                _ => "<variant>".into(),
            };
            if args.is_empty() {
                name
            } else {
                let inner: Vec<String> = args
                    .iter()
                    .map(|a| {
                        let p = describe_pattern(hir, a.pattern);
                        if let Some(label) = &a.label {
                            format!("{}: {}", label, p)
                        } else {
                            p
                        }
                    })
                    .collect();
                format!("{}({})", name, inner.join(", "))
            }
        },
        HirPat::Range {
            start,
            end,
            inclusive,
            ..
        } => {
            let s = start
                .as_ref()
                .map(|l| format!("{:?}", l))
                .unwrap_or_default();
            let e = end.as_ref().map(|l| format!("{:?}", l)).unwrap_or_default();
            let op = if *inclusive { "..=" } else { "..<" };
            format!("{}{}{}", s, op, e)
        },
        HirPat::Struct { fields, .. } => {
            let inner: Vec<String> = fields
                .iter()
                .map(|f| {
                    if let Some(pat) = f.pattern {
                        format!("{}: {}", f.field_name, describe_pattern(hir, pat))
                    } else {
                        f.field_name.as_str_or_empty().to_string()
                    }
                })
                .collect();
            format!("{{ {} }}", inner.join(", "))
        },
        HirPat::Array {
            prefix,
            rest,
            suffix,
            ..
        } => {
            let mut parts: Vec<String> = prefix.iter().map(|&e| describe_pattern(hir, e)).collect();
            match rest {
                Some(Some(local)) => {
                    parts.push(format!("..{}", hir.locals[*local].name));
                    parts.extend(suffix.iter().map(|&e| describe_pattern(hir, e)));
                },
                Some(None) => {
                    parts.push("..".into());
                    parts.extend(suffix.iter().map(|&e| describe_pattern(hir, e)));
                },
                None => {},
            }
            format!("[{}]", parts.join(", "))
        },
        HirPat::Or { alternatives, .. } => {
            let parts: Vec<String> = alternatives
                .iter()
                .map(|&a| describe_pattern(hir, a))
                .collect();
            parts.join(" | ")
        },
        HirPat::At {
            binding,
            subpattern,
            ..
        } => {
            format!(
                "{} @ {}",
                hir.locals[*binding].name,
                describe_pattern(hir, *subpattern)
            )
        },
        HirPat::Error { .. } => "<error>".into(),
    }
}
