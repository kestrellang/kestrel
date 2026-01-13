//! Pattern matrix representation for exhaustiveness checking.
//!
//! The pattern matrix is the core data structure in Maranget's algorithm.
//! Each row represents a match arm (or a vector of patterns being checked),
//! and each column corresponds to a component of the scrutinee type.
//!
//! # Key Operations
//!
//! - **specialize(matrix, constructor)**: Keeps rows that match the constructor,
//!   expanding sub-patterns. Used to narrow down the problem.
//! - **default(matrix)**: Keeps only wildcard rows, removing the first column.
//!   Used when checking for uncovered constructors.
//!
//! # Example
//!
//! Given scrutinee type `(Bool, Int)` and patterns:
//! ```text
//! (true, 1)
//! (false, _)
//! (_, 2)
//! ```
//!
//! The matrix is:
//! ```text
//! | true  | 1 |
//! | false | _ |
//! | _     | 2 |
//! ```
//!
//! Specializing by `true` for the first column gives:
//! ```text
//! | 1 |   // from row 1: true matches true
//! | 2 |   // from row 3: _ matches true
//! ```

use crate::constructor::Constructor;
use kestrel_semantic_tree::pattern::{Pattern, PatternKind};
use kestrel_semantic_tree::ty::Ty;

/// A row in the pattern matrix.
///
/// Each row represents a single match arm (or pattern vector).
#[derive(Debug, Clone)]
pub struct PatternRow {
    /// The patterns in this row (one per column)
    pub patterns: Vec<Pattern>,
    /// Index of the original match arm (for redundancy reporting)
    pub arm_index: usize,
    /// Whether this arm has a guard condition
    pub has_guard: bool,
}

impl PatternRow {
    /// Create a new row.
    pub fn new(patterns: Vec<Pattern>, arm_index: usize, has_guard: bool) -> Self {
        PatternRow {
            patterns,
            arm_index,
            has_guard,
        }
    }

    /// Get the first pattern in the row.
    pub fn first(&self) -> Option<&Pattern> {
        self.patterns.first()
    }

    /// Get patterns from index 1 onwards.
    pub fn rest(&self) -> &[Pattern] {
        if self.patterns.len() > 1 {
            &self.patterns[1..]
        } else {
            &[]
        }
    }

    /// Check if the row is empty.
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

/// A pattern matrix for exhaustiveness checking.
///
/// The matrix has a fixed number of columns (corresponding to the
/// "width" of the scrutinee type) and a variable number of rows
/// (one per match arm or pattern being checked).
#[derive(Debug, Clone)]
pub struct PatternMatrix {
    /// The rows of patterns
    pub rows: Vec<PatternRow>,
    /// Column types (one per column)
    pub column_types: Vec<Ty>,
}

impl PatternMatrix {
    /// Create an empty matrix with given column types.
    pub fn new(column_types: Vec<Ty>) -> Self {
        PatternMatrix {
            rows: Vec::new(),
            column_types,
        }
    }

    /// Create a single-column matrix for a simple scrutinee.
    pub fn single_column(scrutinee_type: Ty) -> Self {
        PatternMatrix::new(vec![scrutinee_type])
    }

    /// Add a row to the matrix.
    pub fn push_row(&mut self, patterns: Vec<Pattern>, arm_index: usize, has_guard: bool) {
        debug_assert_eq!(
            patterns.len(),
            self.column_types.len(),
            "Row has {} patterns but matrix has {} columns",
            patterns.len(),
            self.column_types.len()
        );
        self.rows
            .push(PatternRow::new(patterns, arm_index, has_guard));
    }

    /// Add a pattern row directly.
    pub fn push(&mut self, row: PatternRow) {
        debug_assert_eq!(
            row.patterns.len(),
            self.column_types.len(),
            "Row has {} patterns but matrix has {} columns",
            row.patterns.len(),
            self.column_types.len()
        );
        self.rows.push(row);
    }

    /// Check if the matrix has no rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Get the number of columns.
    pub fn width(&self) -> usize {
        self.column_types.len()
    }

    /// Check if this is a unit matrix (no columns).
    pub fn is_unit(&self) -> bool {
        self.column_types.is_empty()
    }

    /// Get the type of the first column.
    pub fn first_column_type(&self) -> Option<&Ty> {
        self.column_types.first()
    }

    /// Get all constructors that appear in the first column.
    pub fn head_constructors(&self) -> Vec<Constructor> {
        self.rows
            .iter()
            .filter_map(|row| {
                row.first().map(|p| {
                    let ctor = Constructor::from_pattern(p);
                    if ctor.is_wildcard() { None } else { Some(ctor) }
                })
            })
            .flatten()
            .collect()
    }

    /// Get unique constructors that appear in the first column.
    pub fn unique_head_constructors(&self) -> Vec<Constructor> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for row in &self.rows {
            if let Some(pattern) = row.first() {
                let ctor = Constructor::from_pattern(pattern);
                if !ctor.is_wildcard() && seen.insert(ctor.clone()) {
                    result.push(ctor);
                }
            }
        }
        result
    }

    /// Specialize the matrix for a given constructor.
    ///
    /// This is the S(c, P) operation from Maranget's paper.
    ///
    /// For each row in the matrix:
    /// - If the first pattern has constructor c: expand sub-patterns into columns
    /// - If the first pattern is a wildcard: create c.arity() wildcards
    /// - If the first pattern has a different constructor: drop the row
    ///
    /// The result has c.arity() + (width - 1) columns.
    pub fn specialize(&self, ctor: &Constructor, ctor_field_types: &[Ty]) -> PatternMatrix {
        // New column types: sub-pattern types from constructor + rest of original columns
        let mut new_column_types = ctor_field_types.to_vec();
        if self.column_types.len() > 1 {
            new_column_types.extend(self.column_types[1..].iter().cloned());
        }

        let mut result = PatternMatrix::new(new_column_types);

        for row in &self.rows {
            if let Some(specialized_row) = self.specialize_row(row, ctor) {
                result.push(specialized_row);
            }
        }

        result
    }

    /// Specialize a single row for a constructor.
    ///
    /// Returns None if the row's first pattern doesn't match the constructor.
    fn specialize_row(&self, row: &PatternRow, ctor: &Constructor) -> Option<PatternRow> {
        let first = row.first()?;

        // Get sub-patterns and check compatibility
        let sub_patterns = self.extract_sub_patterns(first, ctor)?;

        // Build new pattern vector: sub-patterns + rest of row
        let mut new_patterns = sub_patterns;
        new_patterns.extend(row.rest().iter().cloned());

        Some(PatternRow::new(new_patterns, row.arm_index, row.has_guard))
    }

    /// Extract sub-patterns from a pattern when matching against a constructor.
    ///
    /// Returns None if the pattern can't match the constructor.
    fn extract_sub_patterns(&self, pattern: &Pattern, ctor: &Constructor) -> Option<Vec<Pattern>> {
        let pattern_ctor = Constructor::from_pattern(pattern);

        // Handle or-patterns by expanding them
        if let PatternKind::Or { alternatives } = &pattern.kind {
            // For or-patterns, try each alternative
            // If any matches, use its sub-patterns
            for alt in alternatives {
                if let Some(sub) = self.extract_sub_patterns(alt, ctor) {
                    return Some(sub);
                }
            }
            return None;
        }

        // Handle @-patterns by looking at the subpattern
        if let PatternKind::At { subpattern, .. } = &pattern.kind {
            return self.extract_sub_patterns(subpattern, ctor);
        }

        if pattern_ctor.is_wildcard() {
            // Wildcard matches any constructor: generate arity wildcards
            let sub_patterns: Vec<Pattern> = (0..ctor.arity())
                .map(|i| {
                    let ty = self
                        .column_types
                        .first()
                        .cloned()
                        .unwrap_or_else(|| pattern.ty.clone());
                    // Try to get appropriate type for sub-pattern
                    let sub_ty = self.get_sub_type(&ty, ctor, i);
                    Pattern::wildcard(sub_ty, pattern.span.clone())
                })
                .collect();
            return Some(sub_patterns);
        }

        // Check if constructors match
        if !constructors_match(&pattern_ctor, ctor) {
            return None;
        }

        // Extract actual sub-patterns based on pattern kind
        Some(self.extract_pattern_children(pattern, ctor))
    }

    /// Get the type for a sub-pattern at the given index.
    fn get_sub_type(&self, parent_ty: &Ty, ctor: &Constructor, index: usize) -> Ty {
        use kestrel_semantic_tree::ty::TyKind;

        match (parent_ty.kind(), ctor) {
            (TyKind::Tuple(elements), Constructor::Tuple { .. }) => elements
                .get(index)
                .cloned()
                .unwrap_or_else(|| parent_ty.clone()),
            (TyKind::Enum { symbol, .. }, Constructor::Variant { name, .. }) => {
                // Find the case and get parameter type
                if let Some(case) = symbol.cases().iter().find(|c| {
                    use semantic_tree::symbol::Symbol;
                    c.metadata().name().value == *name
                }) {
                    if let Some(cb) = case.callable_behavior() {
                        if let Some(param) = cb.parameters().get(index) {
                            return param.ty.clone();
                        }
                    }
                }
                parent_ty.clone()
            }
            (
                TyKind::Struct {
                    symbol,
                    substitutions,
                },
                Constructor::Struct { .. },
            ) => {
                // Get field type at the given index
                use kestrel_semantic_tree::behavior::typed::TypedBehavior;
                use kestrel_semantic_tree::symbol::field::FieldSymbol;
                use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
                use semantic_tree::symbol::Symbol;

                let fields: Vec<_> = symbol
                    .metadata()
                    .children()
                    .into_iter()
                    .filter(|c| c.metadata().kind() == KestrelSymbolKind::Field)
                    .filter_map(|c| c.downcast_arc::<FieldSymbol>().ok())
                    .collect();

                if let Some(field) = fields.get(index) {
                    let raw_field_ty = field
                        .metadata()
                        .get_behavior::<TypedBehavior>()
                        .map(|typed| typed.ty().clone())
                        .unwrap_or_else(|| field.field_type().clone());
                    raw_field_ty.apply_substitutions(substitutions)
                } else {
                    parent_ty.clone()
                }
            }
            _ => parent_ty.clone(),
        }
    }

    /// Extract child patterns from a pattern.
    ///
    /// The `target_ctor` parameter is used to determine how many sub-patterns to extract
    /// when the pattern has a rest element (like `[a, ..]` or `(x, ..)`).
    fn extract_pattern_children(
        &self,
        pattern: &Pattern,
        target_ctor: &Constructor,
    ) -> Vec<Pattern> {
        match &pattern.kind {
            PatternKind::Tuple {
                prefix,
                has_rest,
                suffix,
            } => {
                // For tuple patterns with rest, we need to produce the right number of sub-patterns
                // based on the tuple type arity
                if *has_rest {
                    if let kestrel_semantic_tree::ty::TyKind::Tuple(elem_tys) = pattern.ty.kind() {
                        // Generate patterns: prefix + wildcards for rest + suffix
                        let rest_count = elem_tys.len().saturating_sub(prefix.len() + suffix.len());
                        let mut result = prefix.clone();
                        for i in 0..rest_count {
                            let ty_idx = prefix.len() + i;
                            let ty = elem_tys.get(ty_idx).cloned().unwrap_or(pattern.ty.clone());
                            result.push(Pattern::wildcard(ty, pattern.span.clone()));
                        }
                        result.extend(suffix.clone());
                        result
                    } else {
                        // Fallback: just return prefix + suffix
                        prefix.iter().chain(suffix.iter()).cloned().collect()
                    }
                } else {
                    // No rest pattern - just return prefix (suffix should be empty)
                    prefix.clone()
                }
            }
            PatternKind::EnumVariant { bindings, .. } => {
                bindings.iter().map(|b| (*b.pattern).clone()).collect()
            }
            PatternKind::Struct {
                fields,
                has_rest: _,
                ..
            } => {
                // For struct patterns, we need to return patterns for ALL fields
                // in the order they appear in the struct, not just the ones matched
                use kestrel_semantic_tree::symbol::field::FieldSymbol;
                use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
                use kestrel_semantic_tree::ty::TyKind;
                use semantic_tree::symbol::Symbol;

                if let TyKind::Struct {
                    symbol,
                    substitutions,
                } = pattern.ty.kind()
                {
                    // Get all field names from the struct in order
                    let struct_fields: Vec<_> = symbol
                        .metadata()
                        .children()
                        .into_iter()
                        .filter(|c| c.metadata().kind() == KestrelSymbolKind::Field)
                        .filter_map(|c| c.downcast_arc::<FieldSymbol>().ok())
                        .collect();

                    // Build the result by matching pattern fields to struct fields
                    let mut result = Vec::with_capacity(struct_fields.len());
                    for struct_field in &struct_fields {
                        let field_name = &struct_field.metadata().name().value;
                        // Find the pattern field for this struct field
                        let matched_field = fields.iter().find(|f| &f.field_name == field_name);

                        if let Some(pf) = matched_field {
                            result.push(pf.pattern.clone());
                        } else {
                            // Field not matched in pattern - use a wildcard
                            // Get the field type for the wildcard
                            use kestrel_semantic_tree::behavior::typed::TypedBehavior;
                            let raw_field_ty = struct_field
                                .metadata()
                                .get_behavior::<TypedBehavior>()
                                .map(|typed| typed.ty().clone())
                                .unwrap_or_else(|| struct_field.field_type().clone());
                            let field_ty = raw_field_ty.apply_substitutions(substitutions);
                            result.push(Pattern::wildcard(field_ty, pattern.span.clone()));
                        }
                    }
                    result
                } else {
                    // Fallback: just return the fields from the pattern
                    fields.iter().map(|f| f.pattern.clone()).collect()
                }
            }
            PatternKind::Array {
                prefix,
                rest,
                suffix,
            } => {
                // Array pattern specialization needs to match the target constructor's arity.
                // This is complex because we need to handle several cases:
                //
                // 1. Pattern with rest, target without rest (expansion):
                //    Pattern `[0, ..]` with target `Array{3, 0, false}` → `[0, _, _]`
                //
                // 2. Pattern without rest, target with rest (compression):
                //    Pattern `[1, 2, 3]` with target `Array{1, 0, true}` → `[1, _]`
                //    where _ is a wildcard for the rest (an array/slice)
                //
                // 3. Both have rest, or neither has rest (direct mapping)
                if let Constructor::Array {
                    prefix_len: target_prefix,
                    suffix_len: target_suffix,
                    has_rest: target_has_rest,
                } = target_ctor
                {
                    let target_arity =
                        target_prefix + target_suffix + if *target_has_rest { 1 } else { 0 };

                    // Get element type from array type
                    let elem_ty = if let kestrel_semantic_tree::ty::TyKind::Array(elem_ty) =
                        pattern.ty.kind()
                    {
                        (**elem_ty).clone()
                    } else {
                        pattern.ty.clone()
                    };

                    if rest.is_some() && !target_has_rest {
                        // Case 1: Pattern has rest, target doesn't - expand rest to wildcards
                        let mut result = Vec::with_capacity(target_arity);

                        // Take prefix elements (up to target_prefix)
                        for i in 0..*target_prefix {
                            if i < prefix.len() {
                                result.push(prefix[i].clone());
                            } else {
                                result
                                    .push(Pattern::wildcard(elem_ty.clone(), pattern.span.clone()));
                            }
                        }

                        // Take suffix elements (from the end)
                        for i in 0..*target_suffix {
                            let suffix_idx = suffix.len().saturating_sub(*target_suffix - i);
                            if suffix_idx < suffix.len() {
                                result.push(suffix[suffix_idx].clone());
                            } else {
                                result
                                    .push(Pattern::wildcard(elem_ty.clone(), pattern.span.clone()));
                            }
                        }

                        result
                    } else if rest.is_none() && *target_has_rest {
                        // Case 2: Pattern doesn't have rest, target does - compress to target arity
                        let mut result = Vec::with_capacity(target_arity);

                        // Take the first target_prefix elements from prefix
                        for i in 0..*target_prefix {
                            if i < prefix.len() {
                                result.push(prefix[i].clone());
                            } else {
                                result
                                    .push(Pattern::wildcard(elem_ty.clone(), pattern.span.clone()));
                            }
                        }

                        // Add a wildcard for the rest slot (represents remaining elements)
                        result.push(Pattern::wildcard(pattern.ty.clone(), pattern.span.clone()));

                        // Take the last target_suffix elements from suffix
                        for i in 0..*target_suffix {
                            let suffix_idx = suffix.len().saturating_sub(*target_suffix - i);
                            if suffix_idx < suffix.len() {
                                result.push(suffix[suffix_idx].clone());
                            } else {
                                result
                                    .push(Pattern::wildcard(elem_ty.clone(), pattern.span.clone()));
                            }
                        }

                        result
                    } else if rest.is_some() && *target_has_rest {
                        // Case 3: Both have rest - map prefix, rest, suffix
                        let mut result = Vec::with_capacity(target_arity);

                        // Map prefix elements
                        for i in 0..*target_prefix {
                            if i < prefix.len() {
                                result.push(prefix[i].clone());
                            } else {
                                result
                                    .push(Pattern::wildcard(elem_ty.clone(), pattern.span.clone()));
                            }
                        }

                        // Add a wildcard for the rest slot
                        result.push(Pattern::wildcard(pattern.ty.clone(), pattern.span.clone()));

                        // Map suffix elements
                        for i in 0..*target_suffix {
                            let suffix_idx = suffix.len().saturating_sub(*target_suffix - i);
                            if suffix_idx < suffix.len() {
                                result.push(suffix[suffix_idx].clone());
                            } else {
                                result
                                    .push(Pattern::wildcard(elem_ty.clone(), pattern.span.clone()));
                            }
                        }

                        result
                    } else {
                        // Case 4: Neither has rest - direct mapping
                        let mut result = prefix.clone();
                        result.extend(suffix.clone());
                        result
                    }
                } else {
                    // Fallback if not an array constructor
                    let mut children = prefix.clone();
                    if rest.is_some() {
                        children.push(Pattern::wildcard(pattern.ty.clone(), pattern.span.clone()));
                    }
                    children.extend(suffix.clone());
                    children
                }
            }
            _ => vec![], // Literals, wildcards, etc. have no children
        }
    }

    /// Compute the default matrix D(P).
    ///
    /// This keeps only rows where the first pattern is a wildcard/binding,
    /// and removes the first column.
    ///
    /// Used when checking if a wildcard would be useful.
    pub fn default_matrix(&self) -> PatternMatrix {
        let new_column_types = if self.column_types.len() > 1 {
            self.column_types[1..].to_vec()
        } else {
            vec![]
        };

        let mut result = PatternMatrix::new(new_column_types);

        for row in &self.rows {
            if let Some(first) = row.first() {
                // Handle or-patterns: if any alternative is a wildcard, include the row
                let is_wildcard = self.is_wildcard_like(first);

                if is_wildcard {
                    let new_patterns = row.rest().to_vec();
                    result.push(PatternRow::new(new_patterns, row.arm_index, row.has_guard));
                }
            }
        }

        result
    }

    /// Check if a pattern acts like a wildcard (matches anything).
    fn is_wildcard_like(&self, pattern: &Pattern) -> bool {
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Local { .. } | PatternKind::Rest => true,
            PatternKind::At { subpattern, .. } => self.is_wildcard_like(subpattern),
            PatternKind::Or { alternatives } => {
                // Or-pattern is wildcard-like if any alternative is
                alternatives.iter().any(|a| self.is_wildcard_like(a))
            }
            _ => false,
        }
    }
}

/// Check if two constructors match (for the purpose of specialization).
fn constructors_match(pattern_ctor: &Constructor, target_ctor: &Constructor) -> bool {
    match (pattern_ctor, target_ctor) {
        (Constructor::Wildcard, _) => true,
        (Constructor::True, Constructor::True) => true,
        (Constructor::False, Constructor::False) => true,
        (Constructor::Unit, Constructor::Unit) => true,

        (Constructor::Variant { name: n1, .. }, Constructor::Variant { name: n2, .. }) => n1 == n2,

        (Constructor::Tuple { arity: a1 }, Constructor::Tuple { arity: a2 }) => a1 == a2,

        (Constructor::Struct { name: n1, .. }, Constructor::Struct { name: n2, .. }) => n1 == n2,

        (Constructor::IntLiteral(v1), Constructor::IntLiteral(v2)) => v1 == v2,
        (Constructor::IntLiteral(v), Constructor::IntRange { start, end }) => {
            *v >= *start && *v <= *end
        }
        (
            Constructor::IntRange { start: s1, end: e1 },
            Constructor::IntRange { start: s2, end: e2 },
        ) => {
            // Ranges match if they overlap
            s1 <= e2 && s2 <= e1
        }

        (Constructor::CharLiteral(v1), Constructor::CharLiteral(v2)) => v1 == v2,
        (Constructor::CharLiteral(v), Constructor::CharRange { start, end }) => {
            *v >= *start && *v <= *end
        }
        (
            Constructor::CharRange { start: s1, end: e1 },
            Constructor::CharRange { start: s2, end: e2 },
        ) => s1 <= e2 && s2 <= e1,

        (Constructor::StringLiteral(s1), Constructor::StringLiteral(s2)) => s1 == s2,

        (
            Constructor::Array {
                prefix_len: p1,
                suffix_len: s1,
                has_rest: r1,
            },
            Constructor::Array {
                prefix_len: p2,
                suffix_len: s2,
                has_rest: r2,
            },
        ) => {
            // Array patterns match if they're compatible length-wise.
            // - A pattern with rest matches arrays of length >= min_len (prefix + suffix)
            // - A pattern without rest matches exactly one length
            let min_len_1 = p1 + s1;
            let min_len_2 = p2 + s2;

            match (*r1, *r2) {
                (true, true) => {
                    // Both have rest: compatible if their length ranges overlap
                    // Pattern with rest can match >= min_len, so they always overlap
                    true
                }
                (true, false) => {
                    // Pattern 1 has rest, pattern 2 is exact length
                    // Pattern 1 can match min_len_2 only if min_len_1 <= min_len_2
                    min_len_1 <= min_len_2
                }
                (false, true) => {
                    // Pattern 2 has rest, pattern 1 is exact length
                    // Pattern 2 can match min_len_1 only if min_len_2 <= min_len_1
                    min_len_2 <= min_len_1
                }
                (false, false) => {
                    // Both are fixed length: must match exactly
                    min_len_1 == min_len_2
                }
            }
        }

        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_semantic_tree::expr::LiteralValue;
    use kestrel_semantic_tree::pattern::Mutability;
    use kestrel_semantic_tree::symbol::local::LocalId;
    use kestrel_semantic_tree::ty::IntBits;
    use kestrel_span::Span;

    fn test_span() -> Span {
        Span::new(0, 0..1)
    }

    fn int_ty() -> Ty {
        Ty::int(IntBits::I64, test_span())
    }

    fn bool_ty() -> Ty {
        Ty::bool(test_span())
    }

    #[test]
    fn test_empty_matrix() {
        let matrix = PatternMatrix::single_column(int_ty());
        assert!(matrix.is_empty());
        assert_eq!(matrix.width(), 1);
    }

    #[test]
    fn test_add_row() {
        let mut matrix = PatternMatrix::single_column(int_ty());
        let pattern = Pattern::wildcard(int_ty(), test_span());
        matrix.push_row(vec![pattern], 0, false);
        assert_eq!(matrix.rows.len(), 1);
    }

    #[test]
    fn test_head_constructors() {
        let mut matrix = PatternMatrix::single_column(bool_ty());

        let true_pat = Pattern::literal(LiteralValue::Bool(true), bool_ty(), test_span());
        let wildcard = Pattern::wildcard(bool_ty(), test_span());

        matrix.push_row(vec![true_pat], 0, false);
        matrix.push_row(vec![wildcard], 1, false);

        let ctors = matrix.head_constructors();
        assert_eq!(ctors.len(), 1);
        assert_eq!(ctors[0], Constructor::True);
    }

    #[test]
    fn test_specialize_bool() {
        let mut matrix = PatternMatrix::single_column(bool_ty());

        let true_pat = Pattern::literal(LiteralValue::Bool(true), bool_ty(), test_span());
        let false_pat = Pattern::literal(LiteralValue::Bool(false), bool_ty(), test_span());
        let wildcard = Pattern::wildcard(bool_ty(), test_span());

        matrix.push_row(vec![true_pat], 0, false);
        matrix.push_row(vec![false_pat], 1, false);
        matrix.push_row(vec![wildcard.clone()], 2, false);

        // Specialize for True
        let specialized = matrix.specialize(&Constructor::True, &[]);

        // Should keep row 0 (true) and row 2 (wildcard)
        assert_eq!(specialized.rows.len(), 2);
        assert_eq!(specialized.rows[0].arm_index, 0);
        assert_eq!(specialized.rows[1].arm_index, 2);
    }

    #[test]
    fn test_specialize_tuple() {
        let tuple_ty = Ty::tuple(vec![bool_ty(), int_ty()], test_span());
        let mut matrix = PatternMatrix::single_column(tuple_ty.clone());

        // Pattern: (true, _)
        let pat1 = Pattern::tuple(
            vec![
                Pattern::literal(LiteralValue::Bool(true), bool_ty(), test_span()),
                Pattern::wildcard(int_ty(), test_span()),
            ],
            tuple_ty.clone(),
            test_span(),
        );

        // Pattern: (_, 42)
        let pat2 = Pattern::tuple(
            vec![
                Pattern::wildcard(bool_ty(), test_span()),
                Pattern::literal(LiteralValue::Integer(42), int_ty(), test_span()),
            ],
            tuple_ty.clone(),
            test_span(),
        );

        matrix.push_row(vec![pat1], 0, false);
        matrix.push_row(vec![pat2], 1, false);

        // Specialize for Tuple { arity: 2 }
        let specialized =
            matrix.specialize(&Constructor::Tuple { arity: 2 }, &[bool_ty(), int_ty()]);

        // Both rows should be kept, each expanded to 2 columns
        assert_eq!(specialized.rows.len(), 2);
        assert_eq!(specialized.width(), 2);
    }

    #[test]
    fn test_default_matrix() {
        let mut matrix = PatternMatrix::single_column(bool_ty());

        let true_pat = Pattern::literal(LiteralValue::Bool(true), bool_ty(), test_span());
        let wildcard = Pattern::wildcard(bool_ty(), test_span());
        let binding = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            bool_ty(),
            test_span(),
        );

        matrix.push_row(vec![true_pat], 0, false);
        matrix.push_row(vec![wildcard], 1, false);
        matrix.push_row(vec![binding], 2, false);

        let default = matrix.default_matrix();

        // Should only include rows 1 and 2 (wildcard and binding)
        assert_eq!(default.rows.len(), 2);
        assert_eq!(default.rows[0].arm_index, 1);
        assert_eq!(default.rows[1].arm_index, 2);
        // And should have 0 columns (original was 1 column, we removed it)
        assert_eq!(default.width(), 0);
    }
}
