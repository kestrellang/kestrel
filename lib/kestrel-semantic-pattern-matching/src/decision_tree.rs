//! Decision tree compilation for pattern matching.
//!
//! This module implements the Maranget algorithm for compiling pattern matrices
//! into decision trees that can be efficiently lowered to control flow.
//!
//! The decision tree is a representation of the pattern matching logic that:
//! - Eliminates redundant tests through matrix specialization
//! - Selects optimal columns for testing using heuristics
//! - Tracks variable bindings for each pattern
//!
//! # Algorithm
//!
//! The compilation uses Maranget's algorithm from "Compiling Pattern Matching to Good Decision Trees" (2008):
//!
//! 1. **Base cases**:
//!    - Empty matrix with non-empty rows: Success (first matching arm)
//!    - Empty matrix: Failure (no match possible)
//!
//! 2. **Column selection**:
//!    - Choose the column with the most distinct constructors (necessity heuristic)
//!    - This minimizes the number of tests needed
//!
//! 3. **Specialization**:
//!    - For each constructor in the chosen column, specialize the matrix
//!    - Recursively compile the specialized matrices
//!    - Build a switch over the constructors
//!
//! # Example
//!
//! Given patterns:
//! ```text
//! match x {
//!     .None => 0
//!     .Some(0) => 1
//!     .Some(n) => n
//! }
//! ```
//!
//! The decision tree is:
//! ```text
//! Switch(x, [
//!     None => Success(0, []),
//!     Some => Switch(x.0, [
//!         0 => Success(1, []),
//!         _ => Success(2, [n = x.0])
//!     ])
//! ])
//! ```

use crate::constructor::Constructor;
use crate::matrix::{PatternMatrix, PatternRow};
use kestrel_semantic_tree::pattern::{Pattern, PatternKind};
use kestrel_semantic_tree::symbol::local::LocalId;
use kestrel_semantic_tree::ty::Ty;

/// A path from the scrutinee to a sub-value.
///
/// Used to track where bindings come from in the decision tree.
/// For example, if matching `x` against `.Some((a, b))`:
/// - `a` comes from path `[Downcast("Some"), Index(0)]`
/// - `b` comes from path `[Downcast("Some"), Index(1)]`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathElement {
    /// Field access: `.field_name`
    Field(String),
    /// Tuple/array index: `.0`, `.1`, etc.
    Index(usize),
    /// Enum downcast: after switching on variant
    Downcast(String),
}

/// A path from the scrutinee root to a sub-value.
pub type AccessPath = Vec<PathElement>;

/// A binding extracted from a pattern.
#[derive(Debug, Clone)]
pub struct Binding {
    /// The local variable ID being bound
    pub local_id: LocalId,
    /// The name of the binding (for debugging)
    pub name: String,
    /// Whether the binding is mutable
    pub is_mutable: bool,
    /// The type of the bound value
    pub ty: Ty,
    /// The path from scrutinee to this binding's value
    pub path: AccessPath,
}

/// The decision tree for pattern matching.
///
/// This is the intermediate representation between patterns and MIR.
/// It represents the control flow needed to implement pattern matching.
#[derive(Debug, Clone)]
pub enum DecisionTree {
    /// Test a value and branch based on constructor.
    ///
    /// For enums, this generates a switch on the discriminant.
    /// For booleans, this generates a branch.
    /// For integers/strings, this generates comparison chains.
    Switch {
        /// Path to the value being tested
        path: AccessPath,
        /// Type of the value being tested  
        ty: Ty,
        /// Cases: (constructor, subtree)
        cases: Vec<(Constructor, DecisionTree)>,
        /// Optional default case (for infinite types like Int)
        default: Option<Box<DecisionTree>>,
    },

    /// Successfully matched an arm.
    Success {
        /// Index of the matched arm in the original match expression
        arm_index: usize,
        /// Bindings to create before executing the arm body
        bindings: Vec<Binding>,
    },

    /// Guard check: test a guard condition.
    ///
    /// If the guard succeeds, continue with `success`.
    /// If the guard fails, continue with `failure` (try next arm).
    Guard {
        /// Index of the arm with this guard
        arm_index: usize,
        /// Bindings needed for the guard expression
        bindings: Vec<Binding>,
        /// Tree to use if guard succeeds
        success: Box<DecisionTree>,
        /// Tree to use if guard fails
        failure: Box<DecisionTree>,
    },

    /// No patterns matched - this should be unreachable if exhaustiveness checking passed.
    Failure,
}

/// Context for decision tree compilation.
struct CompileContext {
    /// Maps column index to the path for accessing that column's value
    column_paths: Vec<AccessPath>,
    /// Maps column index to the type of that column
    column_types: Vec<Ty>,
    /// Original patterns for each arm (indexed by arm_index), used for binding collection
    original_patterns: Vec<Pattern>,
}

impl CompileContext {
    fn new(scrutinee_type: Ty, original_patterns: Vec<Pattern>) -> Self {
        CompileContext {
            column_paths: vec![vec![]], // Single column: the scrutinee itself
            column_types: vec![scrutinee_type],
            original_patterns,
        }
    }

    fn from_matrix(
        matrix: &PatternMatrix,
        column_paths: Vec<AccessPath>,
        original_patterns: Vec<Pattern>,
    ) -> Self {
        CompileContext {
            column_paths,
            column_types: matrix.column_types.clone(),
            original_patterns,
        }
    }
}

/// Compile a pattern matrix into a decision tree.
///
/// This is the main entry point for decision tree compilation.
///
/// # Arguments
///
/// * `patterns` - The patterns from match arms (one per arm)
/// * `scrutinee_type` - The type of the scrutinee being matched
/// * `has_guards` - For each arm, whether it has a guard
///
/// # Returns
///
/// A decision tree that implements the pattern matching logic.
pub fn compile(patterns: &[Pattern], scrutinee_type: &Ty, has_guards: &[bool]) -> DecisionTree {
    // Build the initial pattern matrix
    let mut matrix = PatternMatrix::single_column(scrutinee_type.clone());
    for (i, pattern) in patterns.iter().enumerate() {
        let has_guard = has_guards.get(i).copied().unwrap_or(false);
        matrix.push_row(vec![pattern.clone()], i, has_guard);
    }

    // Store original patterns for binding collection at leaf nodes
    let original_patterns = patterns.to_vec();
    let ctx = CompileContext::new(scrutinee_type.clone(), original_patterns);
    compile_matrix(&matrix, &ctx)
}

/// Compile a pattern matrix into a decision tree (internal recursive function).
fn compile_matrix(matrix: &PatternMatrix, ctx: &CompileContext) -> DecisionTree {
    // Base case 1: Empty matrix means no patterns match
    if matrix.is_empty() {
        return DecisionTree::Failure;
    }

    // Base case 2: Zero-width matrix (all columns exhausted)
    // The first row that doesn't have a guard wins
    if matrix.is_unit() {
        return compile_leaf(&matrix.rows, ctx);
    }

    // Select the best column to split on
    let col = select_column(matrix);
    let col_type = &ctx.column_types[col];
    let col_path = &ctx.column_paths[col];

    // Get all constructors that appear in this column
    let head_ctors = matrix.unique_head_constructors_for_column(col);

    // Check if we have a complete set of constructors
    let all_ctors = Constructor::all_constructors(col_type);
    let is_complete = match &all_ctors {
        Some(all) => head_ctors.len() >= all.len() && all.iter().all(|c| head_ctors.contains(c)),
        None => false, // Infinite type - never complete
    };

    // Build cases for each constructor
    let mut cases = Vec::new();
    for ctor in &head_ctors {
        let specialized = specialize_matrix_and_context(matrix, ctx, col, ctor);
        let subtree = compile_matrix(&specialized.0, &specialized.1);
        cases.push((ctor.clone(), subtree));
    }

    // Build default case if not complete
    let default = if !is_complete {
        let default_matrix = default_matrix_and_context(matrix, ctx, col);
        if !default_matrix.0.is_empty() {
            Some(Box::new(compile_matrix(
                &default_matrix.0,
                &default_matrix.1,
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
fn compile_leaf(rows: &[PatternRow], ctx: &CompileContext) -> DecisionTree {
    // Find the first row without a guard, or the first row with a guard
    if let Some(row) = rows.iter().next() {
        // Collect bindings from the ORIGINAL pattern for this arm, not the (possibly empty) row patterns.
        // The row patterns may have been stripped during matrix operations, but the original
        // pattern contains all the binding information we need.
        let bindings = if let Some(original_pattern) = ctx.original_patterns.get(row.arm_index) {
            let mut bindings = Vec::new();
            // The path is empty (root) because we're collecting from the original pattern
            // which represents the entire scrutinee
            collect_bindings_from_pattern(original_pattern, &vec![], &mut bindings);
            bindings
        } else {
            // Fallback to row-based collection (shouldn't happen in practice)
            collect_bindings_from_row(row, ctx)
        };

        if row.has_guard {
            // Need to check the guard
            // Create success and failure subtrees
            let remaining_rows: Vec<_> = rows
                .iter()
                .skip(1)
                .filter(|r| r.arm_index != row.arm_index)
                .cloned()
                .collect();

            let failure = if remaining_rows.is_empty() {
                DecisionTree::Failure
            } else {
                compile_leaf(&remaining_rows, ctx)
            };

            return DecisionTree::Guard {
                arm_index: row.arm_index,
                bindings,
                success: Box::new(DecisionTree::Success {
                    arm_index: row.arm_index,
                    bindings: vec![], // Bindings already extracted for guard
                }),
                failure: Box::new(failure),
            };
        } else {
            return DecisionTree::Success {
                arm_index: row.arm_index,
                bindings,
            };
        }
    }

    DecisionTree::Failure
}

/// Collect bindings from a pattern row.
fn collect_bindings_from_row(row: &PatternRow, ctx: &CompileContext) -> Vec<Binding> {
    let mut bindings = Vec::new();
    for (i, pattern) in row.patterns.iter().enumerate() {
        let path = if i < ctx.column_paths.len() {
            ctx.column_paths[i].clone()
        } else {
            vec![]
        };
        collect_bindings_from_pattern(pattern, &path, &mut bindings);
    }
    bindings
}

/// Recursively collect bindings from a pattern.
fn collect_bindings_from_pattern(
    pattern: &Pattern,
    path: &AccessPath,
    bindings: &mut Vec<Binding>,
) {
    match &pattern.kind {
        PatternKind::Local {
            local_id,
            mutability,
            name,
        } => {
            bindings.push(Binding {
                local_id: *local_id,
                name: name.clone(),
                is_mutable: mutability.is_mutable(),
                ty: pattern.ty.clone(),
                path: path.clone(),
            });
        },

        PatternKind::At {
            name,
            local_id,
            mutability,
            subpattern,
        } => {
            // Bind the whole value
            bindings.push(Binding {
                local_id: *local_id,
                name: name.clone(),
                is_mutable: mutability.is_mutable(),
                ty: pattern.ty.clone(),
                path: path.clone(),
            });
            // Also process subpattern bindings
            collect_bindings_from_pattern(subpattern, path, bindings);
        },

        PatternKind::Tuple { prefix, suffix, .. } => {
            for (i, elem) in prefix.iter().enumerate() {
                let mut elem_path = path.clone();
                elem_path.push(PathElement::Index(i));
                collect_bindings_from_pattern(elem, &elem_path, bindings);
            }
            // Handle suffix elements (they index from the end conceptually,
            // but we can compute their actual index based on tuple length)
            let prefix_len = prefix.len();
            for (i, elem) in suffix.iter().enumerate() {
                let mut elem_path = path.clone();
                elem_path.push(PathElement::Index(prefix_len + i));
                collect_bindings_from_pattern(elem, &elem_path, bindings);
            }
        },

        PatternKind::EnumVariant {
            case_name,
            bindings: enum_bindings,
            ..
        } => {
            // After matching this variant, we access fields through downcast
            for (i, binding) in enum_bindings.iter().enumerate() {
                let mut field_path = path.clone();
                field_path.push(PathElement::Downcast(case_name.clone()));
                field_path.push(PathElement::Index(i));
                collect_bindings_from_pattern(&binding.pattern, &field_path, bindings);
            }
        },

        PatternKind::Struct { fields, .. } => {
            for field in fields {
                let mut field_path = path.clone();
                field_path.push(PathElement::Field(field.field_name.clone()));
                collect_bindings_from_pattern(&field.pattern, &field_path, bindings);
            }
        },

        PatternKind::Array {
            prefix,
            rest,
            suffix,
        } => {
            for (i, elem) in prefix.iter().enumerate() {
                let mut elem_path = path.clone();
                elem_path.push(PathElement::Index(i));
                collect_bindings_from_pattern(elem, &elem_path, bindings);
            }
            // Rest binding (if named)
            if let Some((Some(name), Some(local_id))) = rest {
                bindings.push(Binding {
                    local_id: *local_id,
                    name: name.clone(),
                    is_mutable: false,
                    ty: pattern.ty.clone(), // Should be slice type
                    path: path.clone(),     // TODO: Need a special path for rest
                });
            }
            // Suffix elements index from the end
            for (i, elem) in suffix.iter().enumerate() {
                let mut elem_path = path.clone();
                // Negative indexing from end - actual index computed at runtime
                elem_path.push(PathElement::Index(prefix.len() + i));
                collect_bindings_from_pattern(elem, &elem_path, bindings);
            }
        },

        PatternKind::Or { alternatives } => {
            // For or-patterns, we need consistent bindings across all alternatives
            // Just use the first alternative's bindings (type checker ensures consistency)
            if let Some(first) = alternatives.first() {
                collect_bindings_from_pattern(first, path, bindings);
            }
        },

        PatternKind::Wildcard
        | PatternKind::Literal { .. }
        | PatternKind::Range { .. }
        | PatternKind::Rest
        | PatternKind::Error => {
            // No bindings
        },
    }
}

/// Select the best column to split on.
///
/// Uses the "necessity" heuristic: prefer columns with more distinct constructors,
/// as they're more likely to discriminate between patterns.
fn select_column(matrix: &PatternMatrix) -> usize {
    if matrix.width() <= 1 {
        return 0;
    }

    let mut best_col = 0;
    let mut best_score = 0usize;

    for col in 0..matrix.width() {
        let ctors = matrix.unique_head_constructors_for_column(col);
        let score = ctors.len();

        // Prefer columns with more constructors
        if score > best_score {
            best_score = score;
            best_col = col;
        }
    }

    best_col
}

/// Specialize the matrix for a constructor at a given column.
fn specialize_matrix_and_context(
    matrix: &PatternMatrix,
    ctx: &CompileContext,
    col: usize,
    ctor: &Constructor,
) -> (PatternMatrix, CompileContext) {
    let col_type = &ctx.column_types[col];
    let col_path = &ctx.column_paths[col];

    // Get field types for this constructor
    let field_types = get_constructor_field_types(ctor, col_type);

    // Specialize the matrix
    let specialized = matrix.specialize_column(col, ctor, &field_types);

    // Build new column paths
    let mut new_paths = Vec::with_capacity(specialized.width());

    // Add paths for the constructor's fields (replacing the specialized column)
    for (i, _) in field_types.iter().enumerate() {
        let mut field_path = col_path.clone();
        match ctor {
            Constructor::Variant { name, .. } => {
                field_path.push(PathElement::Downcast(name.clone()));
                field_path.push(PathElement::Index(i));
            },
            Constructor::Tuple { .. } => {
                field_path.push(PathElement::Index(i));
            },
            Constructor::Struct { .. } => {
                // For structs, we'd ideally use field names, but we don't have them here
                // This is handled specially during lowering
                field_path.push(PathElement::Index(i));
            },
            Constructor::Array { .. } => {
                field_path.push(PathElement::Index(i));
            },
            _ => {
                // Literals, booleans, etc. have no sub-fields
            },
        }
        new_paths.push(field_path);
    }

    // Add paths for remaining columns (after the specialized column)
    for (i, path) in ctx.column_paths.iter().enumerate() {
        if i != col {
            if i < col {
                new_paths.insert(i, path.clone());
            } else {
                new_paths.push(path.clone());
            }
        }
    }

    let new_ctx =
        CompileContext::from_matrix(&specialized, new_paths, ctx.original_patterns.clone());
    (specialized, new_ctx)
}

/// Compute the default matrix (rows with wildcards at the given column).
fn default_matrix_and_context(
    matrix: &PatternMatrix,
    ctx: &CompileContext,
    col: usize,
) -> (PatternMatrix, CompileContext) {
    let default = matrix.default_matrix_for_column(col);

    // Remove the column from paths
    let mut new_paths = Vec::with_capacity(default.width());
    for (i, path) in ctx.column_paths.iter().enumerate() {
        if i != col {
            new_paths.push(path.clone());
        }
    }

    let new_ctx = CompileContext::from_matrix(&default, new_paths, ctx.original_patterns.clone());
    (default, new_ctx)
}

/// Get field types for a constructor.
fn get_constructor_field_types(ctor: &Constructor, ty: &Ty) -> Vec<Ty> {
    use kestrel_semantic_tree::behavior::typed::TypedBehavior;
    use kestrel_semantic_tree::symbol::field::FieldSymbol;
    use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
    use kestrel_semantic_tree::ty::TyKind;
    use semantic_tree::symbol::Symbol;

    match (ctor, ty.kind()) {
        (Constructor::Tuple { arity }, TyKind::Tuple(elements)) => {
            if elements.len() == *arity {
                elements.clone()
            } else {
                vec![ty.clone(); *arity]
            }
        },

        (
            Constructor::Variant { name, arity },
            TyKind::Enum {
                symbol,
                substitutions,
            },
        ) => {
            if let Some(case) = symbol
                .cases()
                .iter()
                .find(|c| c.metadata().name().value == *name)
                && let Some(cb) = case.callable_behavior()
            {
                return cb
                    .parameters()
                    .iter()
                    .map(|p| substitutions.apply(&p.ty))
                    .collect();
            }
            vec![ty.clone(); *arity]
        },

        (
            Constructor::Struct { arity, .. },
            TyKind::Struct {
                symbol,
                substitutions,
            },
        ) => {
            let fields: Vec<_> = symbol
                .metadata()
                .children()
                .iter()
                .filter_map(|c| {
                    if c.metadata().kind() == KestrelSymbolKind::Field {
                        c.clone().downcast_arc::<FieldSymbol>().ok()
                    } else {
                        None
                    }
                })
                .collect();

            if fields.len() == *arity {
                fields
                    .iter()
                    .map(|f| {
                        let raw_field_ty = f
                            .metadata()
                            .get_behavior::<TypedBehavior>()
                            .map(|typed| typed.ty().clone())
                            .unwrap_or_else(|| f.field_type().clone());
                        raw_field_ty.apply_substitutions(substitutions)
                    })
                    .collect()
            } else {
                vec![ty.clone(); *arity]
            }
        },

        (
            Constructor::Array {
                prefix_len,
                suffix_len,
                has_rest,
            },
            TyKind::Array(element_type),
        ) => {
            let elem_ty = (**element_type).clone();
            let mut types = vec![elem_ty.clone(); *prefix_len];
            if *has_rest {
                types.push(ty.clone()); // Rest is an array/slice
            }
            types.extend(vec![elem_ty; *suffix_len]);
            types
        },

        _ => vec![ty.clone(); ctor.arity()],
    }
}

/// Collect all constructors from a pattern, expanding or-patterns recursively.
fn collect_constructors_from_pattern(
    pattern: &Pattern,
    seen: &mut std::collections::HashSet<Constructor>,
    result: &mut Vec<Constructor>,
) {
    match &pattern.kind {
        PatternKind::Or { alternatives } => {
            // Recursively collect from all alternatives
            for alt in alternatives {
                collect_constructors_from_pattern(alt, seen, result);
            }
        },
        PatternKind::At { subpattern, .. } => {
            // For @-patterns, collect from the subpattern
            collect_constructors_from_pattern(subpattern, seen, result);
        },
        _ => {
            // For all other patterns, extract the constructor
            let ctor = Constructor::from_pattern(pattern);
            if !ctor.is_wildcard() && seen.insert(ctor.clone()) {
                result.push(ctor);
            }
        },
    }
}

// Extension methods for PatternMatrix to support multi-column operations
impl PatternMatrix {
    /// Get unique constructors for a specific column.
    pub fn unique_head_constructors_for_column(&self, col: usize) -> Vec<Constructor> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for row in &self.rows {
            if let Some(pattern) = row.patterns.get(col) {
                // Collect all constructors from this pattern, expanding or-patterns
                collect_constructors_from_pattern(pattern, &mut seen, &mut result);
            }
        }
        result
    }

    /// Specialize the matrix for a constructor at a specific column.
    pub fn specialize_column(
        &self,
        col: usize,
        ctor: &Constructor,
        field_types: &[Ty],
    ) -> PatternMatrix {
        // Build new column types: fields from constructor + other columns
        let mut new_column_types = Vec::with_capacity(self.width() - 1 + field_types.len());

        // Columns before the specialized column
        new_column_types.extend(self.column_types[..col].iter().cloned());
        // Field types from the constructor
        new_column_types.extend(field_types.iter().cloned());
        // Columns after the specialized column
        if col + 1 < self.column_types.len() {
            new_column_types.extend(self.column_types[col + 1..].iter().cloned());
        }

        let mut result = PatternMatrix::new(new_column_types);

        for row in &self.rows {
            if let Some(specialized) = self.specialize_row_at_column(row, col, ctor, field_types) {
                result.push(specialized);
            }
        }

        result
    }

    /// Specialize a row at a specific column.
    fn specialize_row_at_column(
        &self,
        row: &PatternRow,
        col: usize,
        ctor: &Constructor,
        field_types: &[Ty],
    ) -> Option<PatternRow> {
        let pattern = row.patterns.get(col)?;
        let sub_patterns = self.extract_sub_patterns_for_column(pattern, ctor, field_types)?;

        // Build new pattern vector
        let mut new_patterns = Vec::with_capacity(row.patterns.len() - 1 + sub_patterns.len());

        // Patterns before the column
        new_patterns.extend(row.patterns[..col].iter().cloned());
        // Sub-patterns from the constructor
        new_patterns.extend(sub_patterns);
        // Patterns after the column
        if col + 1 < row.patterns.len() {
            new_patterns.extend(row.patterns[col + 1..].iter().cloned());
        }

        Some(PatternRow::new(new_patterns, row.arm_index, row.has_guard))
    }

    /// Extract sub-patterns when specializing a pattern for a constructor.
    fn extract_sub_patterns_for_column(
        &self,
        pattern: &Pattern,
        ctor: &Constructor,
        field_types: &[Ty],
    ) -> Option<Vec<Pattern>> {
        let pattern_ctor = Constructor::from_pattern(pattern);

        // Handle or-patterns
        if let PatternKind::Or { alternatives } = &pattern.kind {
            for alt in alternatives {
                if let Some(sub) = self.extract_sub_patterns_for_column(alt, ctor, field_types) {
                    return Some(sub);
                }
            }
            return None;
        }

        // Handle @-patterns
        if let PatternKind::At { subpattern, .. } = &pattern.kind {
            return self.extract_sub_patterns_for_column(subpattern, ctor, field_types);
        }

        // Wildcard matches any constructor
        if pattern_ctor.is_wildcard() {
            let wildcards: Vec<Pattern> = field_types
                .iter()
                .map(|ty| Pattern::wildcard(ty.clone(), pattern.span.clone()))
                .collect();
            return Some(wildcards);
        }

        // Check constructor compatibility
        if !constructors_compatible(&pattern_ctor, ctor) {
            return None;
        }

        // Extract sub-patterns based on pattern kind
        Some(extract_pattern_children(pattern, ctor, field_types))
    }

    /// Default matrix for a specific column (rows where the column is a wildcard).
    pub fn default_matrix_for_column(&self, col: usize) -> PatternMatrix {
        let mut new_column_types = Vec::with_capacity(self.width() - 1);
        new_column_types.extend(self.column_types[..col].iter().cloned());
        if col + 1 < self.column_types.len() {
            new_column_types.extend(self.column_types[col + 1..].iter().cloned());
        }

        let mut result = PatternMatrix::new(new_column_types);

        for row in &self.rows {
            if let Some(pattern) = row.patterns.get(col)
                && is_wildcard_like(pattern)
            {
                let mut new_patterns = Vec::with_capacity(row.patterns.len() - 1);
                new_patterns.extend(row.patterns[..col].iter().cloned());
                if col + 1 < row.patterns.len() {
                    new_patterns.extend(row.patterns[col + 1..].iter().cloned());
                }
                result.push(PatternRow::new(new_patterns, row.arm_index, row.has_guard));
            }
        }

        result
    }
}

/// Check if a pattern is wildcard-like.
fn is_wildcard_like(pattern: &Pattern) -> bool {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Local { .. } | PatternKind::Rest => true,
        PatternKind::At { subpattern, .. } => is_wildcard_like(subpattern),
        PatternKind::Or { alternatives } => alternatives.iter().any(is_wildcard_like),
        _ => false,
    }
}

/// Check if two constructors are compatible for specialization.
fn constructors_compatible(pattern_ctor: &Constructor, target_ctor: &Constructor) -> bool {
    match (pattern_ctor, target_ctor) {
        (Constructor::Wildcard, _) => true,
        (Constructor::True, Constructor::True) => true,
        (Constructor::False, Constructor::False) => true,
        (Constructor::Unit, Constructor::Unit) => true,
        (Constructor::Variant { name: n1, .. }, Constructor::Variant { name: n2, .. }) => n1 == n2,
        (Constructor::Tuple { arity: a1 }, Constructor::Tuple { arity: a2 }) => a1 == a2,
        (Constructor::Struct { name: n1, .. }, Constructor::Struct { name: n2, .. }) => n1 == n2,
        (Constructor::IntLiteral(v1), Constructor::IntLiteral(v2)) => v1 == v2,
        (Constructor::CharLiteral(v1), Constructor::CharLiteral(v2)) => v1 == v2,
        (Constructor::StringLiteral(s1), Constructor::StringLiteral(s2)) => s1 == s2,
        (
            Constructor::IntRange { start: s1, end: e1 },
            Constructor::IntRange { start: s2, end: e2 },
        ) => {
            s1 <= e2 && s2 <= e1 // Overlapping ranges
        },
        (Constructor::IntLiteral(v), Constructor::IntRange { start, end }) => {
            *v >= *start && *v <= *end
        },
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
            let min1 = p1 + s1;
            let min2 = p2 + s2;
            match (*r1, *r2) {
                (true, true) => true,
                (true, false) => min1 <= min2,
                (false, true) => min2 <= min1,
                (false, false) => min1 == min2,
            }
        },
        _ => false,
    }
}

/// Extract child patterns from a pattern.
fn extract_pattern_children(
    pattern: &Pattern,
    _target_ctor: &Constructor,
    field_types: &[Ty],
) -> Vec<Pattern> {
    match &pattern.kind {
        PatternKind::Tuple {
            prefix,
            suffix,
            has_rest,
        } => {
            if *has_rest {
                // Fill in wildcards for the rest
                let total = field_types.len();
                let known = prefix.len() + suffix.len();
                let rest_count = total.saturating_sub(known);

                let mut result = prefix.clone();
                for i in 0..rest_count {
                    let ty = field_types
                        .get(prefix.len() + i)
                        .cloned()
                        .unwrap_or_else(|| pattern.ty.clone());
                    result.push(Pattern::wildcard(ty, pattern.span.clone()));
                }
                result.extend(suffix.clone());
                result
            } else {
                prefix.clone()
            }
        },

        PatternKind::EnumVariant { bindings, .. } => {
            bindings.iter().map(|b| (*b.pattern).clone()).collect()
        },

        PatternKind::Struct { fields, .. } => {
            // Return patterns for each field in order
            // Missing fields are wildcards
            fields.iter().map(|f| f.pattern.clone()).collect()
        },

        PatternKind::Array {
            prefix,
            rest,
            suffix,
        } => {
            let mut result = prefix.clone();
            if rest.is_some() {
                result.push(Pattern::wildcard(pattern.ty.clone(), pattern.span.clone()));
            }
            result.extend(suffix.clone());
            result
        },

        _ => vec![], // Literals, wildcards, etc. have no children
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_semantic_tree::ty::IntBits;
    use kestrel_span::Span;

    fn test_span() -> Span {
        Span::new(0, 0..1)
    }

    fn bool_ty() -> Ty {
        Ty::bool(test_span())
    }

    fn int_ty() -> Ty {
        Ty::int(IntBits::I64, test_span())
    }

    #[test]
    fn test_compile_single_wildcard() {
        let patterns = vec![Pattern::wildcard(int_ty(), test_span())];
        let has_guards = vec![false];

        let tree = compile(&patterns, &int_ty(), &has_guards);

        // For infinite types like Int, a wildcard produces a Switch with
        // empty cases and a default that leads to Success
        match tree {
            DecisionTree::Success {
                arm_index,
                bindings,
            } => {
                assert_eq!(arm_index, 0);
                assert!(bindings.is_empty());
            },
            DecisionTree::Switch { cases, default, .. } => {
                // Should have no specific cases (no literals matched)
                assert!(cases.is_empty());
                // Default should lead to success
                match default.as_deref() {
                    Some(DecisionTree::Success {
                        arm_index,
                        bindings,
                    }) => {
                        assert_eq!(*arm_index, 0);
                        assert!(bindings.is_empty());
                    },
                    _ => panic!("Expected default to be Success, got {:?}", default),
                }
            },
            _ => panic!("Expected Success or Switch, got {:?}", tree),
        }
    }

    #[test]
    fn test_compile_bool_exhaustive() {
        let patterns = vec![
            Pattern::literal(
                kestrel_semantic_tree::expr::LiteralValue::Bool(true),
                bool_ty(),
                test_span(),
            ),
            Pattern::literal(
                kestrel_semantic_tree::expr::LiteralValue::Bool(false),
                bool_ty(),
                test_span(),
            ),
        ];
        let has_guards = vec![false, false];

        let tree = compile(&patterns, &bool_ty(), &has_guards);

        match tree {
            DecisionTree::Switch { cases, default, .. } => {
                assert_eq!(cases.len(), 2);
                assert!(default.is_none()); // Should be complete
            },
            _ => panic!("Expected Switch, got {:?}", tree),
        }
    }

    #[test]
    fn test_compile_int_with_default() {
        let patterns = vec![
            Pattern::literal(
                kestrel_semantic_tree::expr::LiteralValue::Integer(0),
                int_ty(),
                test_span(),
            ),
            Pattern::wildcard(int_ty(), test_span()),
        ];
        let has_guards = vec![false, false];

        let tree = compile(&patterns, &int_ty(), &has_guards);

        match tree {
            DecisionTree::Switch { cases, default, .. } => {
                assert_eq!(cases.len(), 1);
                assert!(default.is_some()); // Should have default for remaining ints
            },
            _ => panic!("Expected Switch, got {:?}", tree),
        }
    }
}
