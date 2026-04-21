//! # Pattern Matching Analysis
//!
//! Implements Maranget's pattern matrix algorithm for analyzing and compiling
//! Kestrel match expressions. Used by two consumers:
//!
//! - **`kestrel-analyze`** — exhaustiveness diagnostics (E304–E307)
//! - **Execution graph lowering** — decision tree compilation for codegen
//!
//! # Capabilities
//!
//! - **Exhaustiveness** — does a match cover all possible values?
//! - **Redundancy** — is a pattern arm unreachable?
//! - **Overlap detection** — do range patterns overlap?
//! - **Decision tree compilation** — optimal control-flow IR for codegen
//!
//! # Entry Points
//!
//! ```ignore
//! // Diagnostics: check a match expression
//! let result = kestrel_pattern_matching::check_match(hir, query, scrutinee_ty, arms);
//! // result.is_exhaustive, result.redundant_arms, result.missing_patterns
//!
//! // Irrefutability: check a let/for pattern
//! let ok = kestrel_pattern_matching::is_irrefutable(hir, query, pat_id, ty);
//!
//! // Codegen: compile to decision tree
//! let tree = kestrel_pattern_matching::compile_decision_tree(hir, query, scrutinee_ty, arms);
//! ```
//!
//! # Architecture
//!
//! ```text
//! HirPat ──► flatten() ──► FlatPat ──► PatternMatrix ──► is_useful() ──► ExhaustivenessResult
//!                             │                              │
//!                             │                              ▼
//!                             └──────────────────────► compile() ──► DecisionTree
//! ```
//!
//! Six modules, each with a single responsibility:
//!
//! | Module | Role |
//! |--------|------|
//! | `constructor` | Constructor enum, TypeShape (type → constructor space) |
//! | `flat_pat` | Normalized pattern, HirPat→FlatPat conversion, decompose() |
//! | `matrix` | Pattern matrix, specialize (S(c,P)), default (D(P)) |
//! | `usefulness` | Core Maranget algorithm, ExhaustivenessResult |
//! | `witness` | Example values for "missing pattern: `.None`" messages |
//! | `decision_tree` | Decision tree compilation, binding extraction |
//!
//! # Deduplication Invariants
//!
//! Each piece of logic exists in exactly one place:
//!
//! - **Pattern decomposition** — `FlatPat::decompose()` (used by matrix + decision tree)
//! - **Constructor field types** — `Constructor::field_types()` (used by matrix + decision tree)
//! - **Constructor matching** — `Constructor::matches()` (used by decompose + matrix)
//! - **Type classification** — `TypeShape::classify()` (used by usefulness + irrefutability)
//!
//! # References
//!
//! - Luc Maranget, "Warnings for pattern matching" (JFP 2007)
//! - Luc Maranget, "Compiling Pattern Matching to Good Decision Trees" (2008)

pub mod constructor;
pub mod decision_tree;
pub mod flat_pat;
pub mod matrix;
pub mod usefulness;
pub mod witness;

// Re-export key types for convenient use
pub use decision_tree::DecisionTree;
pub use usefulness::ExhaustivenessResult;
pub use witness::Witness;

use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::body::*;
use kestrel_type_infer::result::ResolvedTy;

/// Check exhaustiveness and redundancy for a match expression.
pub fn check_match(
    hir: &HirBody,
    query: &QueryContext<'_>,
    root: Entity,
    scrutinee_ty: &ResolvedTy,
    arms: &[HirMatchArm],
) -> ExhaustivenessResult {
    usefulness::check_match(hir, query, root, scrutinee_ty, arms)
}

/// Type-aware irrefutability check.
///
/// Returns true if the pattern is guaranteed to match any value of `ty`.
/// Smarter than a simple syntactic check:
/// - Knows Bool has two constructors (not one)
/// - Knows single-variant enums are irrefutable
/// - Knows Never has zero constructors
pub fn is_irrefutable(
    hir: &HirBody,
    query: &QueryContext<'_>,
    root: Entity,
    pat_id: HirPatId,
    ty: &ResolvedTy,
) -> bool {
    let flat = flat_pat::flatten(hir, query, pat_id, ty);
    check_irrefutable(&flat, query, root, ty)
}

/// Recursive irrefutability check on a FlatPat.
fn check_irrefutable(
    pat: &flat_pat::FlatPat,
    query: &QueryContext<'_>,
    root: Entity,
    ty: &ResolvedTy,
) -> bool {
    match pat {
        flat_pat::FlatPat::Wildcard => true,

        flat_pat::FlatPat::Ctor { ctor, children } => {
            // Check if this constructor is the only one for the type
            let all = constructor::Constructor::all_for_type(query, root, ty);
            let is_sole_ctor = all
                .as_ref()
                .is_some_and(|ctors| ctors.len() == 1 && ctors[0] == *ctor);

            if !is_sole_ctor {
                return false;
            }

            // Constructor is sole — check all children recursively
            let field_types = ctor.field_types(query, ty);
            children.iter().enumerate().all(|(i, child)| {
                let child_ty = field_types.get(i).unwrap_or(&ResolvedTy::Error);
                check_irrefutable(child, query, root, child_ty)
            })
        },

        // Or-pattern is irrefutable if ANY alternative is irrefutable
        flat_pat::FlatPat::Or(alts) => alts
            .iter()
            .any(|alt| check_irrefutable(alt, query, root, ty)),
    }
}

/// Compile patterns into a decision tree for codegen.
pub fn compile_decision_tree(
    hir: &HirBody,
    query: &QueryContext<'_>,
    root: Entity,
    scrutinee_ty: &ResolvedTy,
    arms: &[HirMatchArm],
) -> DecisionTree {
    let flat_pats: Vec<_> = arms
        .iter()
        .map(|arm| flat_pat::flatten(hir, query, arm.pattern, scrutinee_ty))
        .collect();
    let arm_pat_ids: Vec<_> = arms.iter().map(|arm| arm.pattern).collect();
    let has_guards: Vec<_> = arms.iter().map(|arm| arm.guard.is_some()).collect();

    decision_tree::compile(
        hir,
        query,
        root,
        &flat_pats,
        &arm_pat_ids,
        scrutinee_ty,
        &has_guards,
    )
}
