//! Irrefutable pattern warning analyzer.
//!
//! This analyzer warns when irrefutable patterns are used in contexts where
//! refutable patterns are expected:
//!
//! 1. **if-let expressions**: Using an irrefutable pattern makes the condition
//!    always true, so the else branch is dead code. A regular `let` binding
//!    should be used instead.
//!
//! 2. **match expressions**: An irrefutable pattern before the last arm makes
//!    all subsequent arms unreachable.
//!
//! # Examples
//!
//! ```ignore
//! // Warning: irrefutable pattern in if-let
//! if let x = value {
//!     x
//! } else {
//!     0  // Dead code!
//! }
//!
//! // Warning: irrefutable pattern makes arms unreachable
//! match value {
//!     x => println("always matches"),
//!     0 => println("never reached"),  // Unreachable!
//! }
//! ```

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_tree::expr::{ExprKind, Expression, IfCondition};
use kestrel_semantic_tree::pattern::{Pattern, PatternKind};
use kestrel_semantic_tree::ty::TyKind;

mod diagnostics;
use diagnostics::{IrrefutableIfLetWarning, IrrefutableMatchArmWarning};

pub struct IrrefutablePatternAnalyzer;

impl IrrefutablePatternAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for IrrefutablePatternAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for IrrefutablePatternAnalyzer {
    fn name(&self) -> &'static str {
        "irrefutable_pattern"
    }

    fn visit_expression(&mut self, expr: &Expression, ctx: &mut AnalysisContext) {
        match &expr.kind {
            ExprKind::If { conditions, .. } => {
                // Check each let condition for irrefutable patterns
                for condition in conditions {
                    if let IfCondition::Let { pattern, .. } = condition
                        && is_pattern_irrefutable(pattern)
                    {
                        ctx.report(IrrefutableIfLetWarning {
                            pattern_span: pattern.span.clone(),
                            pattern_description: describe_pattern(pattern),
                        });
                    }
                }
            },
            ExprKind::Match { arms, .. } => {
                // Check if any non-last arm has an irrefutable pattern (without a guard)
                if arms.len() > 1 {
                    for (i, arm) in arms.iter().enumerate() {
                        // Skip the last arm - it's fine for it to be irrefutable (like a catch-all)
                        if i == arms.len() - 1 {
                            break;
                        }
                        // Only warn if there's no guard (guards make patterns refutable)
                        if arm.guard.is_none() && is_pattern_irrefutable(&arm.pattern) {
                            let unreachable_count = arms.len() - i - 1;
                            ctx.report(IrrefutableMatchArmWarning {
                                pattern_span: arm.pattern.span.clone(),
                                pattern_description: describe_pattern(&arm.pattern),
                                unreachable_count,
                            });
                        }
                    }
                }
            },
            _ => {},
        }
    }
}

/// Check if a pattern is irrefutable (always matches any value of its type).
///
/// Irrefutable patterns:
/// - Wildcard `_`
/// - Local binding `x`
/// - Tuple of irrefutable patterns `(a, b)`
/// - Struct pattern with all irrefutable fields
/// - Single-case enum with irrefutable bindings
/// - Or-pattern where any alternative is irrefutable
/// - At-pattern where the subpattern is irrefutable
/// - Rest pattern `..`
/// - Error (to avoid cascading errors)
///
/// Refutable patterns:
/// - Literal `42`, `"hello"`, `true`
/// - Enum variant `.Some(x)`, `.None` (unless single-case)
/// - Range `0..=9`
/// - Array (checks length)
fn is_pattern_irrefutable(pattern: &Pattern) -> bool {
    match &pattern.kind {
        // Wildcard always matches any value
        PatternKind::Wildcard => true,

        // Local binding always matches (binds any value to a name)
        PatternKind::Local { .. } => true,

        // Tuple is irrefutable if ALL elements (prefix + suffix) are irrefutable
        PatternKind::Tuple { prefix, suffix, .. } => prefix
            .iter()
            .chain(suffix.iter())
            .all(is_pattern_irrefutable),

        // Literal patterns are REFUTABLE - they only match one specific value
        PatternKind::Literal { .. } => false,

        // Enum variant patterns are REFUTABLE by default, UNLESS it's a single-case enum
        PatternKind::EnumVariant { bindings, .. } => {
            // Check if this is a single-case enum by looking at the pattern's type
            if let TyKind::Enum { symbol, .. } = pattern.ty.kind() {
                let cases = symbol.cases();
                if cases.len() == 1 {
                    // Single-case enum is irrefutable if all bindings are irrefutable
                    return bindings.iter().all(|b| is_pattern_irrefutable(&b.pattern));
                }
            }
            // Multi-case enum or unresolved type - refutable
            false
        },

        // Range patterns are REFUTABLE - they don't cover all values
        PatternKind::Range { .. } => false,

        // Struct patterns are irrefutable if all field patterns are irrefutable
        PatternKind::Struct { fields, .. } => {
            fields.iter().all(|f| is_pattern_irrefutable(&f.pattern))
        },

        // Array patterns are REFUTABLE - they check array length
        PatternKind::Array { .. } => false,

        // Or-patterns are irrefutable if ANY alternative is irrefutable
        PatternKind::Or { alternatives } => alternatives.iter().any(is_pattern_irrefutable),

        // At-patterns are irrefutable if the subpattern is irrefutable
        PatternKind::At { subpattern, .. } => is_pattern_irrefutable(subpattern),

        // Rest patterns are always irrefutable (they match any remaining elements)
        PatternKind::Rest => true,

        // Error patterns are treated as irrefutable to avoid cascading errors
        PatternKind::Error => true,
    }
}

/// Generate a human-readable description of a pattern for error messages.
fn describe_pattern(pattern: &Pattern) -> String {
    match &pattern.kind {
        PatternKind::Wildcard => "_".to_string(),
        PatternKind::Local { name, .. } => name.clone(),
        PatternKind::Tuple {
            prefix,
            has_rest,
            suffix,
        } => {
            let mut parts: Vec<String> = prefix.iter().map(describe_pattern).collect();
            if *has_rest {
                parts.push("..".to_string());
            }
            parts.extend(suffix.iter().map(describe_pattern));
            format!("({})", parts.join(", "))
        },
        PatternKind::Literal { value } => {
            use kestrel_semantic_tree::expr::LiteralValue;
            match value {
                LiteralValue::Unit => "()".to_string(),
                LiteralValue::Integer(i) => i.to_string(),
                LiteralValue::Float(f) => f.to_string(),
                LiteralValue::String(s) => format!("\"{}\"", s),
                LiteralValue::Char(c) => {
                    if let Some(ch) = char::from_u32(*c) {
                        format!("'{}'", ch)
                    } else {
                        format!("'\\u{{{:X}}}'", c)
                    }
                },
                LiteralValue::Bool(b) => b.to_string(),
                LiteralValue::Null => "null".to_string(),
            }
        },
        PatternKind::EnumVariant {
            case_name,
            bindings,
            ..
        } => {
            if bindings.is_empty() {
                format!(".{}", case_name)
            } else {
                let inner: Vec<String> = bindings
                    .iter()
                    .map(|b| {
                        if let Some(label) = &b.label {
                            format!("{}: {}", label, describe_pattern(&b.pattern))
                        } else {
                            describe_pattern(&b.pattern)
                        }
                    })
                    .collect();
                format!(".{}({})", case_name, inner.join(", "))
            }
        },
        PatternKind::Range {
            start,
            end,
            inclusive,
        } => {
            use kestrel_semantic_tree::pattern::RangeBound;
            let start_str = match start {
                Some(RangeBound::Integer(i)) => i.to_string(),
                Some(RangeBound::Char(c)) => format!("'{}'", c),
                None => String::new(),
            };
            let end_str = match end {
                Some(RangeBound::Integer(i)) => i.to_string(),
                Some(RangeBound::Char(c)) => format!("'{}'", c),
                None => String::new(),
            };
            let op = if end.is_none() {
                ".."
            } else if *inclusive {
                "..="
            } else {
                "..<"
            };
            format!("{}{}{}", start_str, op, end_str)
        },
        PatternKind::Struct {
            struct_name,
            fields,
            ..
        } => {
            let inner: Vec<String> = fields
                .iter()
                .map(|f| format!("{}: {}", f.field_name, describe_pattern(&f.pattern)))
                .collect();
            format!("{} {{ {} }}", struct_name, inner.join(", "))
        },
        PatternKind::Array {
            prefix,
            rest,
            suffix,
        } => {
            let mut parts: Vec<String> = prefix.iter().map(describe_pattern).collect();
            if rest.is_some() {
                parts.push("..".to_string());
            }
            parts.extend(suffix.iter().map(describe_pattern));
            format!("[{}]", parts.join(", "))
        },
        PatternKind::Or { alternatives } => {
            let parts: Vec<String> = alternatives.iter().map(describe_pattern).collect();
            parts.join(" | ")
        },
        PatternKind::At {
            name, subpattern, ..
        } => {
            format!("{} @ {}", name, describe_pattern(subpattern))
        },
        PatternKind::Rest => "..".to_string(),
        PatternKind::Error => "<error>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_semantic_tree::expr::LiteralValue;
    use kestrel_semantic_tree::pattern::Mutability;
    use kestrel_semantic_tree::symbol::local::LocalId;
    use kestrel_semantic_tree::ty::{IntBits, Ty};
    use kestrel_span::Span;

    fn test_span() -> Span {
        Span::new(0, 0..1)
    }

    fn int_ty() -> Ty {
        Ty::int(IntBits::I64, test_span())
    }

    #[test]
    fn test_wildcard_is_irrefutable() {
        let pattern = Pattern::wildcard(int_ty(), test_span());
        assert!(is_pattern_irrefutable(&pattern));
    }

    #[test]
    fn test_binding_is_irrefutable() {
        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            int_ty(),
            test_span(),
        );
        assert!(is_pattern_irrefutable(&pattern));
    }

    #[test]
    fn test_tuple_of_irrefutables_is_irrefutable() {
        let elements = vec![
            Pattern::wildcard(int_ty(), test_span()),
            Pattern::local(
                LocalId(0),
                Mutability::Immutable,
                "x".to_string(),
                int_ty(),
                test_span(),
            ),
        ];
        let tuple_ty = Ty::tuple(vec![int_ty(), int_ty()], test_span());
        let pattern = Pattern::tuple(elements, tuple_ty, test_span());
        assert!(is_pattern_irrefutable(&pattern));
    }

    #[test]
    fn test_literal_is_refutable() {
        let pattern = Pattern::literal(LiteralValue::Integer(42), int_ty(), test_span());
        assert!(!is_pattern_irrefutable(&pattern));
    }

    #[test]
    fn test_tuple_with_literal_is_refutable() {
        let elements = vec![
            Pattern::literal(LiteralValue::Integer(0), int_ty(), test_span()),
            Pattern::local(
                LocalId(0),
                Mutability::Immutable,
                "x".to_string(),
                int_ty(),
                test_span(),
            ),
        ];
        let tuple_ty = Ty::tuple(vec![int_ty(), int_ty()], test_span());
        let pattern = Pattern::tuple(elements, tuple_ty, test_span());
        assert!(!is_pattern_irrefutable(&pattern));
    }

    #[test]
    fn test_describe_binding() {
        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "foo".to_string(),
            int_ty(),
            test_span(),
        );
        assert_eq!(describe_pattern(&pattern), "foo");
    }

    #[test]
    fn test_describe_tuple() {
        let elements = vec![
            Pattern::local(
                LocalId(0),
                Mutability::Immutable,
                "a".to_string(),
                int_ty(),
                test_span(),
            ),
            Pattern::local(
                LocalId(1),
                Mutability::Immutable,
                "b".to_string(),
                int_ty(),
                test_span(),
            ),
        ];
        let tuple_ty = Ty::tuple(vec![int_ty(), int_ty()], test_span());
        let pattern = Pattern::tuple(elements, tuple_ty, test_span());
        assert_eq!(describe_pattern(&pattern), "(a, b)");
    }
}
