//! Refutable pattern checker for for-loop bindings.
//!
//! This analyzer validates that patterns used in for-loop bindings are irrefutable
//! (always match any value of the appropriate type). For-loops require irrefutable
//! patterns because the pattern must match every item from the iterator.
//!
//! # Examples
//!
//! ```ignore
//! // OK: Irrefutable patterns
//! for x in array { }
//! for (a, b) in pairs { }
//! for _ in range { }
//!
//! // ERROR: Refutable patterns
//! for 42 in numbers { }           // literal doesn't match all values
//! for .Some(x) in optionals { }   // doesn't match .None
//! for (0, b) in pairs { }         // literal in tuple
//! ```
//!
//! Note: For-loops desugar to `while let .Some(pattern) = iter.next()`, so
//! while-let naturally allows refutable patterns, but for-loops should not.

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_tree::expr::{ExprKind, Expression, IfCondition};
use kestrel_semantic_tree::pattern::{Pattern, PatternKind};
use kestrel_semantic_tree::ty::TyKind;

mod diagnostics;
use diagnostics::RefutableForLoopPatternError;

pub struct ForLoopPatternAnalyzer;

impl ForLoopPatternAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ForLoopPatternAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ForLoopPatternAnalyzer {
    fn name(&self) -> &'static str {
        "for_loop_pattern"
    }

    fn visit_expression(&mut self, expr: &Expression, ctx: &mut AnalysisContext) {
        // Only check while-let expressions that came from for-loop desugaring
        if let ExprKind::WhileLet {
            conditions,
            from_for_loop: true,
            ..
        } = &expr.kind
        {
            // For-loops desugar to: while let .Some(user_pattern) = iter.next()
            // We need to extract the user_pattern from inside the .Some(...)
            for condition in conditions {
                if let IfCondition::Let { pattern, .. } = condition {
                    // The pattern is .Some(user_pattern)
                    if let PatternKind::EnumVariant { bindings, .. } = &pattern.kind {
                        // Get the user's pattern from the first binding
                        if let Some(binding) = bindings.first() {
                            let user_pattern = &binding.pattern;
                            if !is_pattern_irrefutable(user_pattern) {
                                ctx.report(RefutableForLoopPatternError {
                                    pattern_span: user_pattern.span.clone(),
                                    pattern_description: describe_pattern(user_pattern),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Check if a pattern is irrefutable (always matches any value of its type).
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
        }

        // Range patterns are REFUTABLE - they don't cover all values
        PatternKind::Range { .. } => false,

        // Struct patterns are irrefutable if all field patterns are irrefutable
        PatternKind::Struct { fields, .. } => {
            fields.iter().all(|f| is_pattern_irrefutable(&f.pattern))
        }

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
    use kestrel_semantic_tree::expr::LiteralValue;

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
        }
        PatternKind::Literal { value } => match value {
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
            }
            LiteralValue::Bool(b) => b.to_string(),
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
        }
        PatternKind::Range {
            start,
            end,
            inclusive,
        } => {
            use kestrel_semantic_tree::pattern::RangeBound;
            let start_str = match start {
                RangeBound::Integer(i) => i.to_string(),
                RangeBound::Char(c) => format!("'{}'", c),
            };
            let end_str = match end {
                RangeBound::Integer(i) => i.to_string(),
                RangeBound::Char(c) => format!("'{}'", c),
            };
            let op = if *inclusive { "..=" } else { "..<" };
            format!("{}{}{}", start_str, op, end_str)
        }
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
        }
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
        }
        PatternKind::Or { alternatives } => {
            let parts: Vec<String> = alternatives.iter().map(describe_pattern).collect();
            parts.join(" | ")
        }
        PatternKind::At {
            name, subpattern, ..
        } => {
            format!("{} @ {}", name, describe_pattern(subpattern))
        }
        PatternKind::Rest => "..".to_string(),
        PatternKind::Error => "<error>".to_string(),
    }
}
