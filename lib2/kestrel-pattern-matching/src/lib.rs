//! Pattern matching analysis for Kestrel (lib2).
//!
//! Implements Maranget's pattern matrix algorithm for:
//! - **Exhaustiveness**: does a match cover all possible values?
//! - **Redundancy**: is a pattern arm unreachable?
//! - **Overlap**: do range patterns overlap?
//! - **Decision tree compilation**: efficient codegen IR for pattern matching
//!
//! Used by both the analyzer crate (diagnostics) and execution graph
//! lowering (codegen).
//!
//! # Architecture
//!
//! - `constructor` — Constructor enum and TypeShape for type classification
//! - `flat_pat` — Normalized pattern representation with single decompose function
//! - `matrix` — Pattern matrix with specialize/default operations
//! - `usefulness` — Core Maranget algorithm
//! - `witness` — Example values for error messages
//! - `decision_tree` — Compilation to decision trees for codegen
//!
//! All pattern decomposition happens in `FlatPat::decompose` (one function),
//! all type classification in `TypeShape::classify` (one function), and
//! all constructor matching in `Constructor::matches` (one function).

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

use kestrel_hecs::QueryContext;
use kestrel_hir::body::*;
use kestrel_type_infer::result::ResolvedTy;

/// Check exhaustiveness and redundancy for a match expression.
pub fn check_match(
    hir: &HirBody,
    query: &QueryContext<'_>,
    scrutinee_ty: &ResolvedTy,
    arms: &[HirMatchArm],
) -> ExhaustivenessResult {
    usefulness::check_match(hir, query, scrutinee_ty, arms)
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
    pat_id: HirPatId,
    ty: &ResolvedTy,
) -> bool {
    let flat = flat_pat::flatten(hir, query, pat_id, ty);
    check_irrefutable(&flat, query, ty)
}

/// Recursive irrefutability check on a FlatPat.
fn check_irrefutable(pat: &flat_pat::FlatPat, query: &QueryContext<'_>, ty: &ResolvedTy) -> bool {
    match pat {
        flat_pat::FlatPat::Wildcard => true,

        flat_pat::FlatPat::Ctor { ctor, children } => {
            // Check if this constructor is the only one for the type
            let all = constructor::Constructor::all_for_type(query, ty);
            let is_sole_ctor = all.as_ref().is_some_and(|ctors| {
                ctors.len() == 1 && ctors[0] == *ctor
            });

            if !is_sole_ctor {
                return false;
            }

            // Constructor is sole — check all children recursively
            let field_types = ctor.field_types(query, ty);
            children.iter().enumerate().all(|(i, child)| {
                let child_ty = field_types.get(i).unwrap_or(&ResolvedTy::Error);
                check_irrefutable(child, query, child_ty)
            })
        }

        // Or-pattern is irrefutable if ANY alternative is irrefutable
        flat_pat::FlatPat::Or(alts) => {
            alts.iter().any(|alt| check_irrefutable(alt, query, ty))
        }
    }
}

/// Compile patterns into a decision tree for codegen.
pub fn compile_decision_tree(
    hir: &HirBody,
    query: &QueryContext<'_>,
    scrutinee_ty: &ResolvedTy,
    arms: &[HirMatchArm],
) -> DecisionTree {
    let flat_pats: Vec<_> = arms
        .iter()
        .map(|arm| flat_pat::flatten(hir, query, arm.pattern, scrutinee_ty))
        .collect();
    let arm_pat_ids: Vec<_> = arms.iter().map(|arm| arm.pattern).collect();
    let has_guards: Vec<_> = arms.iter().map(|arm| arm.guard.is_some()).collect();

    decision_tree::compile(hir, query, &flat_pats, &arm_pat_ids, scrutinee_ty, &has_guards)
}
