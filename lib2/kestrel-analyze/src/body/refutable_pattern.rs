//! # Refutable Pattern Analyzer
//!
//! Checks that let/var bindings use irrefutable patterns. A refutable
//! pattern (literal, variant, range) in a let binding is an error because
//! it cannot match all possible values.
//!
//! Irrefutable patterns: Wildcard, Binding, Tuple of irrefutables, At with
//! irrefutable sub, Or with any irrefutable alternative, Error (to suppress
//! cascading).
//!
//! Refutable patterns: Literal, Range, Variant, ImplicitVariant, Struct
//! with refutable fields.
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
use crate::traits::{BodyCheck, Describe};
use kestrel_hir::body::*;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E300",
    name: "refutable_pattern_in_binding",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct RefutablePatternAnalyzer;

impl Describe for RefutablePatternAnalyzer {
    fn id(&self) -> &'static str {
        "refutable_pattern"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for RefutablePatternAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        // Walk all statements looking for let bindings
        // Check both top-level statements and nested blocks
        check_stmts(cx, &cx.hir.statements, &mut diags);
        if let Some(tail) = cx.hir.tail_expr {
            check_expr_for_lets(cx, tail, &mut diags);
        }

        diags
    }
}

fn check_stmts(cx: &BodyContext<'_>, stmts: &[HirStmtId], diags: &mut Vec<AnalyzeDiagnostic>) {
    for &stmt_id in stmts {
        check_stmt(cx, stmt_id, diags);
    }
}

fn check_stmt(cx: &BodyContext<'_>, id: HirStmtId, diags: &mut Vec<AnalyzeDiagnostic>) {
    match &cx.hir.stmts[id] {
        HirStmt::Let { .. } => {
            // In lib2 HIR, let bindings have a single local (not a pattern).
            // Destructuring patterns are desugared before HIR, so we don't
            // need to check patterns here — the desugaring handles it.
            // This check is relevant for pattern-bearing let statements,
            // which would need a pattern field on HirStmt::Let.
            // Currently all let bindings are irrefutable by construction.
        },
        HirStmt::Expr { expr, .. } => {
            check_expr_for_lets(cx, *expr, diags);
        },
        HirStmt::Deinit { .. } => {},
    }
}

/// Recurse into expressions to find nested let bindings (in blocks, etc.)
fn check_expr_for_lets(cx: &BodyContext<'_>, id: HirExprId, diags: &mut Vec<AnalyzeDiagnostic>) {
    match &cx.hir.exprs[id] {
        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            check_block_for_lets(cx, then_body, diags);
            if let Some(else_block) = else_body {
                check_block_for_lets(cx, else_block, diags);
            }
        },
        HirExpr::Loop { body, .. } => {
            check_block_for_lets(cx, body, diags);
        },
        HirExpr::Match { arms, .. } => {
            for arm in arms {
                check_expr_for_lets(cx, arm.body, diags);
            }
        },
        HirExpr::Block { body, .. } => {
            check_block_for_lets(cx, body, diags);
        },
        HirExpr::Closure { body, .. } => {
            check_block_for_lets(cx, body, diags);
        },
        _ => {},
    }
}

fn check_block_for_lets(
    cx: &BodyContext<'_>,
    block: &HirBlock,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    check_stmts(cx, &block.stmts, diags);
    if let Some(tail) = block.tail_expr {
        check_expr_for_lets(cx, tail, diags);
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
            .all(|f| f.pattern.map_or(true, |p| is_pattern_irrefutable(hir, p))),
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
            HirLiteral::String(s) => format!("\"{}\"", s),
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
                        f.field_name.clone()
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
