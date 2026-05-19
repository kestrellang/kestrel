//! # Match Pattern Analyzer
//!
//! Validates patterns inside user-written `match` expressions (and the
//! scrutinee-checked positions of `if let` / `while let` / `guard let`).
//! Covers duplicate bindings, float literals in patterns, enum/tuple arity,
//! unknown enum cases, and or-pattern binding consistency.
//!
//! Param-pattern checks (function/closure parameter destructuring) live in
//! `param_pattern.rs`; this analyzer skips `MatchSource::ParamDestructure`
//! to avoid double-reporting.
//!
//! ## Diagnostics
//!
//! ### E310 — `duplicate_match_binding` (Error, Correctness)
//!
//! **Message:** "duplicate binding '{name}' in pattern"
//!
//! **Labels:**
//! - Primary: the second binding site
//!   - Span source: `util::pat_span` on the duplicate `HirPatId`
//!   - Message: "'{name}' already bound in this pattern"
//!
//! **Notes:** (none)
//!
//! ### E311 — `float_literal_in_pattern` (Error, Correctness)
//!
//! **Message:** "float literal in pattern"
//!
//! **Labels:**
//! - Primary: the literal
//!   - Span source: `util::pat_span` on the `HirPat::Literal` with float value
//!   - Message: "float equality is unreliable; match on an integer or range"
//!
//! **Notes:** (none)
//!
//! ### E312 — `unknown_enum_case` (Error, Correctness)
//!
//! **Message:** "unknown enum case '{name}' on type '{ty}'"
//!
//! **Labels:**
//! - Primary: the implicit-variant pattern
//!   - Span source: `util::pat_span` on the `HirPat::ImplicitVariant`
//!   - Message: "no case '{name}' on '{ty}'"
//!
//! **Notes:** (none)
//!
//! ### E313 — `wrong_variant_arity` (Error, Correctness)
//!
//! **Message:** "variant '{name}' takes {expected} argument(s), got {got}"
//!
//! **Labels:**
//! - Primary: the variant pattern
//!   - Span source: `util::pat_span` on the `HirPat::Variant`/`ImplicitVariant`
//!   - Message: "wrong arity for '{name}'"
//!
//! **Notes:** (none)
//!
//! ### E314 — `wrong_tuple_arity_in_pattern` (Error, Correctness)
//!
//! **Message:** "tuple pattern has {pat} elements but type has {ty}"
//!
//! **Labels:**
//! - Primary: the tuple pattern
//!   - Span source: `util::pat_span` on the `HirPat::Tuple`
//!   - Message: "arity mismatch"
//!
//! **Notes:** (none)
//!
//! ### E315 — `or_pattern_inconsistent_bindings` (Error, Correctness)
//!
//! **Message:** "inconsistent bindings across or-pattern alternatives"
//!
//! **Labels:**
//! - Primary: the or-pattern
//!   - Span source: `util::pat_span` on the `HirPat::Or`
//!   - Message: "variable '{name}' is not bound in all alternatives"
//!
//! **Notes:** (none)

use std::collections::BTreeSet;

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Name, NodeKind};
use kestrel_hecs::Entity;
use kestrel_hir::body::*;
use kestrel_hir_lower::LowerCallableTypes;
use kestrel_type_infer::result::ResolvedTy;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E310",
        name: "duplicate_match_binding",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E311",
        name: "float_literal_in_pattern",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E312",
        name: "unknown_enum_case",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E313",
        name: "wrong_variant_arity",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E314",
        name: "wrong_tuple_arity_in_pattern",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E315",
        name: "or_pattern_inconsistent_bindings",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E316",
        name: "many_arm_string_match",
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

pub struct MatchPatternAnalyzer;

impl Describe for MatchPatternAnalyzer {
    fn id(&self) -> &'static str {
        "match_pattern"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for MatchPatternAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        for (match_id, expr) in cx.hir.exprs.iter() {
            let HirExpr::Match {
                scrutinee,
                arms,
                source,
                ..
            } = expr
            else {
                continue;
            };

            // Param-destructure matches are synthetic — their patterns mirror
            // the function signature and are checked by `param_pattern`.
            if matches!(source, MatchSource::ParamDestructure) {
                continue;
            }

            let scrut_ty = cx.typed.expr_types.get(scrutinee);

            for arm in arms {
                // Float literals, variant arity, tuple arity, unknown cases.
                walk_pat(cx, arm.pattern, scrut_ty, &mut diags);
                // Duplicate bindings within a single arm pattern.
                check_duplicate_bindings(cx, arm.pattern, &mut diags);
                // Or-pattern binding-name consistency.
                check_or_consistency(cx, arm.pattern, &mut diags);
            }

            // E316: warn on byte-equality-heavy string matches. Each literal
            // arm becomes a `Matchable.matches` call at runtime, so a long
            // chain is `O(arms × len)` — past a small threshold an `if/else if`
            // chain is just as readable and conveys the cost up front.
            if scrut_ty.is_some_and(|ty| is_string_type(cx, ty)) {
                let lit_count: usize = arms
                    .iter()
                    .map(|arm| count_string_literal_alts(cx, arm.pattern))
                    .sum();
                if lit_count > MANY_STRING_ARM_THRESHOLD {
                    let d = descriptor("E316");
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: d.id,
                        severity: d.default_severity,
                        message: format!(
                            "match on `String` with {} literal arms does byte-equality \
                             per arm (O(arms × len))",
                            lit_count
                        ),
                        labels: vec![DiagLabel {
                            span: util::expr_span(cx.hir, match_id),
                            message: "consider an `if`/`else if` chain on `==` instead".into(),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                }
            }
        }

        diags
    }
}

/// E316 fires once a match has more than this many string-literal alternatives.
/// Chosen to allow small finite-tag lookups (status codes, tokens) but flag
/// switch-table-style use where a pre-built map or `if`-chain communicates
/// intent better.
const MANY_STRING_ARM_THRESHOLD: usize = 4;

/// Returns true if `ty` is the stdlib `String` type. Match-by-name uses the
/// entity's `Name` component (same idiom as `describe_named`/`is_named_enum`).
fn is_string_type(cx: &BodyContext<'_>, ty: &ResolvedTy) -> bool {
    let ResolvedTy::Named { entity, .. } = ty else {
        return false;
    };
    cx.query
        .get::<Name>(*entity)
        .is_some_and(|n| n.0 == "String")
}

/// Count the number of *string-literal* leaves directly under an arm pattern.
/// Descends into or-patterns and `@`-bindings (each branch the user wrote
/// counts as one runtime byte-equality call); other shapes contribute 0.
fn count_string_literal_alts(cx: &BodyContext<'_>, pat: HirPatId) -> usize {
    match &cx.hir.pats[pat] {
        HirPat::Literal {
            value: HirLiteral::String { .. },
            ..
        } => 1,
        HirPat::Or { alternatives, .. } => alternatives
            .iter()
            .map(|&p| count_string_literal_alts(cx, p))
            .sum(),
        HirPat::At { subpattern, .. } => count_string_literal_alts(cx, *subpattern),
        _ => 0,
    }
}

// ===== Walk: arity / float / unknown-case =====

/// Recurse into a pattern, paired with the scrutinee's resolved type when
/// known. Emits E311 (float literal), E312 (unknown case), E313 (variant
/// arity), E314 (tuple arity).
fn walk_pat(
    cx: &BodyContext<'_>,
    pat_id: HirPatId,
    expected: Option<&ResolvedTy>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match &cx.hir.pats[pat_id] {
        HirPat::Literal {
            value: HirLiteral::Float(_),
            ..
        } => {
            let d = descriptor("E311");
            diags.push(AnalyzeDiagnostic {
                descriptor_id: d.id,
                severity: d.default_severity,
                message: "float literal in pattern".into(),
                labels: vec![DiagLabel {
                    span: util::pat_span(cx.hir, pat_id),
                    message: "float equality is unreliable; match on an integer or range".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        },

        HirPat::Tuple {
            prefix,
            has_rest,
            suffix,
            ..
        } => {
            let tuple_elems = match expected {
                Some(ResolvedTy::Tuple(elems)) => Some(elems.as_slice()),
                _ => None,
            };
            if let Some(elems) = tuple_elems {
                let pat_count = prefix.len() + suffix.len();
                let mismatch = if *has_rest {
                    pat_count > elems.len()
                } else {
                    pat_count != elems.len()
                };
                if mismatch {
                    emit_tuple_arity(cx, pat_id, pat_count, elems.len(), diags);
                }
                // Recurse: pair prefix with first N elements, suffix with last M.
                for (&p, t) in prefix.iter().zip(elems.iter()) {
                    walk_pat(cx, p, Some(t), diags);
                }
                let suffix_start = elems.len().saturating_sub(suffix.len());
                for (&p, t) in suffix.iter().zip(elems[suffix_start..].iter()) {
                    walk_pat(cx, p, Some(t), diags);
                }
            } else {
                for &p in prefix.iter().chain(suffix.iter()) {
                    walk_pat(cx, p, None, diags);
                }
            }
        },

        HirPat::Variant { entity, args, .. } => {
            let expected_arity = variant_arity(cx, *entity);
            if args.len() != expected_arity {
                let name = cx
                    .query
                    .get::<Name>(*entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_else(|| "<case>".into());
                emit_variant_arity(cx, pat_id, &name, expected_arity, args.len(), diags);
            }
            for arg in args {
                walk_pat(cx, arg.pattern, None, diags);
            }
        },

        HirPat::ImplicitVariant { name, args, .. } => {
            // Skip case validation entirely when the parser recovered a
            // missing name — the parse error is already on screen.
            let Some(name_str) = name.as_str() else {
                for arg in args {
                    walk_pat(cx, arg.pattern, None, diags);
                }
                return;
            };
            // Try to resolve the case against the scrutinee's enum type.
            let case_entity = expected.and_then(|ty| lookup_enum_case(cx, ty, name_str));

            match (expected, case_entity) {
                (Some(ty), None) if is_named_enum(cx, ty) => {
                    // Scrutinee is a known enum, but this case name doesn't exist.
                    let ty_desc = describe_named(cx, ty);
                    let d = descriptor("E312");
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: d.id,
                        severity: d.default_severity,
                        message: format!("unknown enum case '{}' on type '{}'", name_str, ty_desc),
                        labels: vec![DiagLabel {
                            span: util::pat_span(cx.hir, pat_id),
                            message: format!("no case '{}' on '{}'", name_str, ty_desc),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                },
                (_, Some(case)) => {
                    let expected_arity = variant_arity(cx, case);
                    if args.len() != expected_arity {
                        emit_variant_arity(cx, pat_id, name_str, expected_arity, args.len(), diags);
                    }
                },
                _ => {
                    // Scrutinee not resolved to an enum — defer to solver.
                },
            }

            for arg in args {
                walk_pat(cx, arg.pattern, None, diags);
            }
        },

        HirPat::Struct { fields, .. } => {
            for field in fields {
                if let Some(pat) = field.pattern {
                    walk_pat(cx, pat, None, diags);
                }
            }
        },

        HirPat::Array { prefix, suffix, .. } => {
            for &p in prefix.iter().chain(suffix.iter()) {
                walk_pat(cx, p, None, diags);
            }
        },

        HirPat::Or { alternatives, .. } => {
            for &alt in alternatives {
                walk_pat(cx, alt, expected, diags);
            }
        },

        HirPat::At { subpattern, .. } => {
            walk_pat(cx, *subpattern, expected, diags);
        },

        HirPat::Wildcard { .. }
        | HirPat::Binding { .. }
        | HirPat::Literal { .. }
        | HirPat::Range { .. }
        | HirPat::Error { .. } => {},
    }
}

/// Number of payload arguments an enum case takes. A case without a
/// `Callable` component is a unit case (arity 0).
fn variant_arity(cx: &BodyContext<'_>, case: Entity) -> usize {
    cx.query
        .query(LowerCallableTypes {
            entity: case,
            root: cx.root,
        })
        .map(|params| params.len())
        .unwrap_or(0)
}

/// Look up an enum case by name on a resolved enum type. Returns the case
/// entity if found, `None` otherwise (caller distinguishes "unknown case"
/// from "scrutinee isn't an enum").
fn lookup_enum_case(cx: &BodyContext<'_>, ty: &ResolvedTy, name: &str) -> Option<Entity> {
    let ResolvedTy::Named { entity, .. } = ty else {
        return None;
    };
    if cx.query.get::<NodeKind>(*entity) != Some(&NodeKind::Enum) {
        return None;
    }
    cx.query
        .children_of(*entity)
        .iter()
        .copied()
        .find(|&child| {
            cx.query.get::<NodeKind>(child) == Some(&NodeKind::EnumCase)
                && cx.query.get::<Name>(child).is_some_and(|n| n.0 == name)
        })
}

fn is_named_enum(cx: &BodyContext<'_>, ty: &ResolvedTy) -> bool {
    let ResolvedTy::Named { entity, .. } = ty else {
        return false;
    };
    cx.query.get::<NodeKind>(*entity) == Some(&NodeKind::Enum)
}

fn describe_named(cx: &BodyContext<'_>, ty: &ResolvedTy) -> String {
    match ty {
        ResolvedTy::Named { entity, .. } => util::entity_name(cx.query, *entity),
        _ => "?".into(),
    }
}

fn emit_tuple_arity(
    cx: &BodyContext<'_>,
    pat_id: HirPatId,
    pat_count: usize,
    ty_count: usize,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let d = descriptor("E314");
    diags.push(AnalyzeDiagnostic {
        descriptor_id: d.id,
        severity: d.default_severity,
        message: format!(
            "tuple pattern arity mismatch: pattern has {} elements but type has {}",
            pat_count, ty_count
        ),
        labels: vec![DiagLabel {
            span: util::pat_span(cx.hir, pat_id),
            message: "arity mismatch".into(),
            is_primary: true,
        }],
        notes: vec![],
    });
}

fn emit_variant_arity(
    cx: &BodyContext<'_>,
    pat_id: HirPatId,
    name: &str,
    expected: usize,
    got: usize,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let d = descriptor("E313");
    diags.push(AnalyzeDiagnostic {
        descriptor_id: d.id,
        severity: d.default_severity,
        message: format!(
            "variant '{}' takes {} argument(s), got {}",
            name, expected, got
        ),
        labels: vec![DiagLabel {
            span: util::pat_span(cx.hir, pat_id),
            message: format!("wrong arity for '{}'", name),
            is_primary: true,
        }],
        notes: vec![],
    });
}

// ===== Duplicate bindings =====

/// Flag a name bound more than once in a single pattern. Or-pattern
/// alternatives are separate scopes (the same name across alternatives
/// refers to one logical binding), so each alternative is walked with its
/// own seen-set.
fn check_duplicate_bindings(
    cx: &BodyContext<'_>,
    pat_id: HirPatId,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let mut seen = BTreeSet::new();
    walk_dup(cx, pat_id, &mut seen, diags);
}

fn walk_dup(
    cx: &BodyContext<'_>,
    pat_id: HirPatId,
    seen: &mut BTreeSet<String>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match &cx.hir.pats[pat_id] {
        HirPat::Binding { local, .. } => {
            let name = &cx.hir.locals[*local].name;
            if name == "_" {
                return;
            }
            if !seen.insert(name.clone()) {
                emit_duplicate(cx, pat_id, name, diags);
            }
        },
        HirPat::At {
            binding,
            subpattern,
            ..
        } => {
            let name = cx.hir.locals[*binding].name.clone();
            if name != "_" && !seen.insert(name.clone()) {
                emit_duplicate(cx, pat_id, &name, diags);
            }
            walk_dup(cx, *subpattern, seen, diags);
        },
        HirPat::Tuple { prefix, suffix, .. } | HirPat::Array { prefix, suffix, .. } => {
            for &p in prefix.iter().chain(suffix.iter()) {
                walk_dup(cx, p, seen, diags);
            }
        },
        HirPat::Variant { args, .. } | HirPat::ImplicitVariant { args, .. } => {
            for arg in args {
                walk_dup(cx, arg.pattern, seen, diags);
            }
        },
        HirPat::Struct { fields, .. } => {
            for field in fields {
                if let Some(sub) = field.pattern {
                    walk_dup(cx, sub, seen, diags);
                }
            }
        },
        HirPat::Or { alternatives, .. } => {
            // Each alternative has its own seen-set; bindings from one
            // alternative don't collide with another.
            for &alt in alternatives {
                let mut alt_seen = seen.clone();
                walk_dup(cx, alt, &mut alt_seen, diags);
            }
        },
        // Also catch named rest (`..name`) in array patterns.
        HirPat::Wildcard { .. }
        | HirPat::Literal { .. }
        | HirPat::Range { .. }
        | HirPat::Error { .. } => {},
    }
    // Named array rest — the rest local is separate from prefix/suffix arena.
    if let HirPat::Array {
        rest: Some(Some(local)),
        ..
    } = &cx.hir.pats[pat_id]
    {
        let name = cx.hir.locals[*local].name.clone();
        if name != "_" && !seen.insert(name) {
            emit_duplicate(cx, pat_id, &cx.hir.locals[*local].name, diags);
        }
    }
}

fn emit_duplicate(
    cx: &BodyContext<'_>,
    pat_id: HirPatId,
    name: &str,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let d = descriptor("E310");
    diags.push(AnalyzeDiagnostic {
        descriptor_id: d.id,
        severity: d.default_severity,
        message: format!("duplicate binding '{}' in pattern", name),
        labels: vec![DiagLabel {
            span: util::pat_span(cx.hir, pat_id),
            message: format!("'{}' already bound in this pattern", name),
            is_primary: true,
        }],
        notes: vec![],
    });
}

// ===== Or-pattern binding consistency =====

/// Each alternative of an `or`-pattern must bind the same set of names, so
/// the arm body can reference them unambiguously. Recurses so nested
/// or-patterns are checked too.
fn check_or_consistency(
    cx: &BodyContext<'_>,
    pat_id: HirPatId,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    if let HirPat::Or { alternatives, .. } = &cx.hir.pats[pat_id]
        && alternatives.len() >= 2
    {
        let first = collect_bindings(cx, alternatives[0]);
        for &alt in &alternatives[1..] {
            let other = collect_bindings(cx, alt);
            let missing: Vec<&str> = first
                .difference(&other)
                .chain(other.difference(&first))
                .map(|s| s.as_str())
                .collect();
            if let Some(first_missing) = missing.first() {
                let d = descriptor("E315");
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: d.id,
                    severity: d.default_severity,
                    message: "inconsistent bindings across or-pattern alternatives".into(),
                    labels: vec![DiagLabel {
                        span: util::pat_span(cx.hir, pat_id),
                        message: format!(
                            "variable '{}' is not bound in all alternatives",
                            first_missing
                        ),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
                break;
            }
        }
    }
    // Recurse into sub-patterns to catch nested or-patterns.
    for child in pat_children(&cx.hir.pats[pat_id]) {
        check_or_consistency(cx, child, diags);
    }
}

fn collect_bindings(cx: &BodyContext<'_>, pat_id: HirPatId) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    gather_bindings(cx, pat_id, &mut names);
    names
}

fn gather_bindings(cx: &BodyContext<'_>, pat_id: HirPatId, out: &mut BTreeSet<String>) {
    match &cx.hir.pats[pat_id] {
        HirPat::Binding { local, .. } => {
            let name = &cx.hir.locals[*local].name;
            if name != "_" {
                out.insert(name.clone());
            }
        },
        HirPat::At {
            binding,
            subpattern,
            ..
        } => {
            let name = &cx.hir.locals[*binding].name;
            if name != "_" {
                out.insert(name.clone());
            }
            gather_bindings(cx, *subpattern, out);
        },
        HirPat::Array {
            prefix,
            rest,
            suffix,
            ..
        } => {
            for &p in prefix.iter().chain(suffix.iter()) {
                gather_bindings(cx, p, out);
            }
            if let Some(Some(local)) = rest {
                let name = &cx.hir.locals[*local].name;
                if name != "_" {
                    out.insert(name.clone());
                }
            }
        },
        _ => {
            for child in pat_children(&cx.hir.pats[pat_id]) {
                gather_bindings(cx, child, out);
            }
        },
    }
}

fn pat_children(pat: &HirPat) -> Vec<HirPatId> {
    match pat {
        HirPat::Tuple { prefix, suffix, .. } | HirPat::Array { prefix, suffix, .. } => {
            prefix.iter().chain(suffix.iter()).copied().collect()
        },
        HirPat::Variant { args, .. } | HirPat::ImplicitVariant { args, .. } => {
            args.iter().map(|a| a.pattern).collect()
        },
        HirPat::Struct { fields, .. } => fields.iter().filter_map(|f| f.pattern).collect(),
        HirPat::Or { alternatives, .. } => alternatives.clone(),
        HirPat::At { subpattern, .. } => vec![*subpattern],
        HirPat::Wildcard { .. }
        | HirPat::Binding { .. }
        | HirPat::Literal { .. }
        | HirPat::Range { .. }
        | HirPat::Error { .. } => vec![],
    }
}

/// Public: true if this pattern contains any node that this analyzer will
/// flag as structurally invalid — unknown enum case, variant-arity or
/// tuple-arity mismatch, float literal in a pattern, or inconsistent
/// or-pattern bindings. Exhaustiveness uses this to skip arms whose
/// structure is already broken so the usefulness pass doesn't cascade
/// (spurious E305 "non-exhaustive" / E306 "unreachable" on top of the
/// real error).
pub fn is_invalid(cx: &BodyContext<'_>, pat_id: HirPatId, expected: Option<&ResolvedTy>) -> bool {
    match &cx.hir.pats[pat_id] {
        HirPat::Tuple {
            prefix,
            has_rest,
            suffix,
            ..
        } => {
            let pat_count = prefix.len() + suffix.len();
            if let Some(ResolvedTy::Tuple(elems)) = expected {
                let arity_bad = if *has_rest {
                    pat_count > elems.len()
                } else {
                    pat_count != elems.len()
                };
                if arity_bad {
                    return true;
                }
                if prefix
                    .iter()
                    .zip(elems.iter())
                    .any(|(&p, t)| is_invalid(cx, p, Some(t)))
                {
                    return true;
                }
                let suffix_start = elems.len().saturating_sub(suffix.len());
                if suffix
                    .iter()
                    .zip(elems[suffix_start..].iter())
                    .any(|(&p, t)| is_invalid(cx, p, Some(t)))
                {
                    return true;
                }
                false
            } else {
                prefix
                    .iter()
                    .chain(suffix.iter())
                    .any(|&p| is_invalid(cx, p, None))
            }
        },
        HirPat::Variant { entity, args, .. } => {
            args.len() != variant_arity(cx, *entity)
                || args.iter().any(|a| is_invalid(cx, a.pattern, None))
        },
        HirPat::ImplicitVariant { name, args, .. } => {
            // Missing case name: parser already reported the gap; suppress
            // every downstream analyzer (exhaustiveness, redundancy, …).
            if name.is_missing() {
                return true;
            }
            let case = expected.and_then(|ty| lookup_enum_case(cx, ty, name.as_str_or_empty()));
            let self_bad = match (expected, case) {
                (Some(ty), None) => is_named_enum(cx, ty),
                (_, Some(c)) => args.len() != variant_arity(cx, c),
                _ => false,
            };
            self_bad || args.iter().any(|a| is_invalid(cx, a.pattern, None))
        },
        HirPat::Struct { fields, .. } => fields
            .iter()
            .any(|f| f.pattern.is_some_and(|p| is_invalid(cx, p, None))),
        HirPat::Array { prefix, suffix, .. } => prefix
            .iter()
            .chain(suffix.iter())
            .any(|&p| is_invalid(cx, p, None)),
        HirPat::Or { alternatives, .. } => {
            has_inconsistent_or(cx.hir, pat_id)
                || alternatives.iter().any(|&a| is_invalid(cx, a, expected))
        },
        HirPat::At { subpattern, .. } => is_invalid(cx, *subpattern, expected),
        HirPat::Literal {
            value: HirLiteral::Float(_),
            ..
        } => true,
        HirPat::Wildcard { .. }
        | HirPat::Binding { .. }
        | HirPat::Literal { .. }
        | HirPat::Range { .. }
        | HirPat::Error { .. } => false,
    }
}

/// Public: true if any `or`-pattern in this tree binds an inconsistent set
/// of names across its alternatives. Exhaustiveness uses this to skip
/// arms whose pattern is definitionally broken — otherwise the usefulness
/// pass would flag the following arm as unreachable (spurious E306 on
/// top of E315).
pub fn has_inconsistent_or(hir: &HirBody, pat_id: HirPatId) -> bool {
    match &hir.pats[pat_id] {
        HirPat::Or { alternatives, .. } => {
            if alternatives.len() >= 2 {
                let first = collect_bindings_pure(hir, alternatives[0]);
                for &alt in &alternatives[1..] {
                    let other = collect_bindings_pure(hir, alt);
                    if first != other {
                        return true;
                    }
                }
            }
            alternatives.iter().any(|&a| has_inconsistent_or(hir, a))
        },
        HirPat::Tuple { prefix, suffix, .. } | HirPat::Array { prefix, suffix, .. } => prefix
            .iter()
            .chain(suffix.iter())
            .any(|&p| has_inconsistent_or(hir, p)),
        HirPat::Variant { args, .. } | HirPat::ImplicitVariant { args, .. } => {
            args.iter().any(|a| has_inconsistent_or(hir, a.pattern))
        },
        HirPat::Struct { fields, .. } => fields
            .iter()
            .any(|f| f.pattern.is_some_and(|p| has_inconsistent_or(hir, p))),
        HirPat::At { subpattern, .. } => has_inconsistent_or(hir, *subpattern),
        HirPat::Wildcard { .. }
        | HirPat::Binding { .. }
        | HirPat::Literal { .. }
        | HirPat::Range { .. }
        | HirPat::Error { .. } => false,
    }
}

fn collect_bindings_pure(hir: &HirBody, pat_id: HirPatId) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    gather_bindings_pure(hir, pat_id, &mut out);
    out
}

fn gather_bindings_pure(hir: &HirBody, pat_id: HirPatId, out: &mut BTreeSet<String>) {
    match &hir.pats[pat_id] {
        HirPat::Binding { local, .. } => {
            let name = &hir.locals[*local].name;
            if name != "_" {
                out.insert(name.clone());
            }
        },
        HirPat::At {
            binding,
            subpattern,
            ..
        } => {
            let name = &hir.locals[*binding].name;
            if name != "_" {
                out.insert(name.clone());
            }
            gather_bindings_pure(hir, *subpattern, out);
        },
        HirPat::Array {
            prefix,
            rest,
            suffix,
            ..
        } => {
            for &p in prefix.iter().chain(suffix.iter()) {
                gather_bindings_pure(hir, p, out);
            }
            if let Some(Some(local)) = rest {
                let name = &hir.locals[*local].name;
                if name != "_" {
                    out.insert(name.clone());
                }
            }
        },
        HirPat::Tuple { prefix, suffix, .. } => {
            for &p in prefix.iter().chain(suffix.iter()) {
                gather_bindings_pure(hir, p, out);
            }
        },
        HirPat::Variant { args, .. } | HirPat::ImplicitVariant { args, .. } => {
            for a in args {
                gather_bindings_pure(hir, a.pattern, out);
            }
        },
        HirPat::Struct { fields, .. } => {
            for f in fields {
                if let Some(p) = f.pattern {
                    gather_bindings_pure(hir, p, out);
                }
            }
        },
        HirPat::Or { alternatives, .. } => {
            // Union over alternatives — consistency is checked elsewhere.
            for &a in alternatives {
                gather_bindings_pure(hir, a, out);
            }
        },
        HirPat::Wildcard { .. }
        | HirPat::Literal { .. }
        | HirPat::Range { .. }
        | HirPat::Error { .. } => {},
    }
}
