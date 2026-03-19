//! Pattern matrix for exhaustiveness checking.
//!
//! The matrix is the core data structure in Maranget's algorithm. Each row
//! is a match arm and each column corresponds to a scrutinee component.
//!
//! Two key operations:
//! - `specialize(col, ctor)` — S(c, P): keep rows matching `ctor` at column `col`,
//!   expanding sub-patterns into new columns
//! - `default_matrix(col)` — D(P): keep only wildcard rows, removing column `col`
//!
//! Multi-column support is built in from the start (unlike lib1 where it was
//! bolted on via an external impl block).

use kestrel_hecs::QueryContext;
use kestrel_type_infer::result::ResolvedTy;

use super::constructor::Constructor;
use super::flat_pat::FlatPat;

/// A row in the pattern matrix.
#[derive(Clone, Debug)]
pub struct PatternRow {
    /// One pattern per column
    pub pats: Vec<FlatPat>,
    /// Index of the original match arm
    pub arm_index: usize,
    /// Whether this arm has a guard condition
    pub has_guard: bool,
}

impl PatternRow {
    pub fn new(pats: Vec<FlatPat>, arm_index: usize, has_guard: bool) -> Self {
        PatternRow {
            pats,
            arm_index,
            has_guard,
        }
    }
}

/// A pattern matrix for exhaustiveness checking.
#[derive(Clone, Debug)]
pub struct PatternMatrix {
    /// The rows (one per match arm or pattern vector)
    pub rows: Vec<PatternRow>,
    /// Type of each column
    pub col_types: Vec<ResolvedTy>,
}

impl PatternMatrix {
    /// Create an empty matrix with the given column types.
    pub fn new(col_types: Vec<ResolvedTy>) -> Self {
        PatternMatrix {
            rows: Vec::new(),
            col_types,
        }
    }

    /// Create a single-column matrix for a simple scrutinee.
    pub fn single_column(ty: ResolvedTy) -> Self {
        PatternMatrix::new(vec![ty])
    }

    /// Add a row to the matrix.
    pub fn push(&mut self, row: PatternRow) {
        debug_assert_eq!(
            row.pats.len(),
            self.col_types.len(),
            "row has {} patterns but matrix has {} columns",
            row.pats.len(),
            self.col_types.len()
        );
        self.rows.push(row);
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn width(&self) -> usize {
        self.col_types.len()
    }

    /// True if the matrix has zero columns (unit matrix).
    pub fn is_unit(&self) -> bool {
        self.col_types.is_empty()
    }

    /// Get unique non-wildcard constructors in the given column.
    pub fn head_constructors(&self, col: usize) -> Vec<Constructor> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();

        for row in &self.rows {
            if let Some(pat) = row.pats.get(col) {
                collect_constructors(pat, &mut seen, &mut result);
            }
        }

        result
    }

    /// Specialize the matrix for constructor `ctor` at column `col`.
    ///
    /// This is the S(c, P) operation from Maranget's paper, generalized
    /// to work on any column (not just column 0).
    ///
    /// Delegates to `FlatPat::decompose` for all pattern decomposition —
    /// no duplicated extraction logic.
    pub fn specialize(
        &self,
        query: &QueryContext<'_>,
        col: usize,
        ctor: &Constructor,
    ) -> PatternMatrix {
        let field_types = ctor.field_types(query, &self.col_types[col]);
        let arity = ctor.arity();

        // New column types: [..col] + field_types + [col+1..]
        let mut new_col_types = Vec::with_capacity(self.width() - 1 + field_types.len());
        new_col_types.extend_from_slice(&self.col_types[..col]);
        new_col_types.extend(field_types);
        if col + 1 < self.col_types.len() {
            new_col_types.extend_from_slice(&self.col_types[col + 1..]);
        }

        let mut result = PatternMatrix::new(new_col_types);

        for row in &self.rows {
            // Expand or-patterns: each alternative that matches the constructor
            // becomes a separate row, ensuring all matching alternatives are considered.
            let decompositions = decompose_all(&row.pats[col], ctor, arity);
            if decompositions.is_empty() {
                continue; // incompatible — drop row
            }

            for sub_pats in decompositions {
                // Build new pattern vector: [..col] + sub_pats + [col+1..]
                let mut new_pats = Vec::with_capacity(row.pats.len() - 1 + sub_pats.len());
                new_pats.extend_from_slice(&row.pats[..col]);
                new_pats.extend(sub_pats);
                if col + 1 < row.pats.len() {
                    new_pats.extend_from_slice(&row.pats[col + 1..]);
                }

                result.push(PatternRow::new(new_pats, row.arm_index, row.has_guard));
            }
        }

        result
    }

    /// Compute the default matrix D(P) for the given column.
    ///
    /// Keeps only rows where column `col` is wildcard-like, removing that column.
    pub fn default_matrix(&self, col: usize) -> PatternMatrix {
        let mut new_col_types = Vec::with_capacity(self.width() - 1);
        new_col_types.extend_from_slice(&self.col_types[..col]);
        if col + 1 < self.col_types.len() {
            new_col_types.extend_from_slice(&self.col_types[col + 1..]);
        }

        let mut result = PatternMatrix::new(new_col_types);

        for row in &self.rows {
            if row.pats[col].is_wildcard_like() {
                let mut new_pats = Vec::with_capacity(row.pats.len() - 1);
                new_pats.extend_from_slice(&row.pats[..col]);
                if col + 1 < row.pats.len() {
                    new_pats.extend_from_slice(&row.pats[col + 1..]);
                }
                result.push(PatternRow::new(new_pats, row.arm_index, row.has_guard));
            }
        }

        result
    }
}

/// Decompose a pattern for ALL matching alternatives of an or-pattern.
///
/// Unlike `FlatPat::decompose` which returns only the first match,
/// this returns every compatible decomposition. Each one becomes a
/// separate matrix row during specialization.
fn decompose_all(pat: &FlatPat, ctor: &Constructor, arity: usize) -> Vec<Vec<FlatPat>> {
    match pat {
        FlatPat::Wildcard => vec![vec![FlatPat::Wildcard; arity]],

        FlatPat::Ctor { .. } => {
            // Single constructor — delegate to decompose
            match pat.decompose(ctor, arity) {
                Some(sub) => vec![sub],
                None => vec![],
            }
        }

        FlatPat::Or(alts) => {
            // Collect ALL matching alternatives, not just the first
            alts.iter()
                .flat_map(|alt| decompose_all(alt, ctor, arity))
                .collect()
        }
    }
}

/// Recursively collect constructors from a pattern, expanding or-patterns.
fn collect_constructors(
    pat: &FlatPat,
    seen: &mut std::collections::HashSet<Constructor>,
    result: &mut Vec<Constructor>,
) {
    match pat {
        FlatPat::Wildcard => {}
        FlatPat::Ctor { ctor, .. } => {
            if !ctor.is_wildcard() && seen.insert(ctor.clone()) {
                result.push(ctor.clone());
            }
        }
        FlatPat::Or(alts) => {
            for alt in alts {
                collect_constructors(alt, seen, result);
            }
        }
    }
}
