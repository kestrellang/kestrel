//! # Usefulness Analysis (Maranget 2007)
//!
//! A pattern row `q` is **useful** w.r.t. a matrix `P` if there exists a
//! value that matches `q` but not any row in `P`.
//!
//! - **Exhaustiveness**: match is exhaustive iff wildcard `_` is NOT useful
//! - **Redundancy**: arm is redundant iff it is NOT useful against prior arms
//!
//! ## Algorithm
//!
//! 1. **Empty matrix** — `q` is useful (no patterns to block it)
//! 2. **Zero-width matrix** — useful iff no unguarded row exists
//! 3. **Constructor head** — specialize matrix and `q` for that constructor, recurse
//! 4. **Wildcard head** — check each constructor of the column type:
//!    - Finite type: check uncovered constructors, then recurse into each
//!    - Infinite type: fall back to default matrix
//!
//! ## Entry Point
//!
//! `check_match()` builds the matrix, checks each arm for redundancy,
//! detects overlapping ranges, then checks if wildcard is useful (exhaustiveness).
//!
//! ## Guards
//!
//! Guarded arms are excluded from the exhaustiveness matrix — the guard
//! might fail at runtime, so the arm doesn't truly cover its pattern.

use std::collections::HashSet;

use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::body::*;
use kestrel_type_infer::result::ResolvedTy;

use super::constructor::Constructor;
use super::flat_pat::{self, FlatPat};
use super::matrix::{PatternMatrix, PatternRow};
use super::split;
use super::witness::Witness;

/// Result of exhaustiveness checking.
#[derive(Debug, Clone)]
pub struct ExhaustivenessResult {
    /// Whether all possible values are covered
    pub is_exhaustive: bool,
    /// Witnesses for uncovered values (if non-exhaustive)
    pub missing_patterns: Vec<Witness>,
    /// Indices of redundant (unreachable) arms
    pub redundant_arms: Vec<usize>,
    /// Indices of arms with overlapping range patterns
    pub overlapping_arms: Vec<usize>,
}

/// Result of a single usefulness check.
#[derive(Debug, Clone)]
pub struct UsefulnessResult {
    pub is_useful: bool,
    pub witness: Option<Witness>,
}

impl UsefulnessResult {
    fn not_useful() -> Self {
        UsefulnessResult {
            is_useful: false,
            witness: None,
        }
    }

    fn useful(witness: Witness) -> Self {
        UsefulnessResult {
            is_useful: true,
            witness: Some(witness),
        }
    }
}

/// Check exhaustiveness and redundancy for a match expression.
///
/// Converts HirPat arms to FlatPat, builds the pattern matrix, and runs
/// the Maranget algorithm to detect missing patterns and unreachable arms.
pub fn check_match(
    hir: &HirBody,
    query: &QueryContext<'_>,
    root: Entity,
    scrutinee_ty: &ResolvedTy,
    arms: &[HirMatchArm],
) -> ExhaustivenessResult {
    if arms.is_empty() {
        let is_never = matches!(scrutinee_ty, ResolvedTy::Never);
        return ExhaustivenessResult {
            is_exhaustive: is_never,
            missing_patterns: if is_never {
                vec![]
            } else {
                vec![Witness::any()]
            },
            redundant_arms: vec![],
            overlapping_arms: vec![],
        };
    }

    // Convert all arm patterns to FlatPat
    let flat_pats: Vec<FlatPat> = arms
        .iter()
        .map(|arm| flat_pat::flatten(hir, query, root, arm.pattern, scrutinee_ty))
        .collect();

    // Build matrix and check each arm for redundancy
    let mut matrix = PatternMatrix::single_column(scrutinee_ty.clone());
    let mut redundant_arms = Vec::new();
    let mut overlapping_arms = Vec::new();

    // Top-level literal/range arms seen so far, for the E307 overlap lint.
    // Redundancy needs no help here: `is_useful` splits ranges, so a range
    // covered only by the UNION of earlier ranges is already not useful.
    let mut prior_ranges: Vec<Constructor> = Vec::new();

    for (i, (flat_pat, arm)) in flat_pats.iter().zip(arms.iter()).enumerate() {
        let has_guard = arm.guard.is_some();
        let range = top_level_range(flat_pat);

        // An empty range (`10..=0`) matches nothing; bounds validation reports
        // it, so it is not also called unreachable.
        let is_empty_range = range.as_ref().is_some_and(|r| split::split(&[r.clone()]).is_empty());

        let query_row = PatternRow::new(vec![flat_pat.clone()], i, has_guard);
        let is_redundant =
            !has_guard && !is_empty_range && !is_useful(&matrix, &query_row, root, query).is_useful;

        if is_redundant {
            redundant_arms.push(i);
        } else if let Some(range) = range.filter(|_| !has_guard && !is_empty_range) {
            // Reachable, but shares values with an earlier range: E307.
            if prior_ranges.iter().any(|prior| ranges_overlap(prior, &range)) {
                overlapping_arms.push(i);
            }
            prior_ranges.push(range);
        }

        // Add to matrix (guarded arms don't cover for exhaustiveness)
        if !has_guard {
            // Expand or-patterns into separate rows
            let expanded = expand_or_pattern(flat_pat);
            for alt in expanded {
                matrix.push(PatternRow::new(vec![alt], i, false));
            }
        }
    }

    // Check exhaustiveness: is a wildcard useful?
    let wildcard_row = PatternRow::new(vec![FlatPat::Wildcard], arms.len(), false);
    let exhaustiveness = is_useful(&matrix, &wildcard_row, root, query);

    let (is_exhaustive, missing_patterns) = if exhaustiveness.is_useful {
        let witnesses = generate_witnesses(query, root, scrutinee_ty, &matrix);
        (false, witnesses)
    } else {
        (true, vec![])
    };

    ExhaustivenessResult {
        is_exhaustive,
        missing_patterns,
        redundant_arms,
        overlapping_arms,
    }
}

/// Core Maranget usefulness algorithm.
///
/// Checks if `query` row is useful against the pattern `matrix`.
pub fn is_useful(
    matrix: &PatternMatrix,
    query: &PatternRow,
    root: Entity,
    ctx: &QueryContext<'_>,
) -> UsefulnessResult {
    // Base case 1: empty matrix — query matches everything that fell through
    if matrix.is_empty() {
        return UsefulnessResult::useful(Witness::any());
    }

    // Base case 2: zero-width matrix — check for unguarded catch
    if matrix.is_unit() || query.pats.is_empty() {
        let has_unguarded = matrix.rows.iter().any(|row| !row.has_guard);
        return if has_unguarded {
            UsefulnessResult::not_useful()
        } else {
            UsefulnessResult::useful(Witness::any())
        };
    }

    // Always operate on column 0 for usefulness (decision tree uses other columns)
    let col = 0;
    let col_type = &matrix.col_types[col];
    let query_ctor = query.pats[col].head_constructor();

    if query_ctor.is_wildcard() {
        return is_wildcard_useful(matrix, query, col, col_type, root, ctx);
    }
    // A range (or array length) may be partly covered: cut it against the
    // column and ask of each piece. It is useful iff some piece is.
    for piece in matrix.split_head(col, &query_ctor) {
        let result = is_constructor_useful(matrix, query, col, &piece, col_type, root, ctx);
        if result.is_useful {
            return result;
        }
    }
    UsefulnessResult::not_useful()
}

/// Remove column `col` from a pattern vector.
fn remove_col(pats: &[FlatPat], col: usize) -> Vec<FlatPat> {
    let mut result = Vec::with_capacity(pats.len() - 1);
    result.extend_from_slice(&pats[..col]);
    if col + 1 < pats.len() {
        result.extend_from_slice(&pats[col + 1..]);
    }
    result
}

/// Check if a wildcard in the query is useful.
fn is_wildcard_useful(
    matrix: &PatternMatrix,
    query: &PatternRow,
    col: usize,
    col_type: &ResolvedTy,
    root: Entity,
    ctx: &QueryContext<'_>,
) -> UsefulnessResult {
    // If any row has a wildcard in this column, use the default matrix
    let has_catch_all = matrix
        .rows
        .iter()
        .any(|row| row.pats[col].is_wildcard_like());
    if has_catch_all {
        let default = matrix.default_matrix(col);
        let default_query = PatternRow::new(
            remove_col(&query.pats, col),
            query.arm_index,
            query.has_guard,
        );
        return is_useful(&default, &default_query, root, ctx);
    }

    // Get covered constructors
    let heads = matrix.head_constructors(col);
    let covered: HashSet<Constructor> = heads.iter().cloned().collect();

    match Constructor::all_for_type(ctx, root, col_type) {
        Some(all_ctors) => {
            // Finite constructor set — check for uncovered constructors
            for ctor in &all_ctors {
                if !covered.contains(ctor) {
                    return UsefulnessResult::useful(ctor.to_witness(ctx));
                }
            }
            // All covered — check sub-patterns
            for ctor in &all_ctors {
                let result = is_constructor_useful(matrix, query, col, ctor, col_type, root, ctx);
                if result.is_useful {
                    return result;
                }
            }
            UsefulnessResult::not_useful()
        },

        None => {
            // Range patterns (`..<0 | 0..=59 | 60..`) can cover all of an
            // infinite type (Int64, Char). The default matrix can't see that and
            // would report a false non-exhaustive, so treat the split pieces
            // like a finite constructor set: the wildcard is useful iff it is
            // useful under some piece — the OTHER columns may still be open.
            if covers_whole_domain(&heads) {
                for ctor in &heads {
                    let result = is_constructor_useful(matrix, query, col, ctor, col_type, root, ctx);
                    if result.is_useful {
                        return result;
                    }
                }
                return UsefulnessResult::not_useful();
            }

            // Infinite constructor set — check missing_constructors for special cases
            if let Some(missing) = Constructor::missing(ctx, root, col_type, &covered) {
                if missing.is_empty() {
                    // All covered (e.g., array with rest) — check sub-patterns
                    for ctor in &covered {
                        let result =
                            is_constructor_useful(matrix, query, col, ctor, col_type, root, ctx);
                        if result.is_useful {
                            return result;
                        }
                    }
                    return UsefulnessResult::not_useful();
                } else if !missing.contains(&Constructor::NonExhaustive) {
                    return UsefulnessResult::useful(missing[0].to_witness(ctx));
                }
            }

            // Fall back to default matrix
            let default = matrix.default_matrix(col);
            let default_query = PatternRow::new(
                remove_col(&query.pats, col),
                query.arm_index,
                query.has_guard,
            );

            if default.is_unit() && default.is_empty() {
                UsefulnessResult::useful(Witness::any())
            } else {
                is_useful(&default, &default_query, root, ctx)
            }
        },
    }
}

/// Check if a specific constructor is useful.
fn is_constructor_useful(
    matrix: &PatternMatrix,
    query: &PatternRow,
    col: usize,
    ctor: &Constructor,
    _col_type: &ResolvedTy,
    root: Entity,
    ctx: &QueryContext<'_>,
) -> UsefulnessResult {
    // Specialize matrix and query for this constructor
    let specialized = matrix.specialize(ctx, root, col, ctor);

    // Build specialized query row: [..col] + sub_pats + [col+1..]
    let arity = ctor.arity();
    let sub_pats = query.pats[col]
        .decompose(ctor, arity)
        .unwrap_or_else(|| vec![FlatPat::Wildcard; arity]);
    let mut new_query_pats = Vec::with_capacity(query.pats.len() - 1 + sub_pats.len());
    new_query_pats.extend_from_slice(&query.pats[..col]);
    new_query_pats.extend(sub_pats);
    if col + 1 < query.pats.len() {
        new_query_pats.extend_from_slice(&query.pats[col + 1..]);
    }
    let specialized_query = PatternRow::new(new_query_pats, query.arm_index, query.has_guard);

    let result = is_useful(&specialized, &specialized_query, root, ctx);

    if result.is_useful {
        // Wrap witness with this constructor
        let inner = result.witness.unwrap_or(Witness::any());
        let wrapped = wrap_witness(inner, ctor, ctx);
        UsefulnessResult::useful(wrapped)
    } else {
        UsefulnessResult::not_useful()
    }
}

/// Wrap an inner witness with a constructor.
fn wrap_witness(inner: Witness, ctor: &Constructor, ctx: &QueryContext<'_>) -> Witness {
    match ctor {
        Constructor::True => Witness::bool(true),
        Constructor::False => Witness::bool(false),
        Constructor::Variant { .. } => {
            let name = ctor.display_name(ctx).trim_start_matches('.').to_string();
            let args = match inner {
                Witness::Tuple(elems) => elems,
                Witness::Any if ctor.arity() == 0 => vec![],
                other => vec![other],
            };
            if args.is_empty() {
                Witness::enum_case(&name)
            } else {
                Witness::enum_case_with_args(&name, args)
            }
        },
        Constructor::Tuple { .. } => match inner {
            Witness::Tuple(_) => inner,
            Witness::Any => Witness::any(),
            other => Witness::tuple(vec![other]),
        },
        _ => inner,
    }
}

/// Generate witnesses for missing patterns.
fn generate_witnesses(
    query: &QueryContext<'_>,
    root: Entity,
    scrutinee_ty: &ResolvedTy,
    matrix: &PatternMatrix,
) -> Vec<Witness> {
    let covered: HashSet<Constructor> = matrix.head_constructors(0).into_iter().collect();

    match Constructor::all_for_type(query, root, scrutinee_ty) {
        Some(all) => {
            let missing: Vec<_> = all.into_iter().filter(|c| !covered.contains(c)).collect();
            if missing.is_empty() {
                vec![Witness::any()]
            } else {
                missing.iter().map(|c| c.to_witness(query)).collect()
            }
        },
        None => vec![Witness::any()],
    }
}

/// Expand or-patterns into a list of alternatives for the matrix.
fn expand_or_pattern(pat: &FlatPat) -> Vec<FlatPat> {
    match pat {
        FlatPat::Or(alts) => alts.iter().flat_map(expand_or_pattern).collect(),
        _ => vec![pat.clone()],
    }
}

/// The literal or range at the top of an arm's pattern, if any.
fn top_level_range(pat: &FlatPat) -> Option<Constructor> {
    let FlatPat::Ctor { ctor, .. } = pat else {
        return None;
    };
    matches!(
        ctor,
        Constructor::IntLiteral(_)
            | Constructor::IntRange { .. }
            | Constructor::CharLiteral(_)
            | Constructor::CharRange { .. }
    )
    .then(|| ctor.clone())
}

/// Do two literal/range constructors share a value? True iff splitting them
/// together yields a piece inside both.
fn ranges_overlap(a: &Constructor, b: &Constructor) -> bool {
    split::split(&[a.clone(), b.clone()])
        .iter()
        .any(|piece| a.matches(piece) && b.matches(piece))
}

/// Do these split int (or char) pieces cover every value of their type?
/// Pieces are sorted and disjoint, so they cover it iff they tile it.
fn covers_whole_domain(pieces: &[Constructor]) -> bool {
    let tiles = |mut intervals: Vec<(i128, i128)>, lo: i128, hi: i128| {
        intervals.sort_unstable();
        let mut next = lo;
        for (s, e) in intervals {
            if s > next {
                return false;
            }
            next = next.max(e + 1);
        }
        next > hi
    };
    let ints: Option<Vec<_>> = pieces.iter().map(split::int_interval).collect();
    if let Some(ints) = ints.filter(|v| !v.is_empty()) {
        return tiles(ints, i64::MIN as i128, i64::MAX as i128);
    }
    let chars: Option<Vec<_>> = pieces.iter().map(split::char_interval).collect();
    // The surrogate block holds no char, so a tiling may skip it.
    let chars = chars.filter(|v| !v.is_empty()).map(|mut v| {
        v.push((0xD800, 0xDFFF));
        v
    });
    chars.is_some_and(|v| tiles(v, 0, char::MAX as i128))
}
