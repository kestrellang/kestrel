//! # Integer Literal Range Analyzer
//!
//! Rejects integer literals whose value does not fit the fixed-width integer
//! type they resolved to. Without this check an out-of-range literal silently
//! truncates/wraps at codegen (`let a: Int8 = 200` runs as `-56`,
//! `let c: Int64 = 9223372036854775808` becomes `Int64.minValue`), with no
//! diagnostic — character escapes (`'\xFF'`) are range-checked but numeric
//! literals were not. This runs post-inference, where each literal's concrete
//! target type (and thus its valid range) is known.
//!
//! The literal value is carried as `i128` (see `HirLiteral::Integer`), so the
//! full magnitude is available — `2^63` and `Int64.minValue` no longer collide.
//! A leading `-` is the `negate` operator over the literal, so a unary-`negate`
//! whose receiver is an integer literal is checked as the *negated* value
//! (this is what makes `Int8.minValue = -128` valid while `-129` is rejected).
//!
//! ## Diagnostics
//!
//! ### E121 — `integer_literal_out_of_range` (Error, Correctness)
//!
//! **Message:** "integer literal out of range for `{type}`"
//!
//! **Labels:**
//! - Primary: the literal expression (or the `-literal` operator expression)
//!   - Span source: `util::expr_span` on the literal `HirExprId`, or on the
//!     `negate` `ProtocolCall` `HirExprId` when the literal is negated
//!   - Message: "{value} is not in the range {min}...{max}"

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use crate::util;
use kestrel_ast_builder::Name;
use kestrel_hir::body::*;
use kestrel_type_infer::result::ResolvedTy;
use std::collections::HashSet;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E121",
    name: "integer_literal_out_of_range",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct IntegerLiteralRangeAnalyzer;

impl Describe for IntegerLiteralRangeAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::IntegerLiteralRange
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for IntegerLiteralRangeAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        // Literals that are the receiver of a `negate` operator — checked below
        // as their negated value, so the bare-literal pass must skip them.
        let mut negated: HashSet<HirExprId> = HashSet::new();

        // Pass 1: `negate` over an integer literal → check the negated value.
        // The `-literal` operator and its receiver share the same resolved type.
        for (call_id, expr) in cx.hir.exprs.iter() {
            let HirExpr::ProtocolCall {
                receiver, method, ..
            } = expr
            else {
                continue;
            };
            if method.as_str() != Some("negate") {
                continue;
            }
            let HirExpr::Literal {
                value: HirLiteral::Integer(v),
                ..
            } = &cx.hir.exprs[*receiver]
            else {
                continue;
            };
            negated.insert(*receiver);
            if let Some(ty) = cx.typed.expr_types.get(receiver) {
                check_range(cx, v.wrapping_neg(), ty, call_id, &mut diags);
            }
        }

        // Pass 2: every other integer literal → check the literal value.
        for (id, expr) in cx.hir.exprs.iter() {
            if negated.contains(&id) {
                continue;
            }
            let HirExpr::Literal {
                value: HirLiteral::Integer(v),
                ..
            } = expr
            else {
                continue;
            };
            if let Some(ty) = cx.typed.expr_types.get(&id) {
                check_range(cx, *v, ty, id, &mut diags);
            }
        }

        diags
    }
}

/// Emit E121 if `value` falls outside the range of the fixed-width integer
/// type `ty`. Non-integer / non-concrete types are ignored.
fn check_range(
    cx: &BodyContext<'_>,
    value: i128,
    ty: &ResolvedTy,
    site: HirExprId,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let Some((min, max, type_name)) = int_type_bounds(cx, ty) else {
        return;
    };
    if value >= min && value <= max {
        return;
    }
    diags.push(AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[0].id,
        severity: DESCRIPTORS[0].default_severity,
        message: format!("integer literal out of range for `{}`", type_name),
        labels: vec![DiagLabel {
            span: util::expr_span(cx.hir, site),
            message: format!("{} is not in the range {}...{}", value, min, max),
            is_primary: true,
        }],
        notes: vec![],
    });
}

/// `(min, max, name)` for a fixed-width integer type, by resolved-type name.
/// Handles both the stdlib wrappers (`Int8`..`Int64`, `UInt8`..`UInt64`) and
/// the underlying `lang` intrinsics (`i8`..`i64`, `u8`..`u64`). All bounds fit
/// in `i128` (`u64::MAX` < `i128::MAX`). Returns `None` for any other type,
/// including unresolved type params and generic `Self`.
fn int_type_bounds(cx: &BodyContext<'_>, ty: &ResolvedTy) -> Option<(i128, i128, String)> {
    let ResolvedTy::Named { entity, .. } = ty else {
        return None;
    };
    let name = cx.query.get::<Name>(*entity)?.0.clone();
    let (bits, signed) = match name.as_str() {
        "Int8" | "i8" => (8u32, true),
        "Int16" | "i16" => (16, true),
        "Int32" | "i32" => (32, true),
        "Int64" | "i64" => (64, true),
        "UInt8" | "u8" => (8, false),
        "UInt16" | "u16" => (16, false),
        "UInt32" | "u32" => (32, false),
        "UInt64" | "u64" => (64, false),
        _ => return None,
    };
    let (min, max) = if signed {
        (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
    } else {
        (0, (1i128 << bits) - 1)
    };
    Some((min, max, name))
}
