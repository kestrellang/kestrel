//! # Flattened Pattern Representation
//!
//! `FlatPat` is a normalized pattern stripped of spans, locals, and type
//! annotations. It's built once from `HirPat` via `flatten()`, then used
//! throughout the matrix algorithm and decision tree compilation.
//!
//! ## Why flatten?
//!
//! `HirPat` lives in an arena (`HirBody.pats`) and carries spans, local IDs,
//! and unresolved implicit variants. The matrix algorithm only cares about
//! constructor structure. `FlatPat` provides that:
//!
//! ```text
//! HirPat::ImplicitVariant { name: "Some", args: [Binding { local: x }] }
//!   ──flatten()──►  FlatPat::Ctor { ctor: Variant(some_entity, 1), children: [Wildcard] }
//! ```
//!
//! ## Key Functions
//!
//! - `flatten(hir, query, pat_id, scrutinee_ty)` — convert HirPat → FlatPat
//! - `FlatPat::decompose(ctor, arity)` — extract sub-patterns for specialization
//!   (the SINGLE decomposition function used by both matrix and decision tree)
//! - `FlatPat::is_wildcard_like()` — does this pattern match anything?
//! - `FlatPat::head_constructor()` — extract the constructor head

use kestrel_ast_builder::{Callable, Name, NodeKind};
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::body::*;
use kestrel_type_infer::result::ResolvedTy;

use super::constructor::{Constructor, collect_fields};

/// A normalized pattern for matrix operations.
///
/// Contains only the information the algorithm needs: constructor structure.
/// No spans, no local IDs, no type annotations.
#[derive(Clone, Debug)]
pub enum FlatPat {
    /// Matches anything (wildcard, binding, error recovery)
    Wildcard,
    /// Matches a specific constructor with sub-patterns
    Ctor {
        ctor: Constructor,
        children: Vec<FlatPat>,
    },
    /// Matches any of the alternatives
    Or(Vec<FlatPat>),
}

impl FlatPat {
    /// Extract the head constructor of this pattern.
    pub fn head_constructor(&self) -> Constructor {
        match self {
            FlatPat::Wildcard => Constructor::Wildcard,
            FlatPat::Ctor { ctor, .. } => ctor.clone(),
            FlatPat::Or(alts) => alts
                .first()
                .map(|a| a.head_constructor())
                .unwrap_or(Constructor::Wildcard),
        }
    }

    /// Check if this pattern matches any value (wildcard-like).
    pub fn is_wildcard_like(&self) -> bool {
        match self {
            FlatPat::Wildcard => true,
            FlatPat::Or(alts) => alts.iter().any(|a| a.is_wildcard_like()),
            FlatPat::Ctor { .. } => false,
        }
    }

    /// Decompose this pattern for specialization against `target_ctor`.
    ///
    /// Returns `Some(sub_patterns)` if this pattern is compatible with the
    /// constructor, `None` if incompatible (row should be dropped).
    ///
    /// This is the SINGLE decomposition function — used by both
    /// `PatternMatrix::specialize` and decision tree compilation.
    pub fn decompose(&self, target_ctor: &Constructor, arity: usize) -> Option<Vec<FlatPat>> {
        match self {
            // Wildcard matches any constructor — produce `arity` wildcards
            FlatPat::Wildcard => Some(vec![FlatPat::Wildcard; arity]),

            FlatPat::Ctor { ctor, children } => {
                if !ctor.matches(target_ctor) {
                    return None; // incompatible constructor
                }

                // Handle array patterns where source and target shapes differ
                if let (
                    Constructor::Array {
                        prefix_len: src_prefix,
                        suffix_len: src_suffix,
                        has_rest: src_rest,
                    },
                    Constructor::Array {
                        prefix_len: tgt_prefix,
                        suffix_len: tgt_suffix,
                        has_rest: tgt_rest,
                    },
                ) = (ctor, target_ctor)
                {
                    return Some(decompose_array(
                        children,
                        *src_prefix,
                        *src_suffix,
                        *src_rest,
                        *tgt_prefix,
                        *tgt_suffix,
                        *tgt_rest,
                    ));
                }

                Some(children.clone())
            },

            // Or-pattern: try each alternative, use first compatible one
            FlatPat::Or(alts) => alts
                .iter()
                .find_map(|alt| alt.decompose(target_ctor, arity)),
        }
    }
}

/// Handle array decomposition where source and target shapes may differ.
///
/// Four cases based on whether source/target have rest patterns:
/// 1. Both have rest → map prefix, rest, suffix
/// 2. Source has rest, target doesn't → expand rest to wildcards
/// 3. Source doesn't, target has rest → compress to target arity
/// 4. Neither has rest → direct mapping
fn decompose_array(
    children: &[FlatPat],
    src_prefix: usize,
    src_suffix: usize,
    src_rest: bool,
    tgt_prefix: usize,
    tgt_suffix: usize,
    tgt_rest: bool,
) -> Vec<FlatPat> {
    let tgt_arity = tgt_prefix + tgt_suffix + if tgt_rest { 1 } else { 0 };
    let mut result = Vec::with_capacity(tgt_arity);

    match (src_rest, tgt_rest) {
        // Case 1: both have rest — map prefix, rest wildcard, suffix
        (true, true) => {
            for i in 0..tgt_prefix {
                result.push(children.get(i).cloned().unwrap_or(FlatPat::Wildcard));
            }
            result.push(FlatPat::Wildcard); // rest slot
            // Source suffix children are at the END of the source array.
            let total_src = src_prefix + 1 + src_suffix;
            for i in 0..tgt_suffix {
                let src_idx = total_src.saturating_sub(tgt_suffix) + i;
                result.push(children.get(src_idx).cloned().unwrap_or(FlatPat::Wildcard));
            }
        },

        // Case 2: source has rest, target doesn't — expand rest to wildcards
        (true, false) => {
            for i in 0..tgt_prefix {
                result.push(children.get(i).cloned().unwrap_or(FlatPat::Wildcard));
            }
            // Source suffix children are after prefix + rest slot.
            // Total source children: src_prefix + 1 (rest) + src_suffix.
            let total_src = src_prefix + 1 + src_suffix;
            for i in 0..tgt_suffix {
                let src_idx = total_src.saturating_sub(tgt_suffix) + i;
                result.push(children.get(src_idx).cloned().unwrap_or(FlatPat::Wildcard));
            }
        },

        // Case 3: source doesn't have rest, target does — compress
        (false, true) => {
            for i in 0..tgt_prefix {
                result.push(children.get(i).cloned().unwrap_or(FlatPat::Wildcard));
            }
            result.push(FlatPat::Wildcard); // rest slot
            // Suffix children are at the END of the source children array.
            // Source has src_prefix + src_suffix children total (no rest slot).
            let total_src = src_prefix + src_suffix;
            for i in 0..tgt_suffix {
                let src_idx = total_src.saturating_sub(tgt_suffix) + i;
                result.push(children.get(src_idx).cloned().unwrap_or(FlatPat::Wildcard));
            }
        },

        // Case 4: neither has rest — direct mapping
        (false, false) => {
            for child in children {
                result.push(child.clone());
            }
        },
    }

    result
}

// ===== Conversion from HirPat =====

/// Convert a HirPat (arena-based) into a FlatPat (value-based).
///
/// Uses the query context to resolve entities and types. The `scrutinee_ty`
/// is needed to resolve ImplicitVariant patterns and expand struct fields.
pub fn flatten(
    hir: &HirBody,
    query: &QueryContext<'_>,
    pat_id: HirPatId,
    scrutinee_ty: &ResolvedTy,
) -> FlatPat {
    match &hir.pats[pat_id] {
        // Wildcards, bindings, and errors all match anything
        HirPat::Wildcard { .. } | HirPat::Binding { .. } | HirPat::Error { .. } => {
            FlatPat::Wildcard
        },

        HirPat::Literal { value, .. } => flatten_literal(value),

        HirPat::Range {
            start,
            end,
            inclusive,
            ..
        } => flatten_range(start, end, *inclusive),

        HirPat::Tuple {
            prefix,
            has_rest,
            suffix,
            ..
        } => {
            // Get element types from scrutinee
            let elem_types = match scrutinee_ty {
                ResolvedTy::Tuple(tys) => tys.clone(),
                _ => vec![ResolvedTy::Error; prefix.len() + suffix.len()],
            };
            let actual_arity = elem_types.len();

            // Flatten prefix elements
            let mut children: Vec<_> = prefix
                .iter()
                .enumerate()
                .map(|(i, &elem_id)| {
                    let elem_ty = elem_types.get(i).unwrap_or(&ResolvedTy::Error);
                    flatten(hir, query, elem_id, elem_ty)
                })
                .collect();

            if *has_rest {
                // Fill middle positions with wildcards
                let rest_count = actual_arity.saturating_sub(prefix.len() + suffix.len());
                for _ in 0..rest_count {
                    children.push(FlatPat::Wildcard);
                }
            }

            // Flatten suffix elements (from the end of the tuple)
            let suffix_start = actual_arity.saturating_sub(suffix.len());
            for (j, &elem_id) in suffix.iter().enumerate() {
                let elem_ty = elem_types
                    .get(suffix_start + j)
                    .unwrap_or(&ResolvedTy::Error);
                children.push(flatten(hir, query, elem_id, elem_ty));
            }

            FlatPat::Ctor {
                ctor: Constructor::Tuple {
                    arity: actual_arity,
                },
                children,
            }
        },

        // Fully resolved variant (entity known from name resolution)
        HirPat::Variant { entity, args, .. } => {
            let arity = args.len();
            let field_types = resolve_variant_field_types(query, *entity, scrutinee_ty);
            let children: Vec<_> = args
                .iter()
                .enumerate()
                .map(|(i, arg)| {
                    let arg_ty = field_types.get(i).unwrap_or(&ResolvedTy::Error);
                    flatten(hir, query, arg.pattern, arg_ty)
                })
                .collect();

            FlatPat::Ctor {
                ctor: Constructor::Variant {
                    entity: *entity,
                    arity,
                },
                children,
            }
        },

        // Implicit variant — resolve entity from scrutinee type's enum cases
        HirPat::ImplicitVariant { name, args, .. } => {
            let (entity, field_types) =
                resolve_implicit_variant(query, name.as_str_or_empty(), args.len(), scrutinee_ty);
            let children: Vec<_> = args
                .iter()
                .enumerate()
                .map(|(i, arg)| {
                    let arg_ty = field_types.get(i).unwrap_or(&ResolvedTy::Error);
                    flatten(hir, query, arg.pattern, arg_ty)
                })
                .collect();

            FlatPat::Ctor {
                ctor: Constructor::Variant {
                    entity,
                    arity: args.len(),
                },
                children,
            }
        },

        HirPat::Struct {
            entity,
            fields,
            has_rest: _,
            ..
        } => {
            // Expand struct pattern to cover ALL fields (missing = wildcard)
            let all_fields = collect_fields(query, *entity);
            // Resolve field types from the scrutinee type so nested patterns get correct types
            let struct_ctor = Constructor::Struct {
                entity: *entity,
                arity: all_fields.len(),
            };
            let field_types = struct_ctor.field_types(query, scrutinee_ty);

            let children: Vec<_> = all_fields
                .iter()
                .enumerate()
                .map(|(i, &field_entity)| {
                    let field_name = query
                        .get::<Name>(field_entity)
                        .map(|n| n.0.as_str())
                        .unwrap_or("");
                    let field_ty = field_types.get(i).unwrap_or(&ResolvedTy::Error);
                    // Find matching pattern field
                    let matched = fields
                        .iter()
                        .find(|f| f.field_name.as_str() == Some(field_name));
                    match matched.and_then(|f| f.pattern) {
                        Some(pat_id) => flatten(hir, query, pat_id, field_ty),
                        None => FlatPat::Wildcard,
                    }
                })
                .collect();

            FlatPat::Ctor {
                ctor: Constructor::Struct {
                    entity: *entity,
                    arity: all_fields.len(),
                },
                children,
            }
        },

        HirPat::Array {
            prefix,
            rest,
            suffix,
            ..
        } => {
            // Extract element type from scrutinee (Array[T] or Slice[T] → T)
            let elem_ty = match scrutinee_ty {
                ResolvedTy::Named { args, .. } => {
                    args.first().cloned().unwrap_or(ResolvedTy::Error)
                },
                _ => ResolvedTy::Error,
            };

            let has_rest = rest.is_some();
            let mut children: Vec<_> = prefix
                .iter()
                .map(|&id| flatten(hir, query, id, &elem_ty))
                .collect();
            if has_rest {
                children.push(FlatPat::Wildcard); // rest slot
            }
            children.extend(suffix.iter().map(|&id| flatten(hir, query, id, &elem_ty)));

            FlatPat::Ctor {
                ctor: Constructor::Array {
                    prefix_len: prefix.len(),
                    suffix_len: suffix.len(),
                    has_rest,
                },
                children,
            }
        },

        HirPat::Or { alternatives, .. } => {
            let alts: Vec<_> = alternatives
                .iter()
                .map(|&alt_id| flatten(hir, query, alt_id, scrutinee_ty))
                .collect();
            FlatPat::Or(alts)
        },

        // At-pattern: the binding is irrelevant to the matrix algorithm,
        // only the subpattern determines constructor structure
        HirPat::At { subpattern, .. } => flatten(hir, query, *subpattern, scrutinee_ty),
    }
}

// ===== Literal/range helpers =====

fn flatten_literal(value: &HirLiteral) -> FlatPat {
    let ctor = match value {
        HirLiteral::Bool(true) => Constructor::True,
        HirLiteral::Bool(false) => Constructor::False,
        HirLiteral::Integer(n) => Constructor::IntLiteral(*n),
        HirLiteral::Char(c) => Constructor::CharLiteral(char::from_u32(*c).unwrap_or('\0')),
        HirLiteral::String { value, .. } => Constructor::StringLiteral(value.clone()),
        HirLiteral::Float(_) => Constructor::NonExhaustive,
        HirLiteral::Null => Constructor::NonExhaustive,
    };
    FlatPat::Ctor {
        ctor,
        children: vec![],
    }
}

fn flatten_range(start: &Option<HirLiteral>, end: &Option<HirLiteral>, inclusive: bool) -> FlatPat {
    // Determine if integer or char range based on bounds
    let is_int = matches!(
        (start, end),
        (Some(HirLiteral::Integer(_)), _) | (_, Some(HirLiteral::Integer(_)))
    );

    let ctor = if is_int {
        let s = match start {
            Some(HirLiteral::Integer(v)) => Some(*v),
            _ => None,
        };
        let e = match end {
            Some(HirLiteral::Integer(v)) => Some(if inclusive { *v } else { v - 1 }),
            _ => None,
        };
        Constructor::IntRange { start: s, end: e }
    } else {
        let s = match start {
            Some(HirLiteral::Char(v)) => char::from_u32(*v),
            _ => None,
        };
        let e = match end {
            Some(HirLiteral::Char(v)) => {
                let c = char::from_u32(*v).unwrap_or('\0');
                Some(if inclusive {
                    c
                } else {
                    char::from_u32(*v - 1).unwrap_or(c)
                })
            },
            _ => None,
        };
        Constructor::CharRange { start: s, end: e }
    };

    FlatPat::Ctor {
        ctor,
        children: vec![],
    }
}

// ===== Entity resolution helpers =====

/// Get the field types for a variant entity within a parent type.
///
/// Delegates to `Constructor::field_types` via a temporary Variant constructor
/// to reuse the shared type resolution logic.
fn resolve_variant_field_types(
    query: &QueryContext<'_>,
    case_entity: Entity,
    scrutinee_ty: &ResolvedTy,
) -> Vec<ResolvedTy> {
    let arity = query
        .get::<Callable>(case_entity)
        .map(|c| c.params.len())
        .unwrap_or(0);
    let ctor = Constructor::Variant {
        entity: case_entity,
        arity,
    };
    ctor.field_types(query, scrutinee_ty)
}

/// Resolve an implicit variant name to its entity, searching the scrutinee type's cases.
fn resolve_implicit_variant(
    query: &QueryContext<'_>,
    name: &str,
    arity: usize,
    scrutinee_ty: &ResolvedTy,
) -> (Entity, Vec<ResolvedTy>) {
    if let ResolvedTy::Named { entity, .. } = scrutinee_ty {
        for &child in query.children_of(*entity) {
            if matches!(query.get::<NodeKind>(child), Some(NodeKind::EnumCase))
                && query.get::<Name>(child).is_some_and(|n| n.0 == name) {
                    let field_types = resolve_variant_field_types(query, child, scrutinee_ty);
                    return (child, field_types);
                }
        }
    }
    // Not found — use a synthetic entity (shouldn't happen after type inference)
    (Entity::from_raw(u32::MAX), vec![ResolvedTy::Error; arity])
}
