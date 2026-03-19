//! # Decision Tree Compilation (Maranget 2008)
//!
//! Compiles pattern matrices into decision trees — an IR between patterns
//! and control flow that codegen (execution graph lowering) can consume.
//!
//! Uses the shared `PatternMatrix::specialize` and `FlatPat::decompose`,
//! so there is no duplicated specialization logic.
//!
//! ## Tree Structure
//!
//! ```text
//! match x {
//!     .None => 0
//!     .Some(0) => 1
//!     .Some(n) => n
//! }
//! ```
//!
//! Compiles to:
//!
//! ```text
//! Switch(x, [
//!     None => Success(arm 0),
//!     Some => Switch(x.Some.0, [
//!         0 => Success(arm 1),
//!         _ => Success(arm 2, [n = x.Some.0])
//!     ])
//! ])
//! ```
//!
//! ## Variants
//!
//! - `Switch` — test a value, branch by constructor
//! - `Success` — matched an arm, extract bindings
//! - `Guard` — test a guard condition, branch on pass/fail
//! - `Failure` — unreachable if exhaustiveness passed
//!
//! ## Column Selection
//!
//! Uses the "necessity" heuristic: prefer the column with the most distinct
//! constructors, minimizing the total number of tests.

use kestrel_hecs::QueryContext;
use kestrel_hir::body::*;
use kestrel_hir::res::LocalId;
use kestrel_type_infer::result::ResolvedTy;

use super::constructor::Constructor;
use super::flat_pat::FlatPat;
use super::matrix::{PatternMatrix, PatternRow};

// ===== Output types =====

/// A path from the scrutinee to a sub-value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathElement {
    /// Struct field access
    Field(String),
    /// Tuple/array index
    Index(usize),
    /// Enum variant downcast
    Downcast(String),
}

/// Path from the scrutinee root to a sub-value.
pub type AccessPath = Vec<PathElement>;

/// A binding extracted from a pattern.
#[derive(Debug, Clone)]
pub struct Binding {
    pub local_id: LocalId,
    pub name: String,
    pub is_mutable: bool,
    pub ty: ResolvedTy,
    pub path: AccessPath,
}

/// The decision tree for pattern matching.
#[derive(Debug, Clone)]
pub enum DecisionTree {
    /// Test a value and branch by constructor.
    Switch {
        path: AccessPath,
        ty: ResolvedTy,
        cases: Vec<(Constructor, DecisionTree)>,
        default: Option<Box<DecisionTree>>,
    },
    /// Successfully matched an arm.
    Success {
        arm_index: usize,
        bindings: Vec<Binding>,
    },
    /// Guard check: test condition, branch on result.
    Guard {
        arm_index: usize,
        bindings: Vec<Binding>,
        success: Box<DecisionTree>,
        failure: Box<DecisionTree>,
    },
    /// No match — unreachable if exhaustiveness passed.
    Failure,
}

// ===== Compilation =====

/// Compile patterns into a decision tree.
///
/// Uses the shared `PatternMatrix::specialize` for all specialization —
/// no duplicated pattern decomposition.
pub fn compile(
    hir: &HirBody,
    query: &QueryContext<'_>,
    patterns: &[FlatPat],
    arm_pat_ids: &[HirPatId],
    scrutinee_ty: &ResolvedTy,
    has_guards: &[bool],
) -> DecisionTree {
    // Build initial matrix
    let mut matrix = PatternMatrix::single_column(scrutinee_ty.clone());
    for (i, pat) in patterns.iter().enumerate() {
        let guard = has_guards.get(i).copied().unwrap_or(false);
        matrix.push(PatternRow::new(vec![pat.clone()], i, guard));
    }

    let col_paths = vec![vec![]]; // single column = scrutinee root
    compile_matrix(hir, query, &matrix, &col_paths, arm_pat_ids)
}

/// Recursive matrix compilation.
fn compile_matrix(
    hir: &HirBody,
    query: &QueryContext<'_>,
    matrix: &PatternMatrix,
    col_paths: &[AccessPath],
    arm_pat_ids: &[HirPatId],
) -> DecisionTree {
    // Base: empty matrix = no patterns match
    if matrix.is_empty() {
        return DecisionTree::Failure;
    }

    // Base: zero-width matrix = first row wins
    if matrix.is_unit() {
        return compile_leaf(hir, &matrix.rows, arm_pat_ids);
    }

    // Select best column (most distinct constructors)
    let col = select_column(matrix);
    let col_type = &matrix.col_types[col];
    let col_path = &col_paths[col];

    // Collect constructors in this column
    let head_ctors = matrix.head_constructors(col);

    // Check completeness
    let all_ctors = Constructor::all_for_type(query, col_type);
    let is_complete = all_ctors
        .as_ref()
        .is_some_and(|all| head_ctors.len() >= all.len() && all.iter().all(|c| head_ctors.contains(c)));

    // Build cases
    let mut cases = Vec::new();
    for ctor in &head_ctors {
        let specialized = matrix.specialize(query, col, ctor);
        let new_paths = build_specialized_paths(col_paths, col, ctor, query);
        let subtree = compile_matrix(hir, query, &specialized, &new_paths, arm_pat_ids);
        cases.push((ctor.clone(), subtree));
    }

    // Default case for incomplete constructor sets
    let default = if !is_complete {
        let default_matrix = matrix.default_matrix(col);
        let default_paths = build_default_paths(col_paths, col);
        if !default_matrix.is_empty() {
            Some(Box::new(compile_matrix(
                hir,
                query,
                &default_matrix,
                &default_paths,
                arm_pat_ids,
            )))
        } else {
            Some(Box::new(DecisionTree::Failure))
        }
    } else {
        None
    };

    DecisionTree::Switch {
        path: col_path.clone(),
        ty: col_type.clone(),
        cases,
        default,
    }
}

/// Compile a leaf node (matrix width is 0).
fn compile_leaf(hir: &HirBody, rows: &[PatternRow], arm_pat_ids: &[HirPatId]) -> DecisionTree {
    let Some(row) = rows.first() else {
        return DecisionTree::Failure;
    };

    // Collect bindings from the ORIGINAL HirPat (not the flattened one)
    let bindings = arm_pat_ids
        .get(row.arm_index)
        .map(|&pat_id| {
            let mut bindings = Vec::new();
            collect_bindings(hir, pat_id, &vec![], &mut bindings);
            bindings
        })
        .unwrap_or_default();

    if row.has_guard {
        let remaining: Vec<_> = rows
            .iter()
            .skip(1)
            .filter(|r| r.arm_index != row.arm_index)
            .cloned()
            .collect();
        let failure = if remaining.is_empty() {
            DecisionTree::Failure
        } else {
            compile_leaf(hir, &remaining, arm_pat_ids)
        };

        DecisionTree::Guard {
            arm_index: row.arm_index,
            bindings,
            success: Box::new(DecisionTree::Success {
                arm_index: row.arm_index,
                bindings: vec![],
            }),
            failure: Box::new(failure),
        }
    } else {
        DecisionTree::Success {
            arm_index: row.arm_index,
            bindings,
        }
    }
}

/// Select the best column to split on (necessity heuristic).
fn select_column(matrix: &PatternMatrix) -> usize {
    if matrix.width() <= 1 {
        return 0;
    }

    let mut best_col = 0;
    let mut best_score = 0usize;

    for col in 0..matrix.width() {
        let score = matrix.head_constructors(col).len();
        if score > best_score {
            best_score = score;
            best_col = col;
        }
    }

    best_col
}

/// Build access paths for a specialized matrix.
fn build_specialized_paths(
    col_paths: &[AccessPath],
    col: usize,
    ctor: &Constructor,
    query: &QueryContext<'_>,
) -> Vec<AccessPath> {
    let col_path = &col_paths[col];
    let arity = ctor.arity();

    let mut new_paths = Vec::with_capacity(col_paths.len() - 1 + arity);

    // Paths before the column
    new_paths.extend_from_slice(&col_paths[..col]);

    // Paths for the constructor's fields
    for i in 0..arity {
        let mut field_path = col_path.clone();
        match ctor {
            Constructor::Variant { .. } => {
                let name = ctor.display_name(query).trim_start_matches('.').to_string();
                field_path.push(PathElement::Downcast(name));
                field_path.push(PathElement::Index(i));
            }
            Constructor::Tuple { .. } | Constructor::Array { .. } => {
                field_path.push(PathElement::Index(i));
            }
            Constructor::Struct { .. } => {
                field_path.push(PathElement::Index(i));
            }
            _ => {} // Literals have no sub-fields
        }
        new_paths.push(field_path);
    }

    // Paths after the column
    if col + 1 < col_paths.len() {
        new_paths.extend_from_slice(&col_paths[col + 1..]);
    }

    new_paths
}

/// Build access paths for a default matrix (remove one column).
fn build_default_paths(col_paths: &[AccessPath], col: usize) -> Vec<AccessPath> {
    let mut paths = Vec::with_capacity(col_paths.len() - 1);
    paths.extend_from_slice(&col_paths[..col]);
    if col + 1 < col_paths.len() {
        paths.extend_from_slice(&col_paths[col + 1..]);
    }
    paths
}

// ===== Binding collection =====

/// Recursively collect bindings from a HirPat.
fn collect_bindings(hir: &HirBody, pat_id: HirPatId, path: &AccessPath, bindings: &mut Vec<Binding>) {
    match &hir.pats[pat_id] {
        HirPat::Binding { local, .. } => {
            let local_data = &hir.locals[*local];
            bindings.push(Binding {
                local_id: *local,
                name: local_data.name.clone(),
                is_mutable: local_data.is_mut,
                ty: ResolvedTy::Error, // resolved later by codegen
                path: path.clone(),
            });
        }

        HirPat::At {
            binding,
            subpattern,
            ..
        } => {
            let local_data = &hir.locals[*binding];
            bindings.push(Binding {
                local_id: *binding,
                name: local_data.name.clone(),
                is_mutable: local_data.is_mut,
                ty: ResolvedTy::Error,
                path: path.clone(),
            });
            collect_bindings(hir, *subpattern, path, bindings);
        }

        HirPat::Tuple { elements, .. } => {
            for (i, &elem) in elements.iter().enumerate() {
                let mut elem_path = path.clone();
                elem_path.push(PathElement::Index(i));
                collect_bindings(hir, elem, &elem_path, bindings);
            }
        }

        HirPat::Variant { args, .. } | HirPat::ImplicitVariant { args, .. } => {
            let case_name = match &hir.pats[pat_id] {
                HirPat::ImplicitVariant { name, .. } => name.clone(),
                _ => "variant".to_string(),
            };
            for (i, arg) in args.iter().enumerate() {
                let mut arg_path = path.clone();
                arg_path.push(PathElement::Downcast(case_name.clone()));
                arg_path.push(PathElement::Index(i));
                collect_bindings(hir, arg.pattern, &arg_path, bindings);
            }
        }

        HirPat::Struct { fields, .. } => {
            for field in fields {
                if let Some(pat) = field.pattern {
                    let mut field_path = path.clone();
                    field_path.push(PathElement::Field(field.field_name.clone()));
                    collect_bindings(hir, pat, &field_path, bindings);
                }
            }
        }

        HirPat::Or { alternatives, .. } => {
            // Use first alternative's bindings (type checker ensures consistency)
            if let Some(&first) = alternatives.first() {
                collect_bindings(hir, first, path, bindings);
            }
        }

        HirPat::Wildcard { .. }
        | HirPat::Literal { .. }
        | HirPat::Range { .. }
        | HirPat::Error { .. } => {
            // No bindings
        }
    }
}
