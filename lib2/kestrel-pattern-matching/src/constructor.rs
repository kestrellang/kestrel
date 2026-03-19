//! Constructor representation and type classification for pattern matching.
//!
//! A "constructor" is the head of a pattern — the way to build a value of a type.
//! For exhaustiveness checking we need to know all constructors of a type.
//!
//! `TypeShape` classifies a `ResolvedTy` into its constructor space. This is
//! the single point where exhaustiveness rules are defined — adding support
//! for new exhaustive types (e.g., single-variant enums) means adding one
//! match arm in `TypeShape::classify`.

use std::collections::HashSet;

use kestrel_ast_builder::{Callable, Name, NodeKind, TypeAnnotation, TypeParams};
use kestrel_hecs::{Entity, QueryContext};
use kestrel_type_infer::result::ResolvedTy;

use super::witness::Witness;

// ===== Constructor =====

/// The head of a pattern, abstracting away sub-patterns.
///
/// Constructors partition the value space of a type. Two patterns with
/// different constructors can never match the same value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constructor {
    /// Boolean true
    True,
    /// Boolean false
    False,
    /// Enum case identified by entity
    Variant { entity: Entity, arity: usize },
    /// Tuple with N elements
    Tuple { arity: usize },
    /// Struct (single constructor for the type)
    Struct { entity: Entity, arity: usize },
    /// Integer literal
    IntLiteral(i64),
    /// Integer range (both ends inclusive after normalization, None = unbounded)
    IntRange { start: Option<i64>, end: Option<i64> },
    /// Character literal
    CharLiteral(char),
    /// Character range (both ends inclusive, None = unbounded)
    CharRange { start: Option<char>, end: Option<char> },
    /// String literal
    StringLiteral(String),
    /// Unit value ()
    Unit,
    /// Wildcard — matches anything
    Wildcard,
    /// Array pattern with fixed prefix/suffix and optional rest
    Array {
        prefix_len: usize,
        suffix_len: usize,
        has_rest: bool,
    },
    /// Marker for types with infinite constructors
    NonExhaustive,
    /// Placeholder for missing constructors in witness generation
    Missing,
}

impl Constructor {
    /// Number of sub-patterns this constructor expects.
    pub fn arity(&self) -> usize {
        match self {
            Constructor::True | Constructor::False => 0,
            Constructor::Variant { arity, .. } | Constructor::Tuple { arity } => *arity,
            Constructor::Struct { arity, .. } => *arity,
            Constructor::IntLiteral(_) | Constructor::IntRange { .. } => 0,
            Constructor::CharLiteral(_) | Constructor::CharRange { .. } => 0,
            Constructor::StringLiteral(_) => 0,
            Constructor::Unit => 0,
            Constructor::Wildcard => 0,
            Constructor::Array {
                prefix_len,
                suffix_len,
                has_rest,
            } => prefix_len + suffix_len + if *has_rest { 1 } else { 0 },
            Constructor::NonExhaustive | Constructor::Missing => 0,
        }
    }

    pub fn is_wildcard(&self) -> bool {
        matches!(self, Constructor::Wildcard)
    }

    /// Check if this constructor is compatible with `other` for specialization.
    ///
    /// Returns true if a value matching `self` could also match `other`.
    /// This is the SINGLE compatibility check — no duplicates elsewhere.
    pub fn matches(&self, other: &Constructor) -> bool {
        match (self, other) {
            (Constructor::Wildcard, _) => true,
            (Constructor::True, Constructor::True) => true,
            (Constructor::False, Constructor::False) => true,
            (Constructor::Unit, Constructor::Unit) => true,

            // Entity-based matching — exact entity equality
            (Constructor::Variant { entity: e1, .. }, Constructor::Variant { entity: e2, .. }) => {
                e1 == e2
            }
            (Constructor::Struct { entity: e1, .. }, Constructor::Struct { entity: e2, .. }) => {
                e1 == e2
            }
            (Constructor::Tuple { arity: a1 }, Constructor::Tuple { arity: a2 }) => a1 == a2,

            // Integer comparisons
            (Constructor::IntLiteral(v1), Constructor::IntLiteral(v2)) => v1 == v2,
            (Constructor::IntLiteral(v), Constructor::IntRange { start, end }) => {
                start.map_or(true, |s| *v >= s) && end.map_or(true, |e| *v <= e)
            }
            (
                Constructor::IntRange { start: s1, end: e1 },
                Constructor::IntRange { start: s2, end: e2 },
            ) => ranges_overlap_i64(*s1, *e1, *s2, *e2),

            // Character comparisons
            (Constructor::CharLiteral(v1), Constructor::CharLiteral(v2)) => v1 == v2,
            (Constructor::CharLiteral(v), Constructor::CharRange { start, end }) => {
                start.map_or(true, |s| *v >= s) && end.map_or(true, |e| *v <= e)
            }
            (
                Constructor::CharRange { start: s1, end: e1 },
                Constructor::CharRange { start: s2, end: e2 },
            ) => ranges_overlap_char(*s1, *e1, *s2, *e2),

            // String
            (Constructor::StringLiteral(s1), Constructor::StringLiteral(s2)) => s1 == s2,

            // Array length compatibility
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
            }

            _ => false,
        }
    }

    /// Get the types of sub-patterns for this constructor applied to `parent_ty`.
    ///
    /// For example, `Variant("Some", 1)` on `Optional[Int]` returns `[Int]`.
    /// This is the SINGLE implementation — no duplicates elsewhere.
    pub fn field_types(&self, query: &QueryContext<'_>, parent_ty: &ResolvedTy) -> Vec<ResolvedTy> {
        match (self, parent_ty) {
            (Constructor::Tuple { .. }, ResolvedTy::Tuple(elems)) => elems.clone(),

            (
                Constructor::Variant { entity, arity },
                ResolvedTy::Named {
                    entity: enum_entity,
                    args,
                },
            ) => {
                // Look up the case's Callable to get parameter types
                if let Some(callable) = query.get::<Callable>(*entity) {
                    return callable
                        .params
                        .iter()
                        .map(|p| {
                            // Resolve the parameter type with the enum's type args
                            resolve_case_param_type(query, *enum_entity, args, p)
                        })
                        .collect();
                }
                vec![ResolvedTy::Error; *arity]
            }

            (
                Constructor::Struct { entity, arity },
                ResolvedTy::Named { args, .. },
            ) => {
                // Get Field children and resolve their types with substitutions
                let fields = collect_fields(query, *entity);
                if fields.len() == *arity {
                    fields
                        .iter()
                        .map(|field_entity| resolve_field_type(query, *field_entity, *entity, args))
                        .collect()
                } else {
                    vec![parent_ty.clone(); *arity]
                }
            }

            // Array[T] — element type from type args
            (
                Constructor::Array {
                    prefix_len,
                    suffix_len,
                    has_rest,
                },
                ResolvedTy::Named { args, .. },
            ) => {
                let elem_ty = args.first().cloned().unwrap_or(ResolvedTy::Error);
                let mut types = vec![elem_ty.clone(); *prefix_len];
                if *has_rest {
                    types.push(parent_ty.clone()); // rest is the array type
                }
                types.extend(vec![elem_ty; *suffix_len]);
                types
            }

            _ => vec![parent_ty.clone(); self.arity()],
        }
    }

    /// Enumerate all constructors for a type. Returns `None` for infinite types.
    pub fn all_for_type(query: &QueryContext<'_>, ty: &ResolvedTy) -> Option<Vec<Constructor>> {
        let shape = TypeShape::classify(query, ty);
        shape.constructors(query)
    }

    /// Find constructors not covered by `covered`. Returns `None` if impossible.
    pub fn missing(
        query: &QueryContext<'_>,
        ty: &ResolvedTy,
        covered: &HashSet<Constructor>,
    ) -> Option<Vec<Constructor>> {
        if covered.contains(&Constructor::Wildcard) {
            return Some(vec![]);
        }

        match Constructor::all_for_type(query, ty) {
            Some(all) => {
                let missing: Vec<_> = all.into_iter().filter(|c| !covered.contains(c)).collect();
                Some(missing)
            }
            None => {
                // Infinite type — check for special cases like arrays
                if is_array_type(query, ty) {
                    return missing_array_constructors(covered);
                }
                // Need a wildcard to cover infinite types
                Some(vec![Constructor::NonExhaustive])
            }
        }
    }

    /// Human-readable name for error messages.
    pub fn display_name(&self, query: &QueryContext<'_>) -> String {
        match self {
            Constructor::True => "true".into(),
            Constructor::False => "false".into(),
            Constructor::Variant { entity, arity } => {
                let name = entity_name(query, *entity);
                if *arity == 0 {
                    format!(".{}", name)
                } else {
                    format!(".{}(_)", name)
                }
            }
            Constructor::Tuple { arity } => {
                let wildcards = vec!["_"; *arity].join(", ");
                format!("({})", wildcards)
            }
            Constructor::Struct { entity, .. } => {
                let name = entity_name(query, *entity);
                format!("{} {{ .. }}", name)
            }
            Constructor::IntLiteral(n) => n.to_string(),
            Constructor::IntRange { start, end } => {
                let s = start.map(|v| v.to_string()).unwrap_or_default();
                let e = end.map(|v| v.to_string()).unwrap_or_default();
                if end.is_none() {
                    format!("{}..", s)
                } else {
                    format!("{}..={}", s, e)
                }
            }
            Constructor::CharLiteral(c) => format!("'{}'", c),
            Constructor::CharRange { start, end } => {
                let s = start.map(|c| format!("'{}'", c)).unwrap_or_default();
                let e = end.map(|c| format!("'{}'", c)).unwrap_or_default();
                if end.is_none() {
                    format!("{}..", s)
                } else {
                    format!("{}..={}", s, e)
                }
            }
            Constructor::StringLiteral(s) => format!("\"{}\"", s),
            Constructor::Unit => "()".into(),
            Constructor::Wildcard | Constructor::NonExhaustive | Constructor::Missing => {
                "_".into()
            }
            Constructor::Array {
                prefix_len,
                suffix_len,
                has_rest,
            } => {
                let mut parts = vec!["_"; *prefix_len];
                if *has_rest {
                    parts.push("..");
                }
                parts.extend(vec!["_"; *suffix_len]);
                format!("[{}]", parts.join(", "))
            }
        }
    }

    /// Convert this constructor to a witness for error messages.
    pub fn to_witness(&self, query: &QueryContext<'_>) -> Witness {
        match self {
            Constructor::True => Witness::bool(true),
            Constructor::False => Witness::bool(false),
            Constructor::Variant { entity, arity } => {
                let name = entity_name(query, *entity);
                if *arity == 0 {
                    Witness::enum_case(&name)
                } else {
                    Witness::enum_case_with_args(&name, vec![Witness::any(); *arity])
                }
            }
            Constructor::Tuple { arity } => Witness::tuple(vec![Witness::any(); *arity]),
            Constructor::Struct { entity, .. } => {
                let name = entity_name(query, *entity);
                Witness::struct_witness(&name, vec![])
            }
            Constructor::IntLiteral(n) => Witness::integer(*n),
            Constructor::IntRange { start, end } => {
                let s = start.map(|v| v.to_string()).unwrap_or_default();
                let e = end.map(|v| v.to_string()).unwrap_or_default();
                if end.is_none() {
                    Witness::Literal(format!("{}..", s))
                } else {
                    Witness::range(s, e, true)
                }
            }
            Constructor::CharLiteral(c) => Witness::Literal(format!("'{}'", c)),
            Constructor::CharRange { start, end } => {
                let s = start.map(|c| format!("'{}'", c)).unwrap_or_default();
                let e = end.map(|c| format!("'{}'", c)).unwrap_or_default();
                if end.is_none() {
                    Witness::Literal(format!("{}..", s))
                } else {
                    Witness::range(s, e, true)
                }
            }
            Constructor::StringLiteral(s) => Witness::string(s),
            Constructor::Unit => Witness::tuple(vec![]),
            Constructor::Array {
                prefix_len,
                suffix_len,
                ..
            } => Witness::array(vec![Witness::any(); prefix_len + suffix_len]),
            Constructor::Wildcard | Constructor::NonExhaustive | Constructor::Missing => {
                Witness::any()
            }
        }
    }
}

// ===== TypeShape =====

/// Describes the constructor space of a type.
///
/// This is the ONE place where exhaustiveness rules live. To make a new
/// type exhaustively matchable, add a variant here and implement its
/// `constructors()` method.
#[derive(Debug, Clone)]
pub enum TypeShape {
    /// Two constructors: True, False (the Bool struct)
    Bool,
    /// One constructor per enum case
    Enum {
        cases: Vec<(Entity, usize)>, // (case_entity, arity)
    },
    /// Single constructor with N positional fields
    Tuple(usize),
    /// Single constructor with N named fields
    Struct { entity: Entity, field_count: usize },
    /// Single constructor: the unit value ()
    Unit,
    /// Zero constructors — uninhabited, always exhaustive
    Never,
    /// Infinite constructors (Int, String, Float, Array) — needs wildcard
    Infinite,
    /// Unresolvable — treated conservatively as infinite
    Unknown,
}

impl TypeShape {
    /// Classify a resolved type into its constructor space.
    ///
    /// This is the single entry point for type → exhaustiveness mapping.
    pub fn classify(query: &QueryContext<'_>, ty: &ResolvedTy) -> Self {
        match ty {
            ResolvedTy::Never => TypeShape::Never,

            ResolvedTy::Tuple(elems) => {
                if elems.is_empty() {
                    TypeShape::Unit // () has one constructor
                } else {
                    TypeShape::Tuple(elems.len())
                }
            }

            ResolvedTy::Named { entity, .. } => {
                let Some(kind) = query.get::<NodeKind>(*entity) else {
                    return TypeShape::Unknown;
                };

                match kind {
                    NodeKind::Enum => {
                        let cases: Vec<_> = query
                            .children_of(*entity)
                            .iter()
                            .filter(|&&child| {
                                matches!(query.get::<NodeKind>(child), Some(NodeKind::EnumCase))
                            })
                            .map(|&child| {
                                let arity = query
                                    .get::<Callable>(child)
                                    .map(|c| c.params.len())
                                    .unwrap_or(0);
                                (child, arity)
                            })
                            .collect();
                        TypeShape::Enum { cases }
                    }

                    NodeKind::Struct => {
                        // Bool is a struct that conforms to ExpressibleByBoolLiteral.
                        // We detect it by name — it's a builtin.
                        if is_bool_struct(query, *entity) {
                            return TypeShape::Bool;
                        }

                        // Array is infinite (variable length)
                        if is_array_struct(query, *entity) {
                            return TypeShape::Infinite;
                        }

                        let field_count = collect_fields(query, *entity).len();
                        TypeShape::Struct {
                            entity: *entity,
                            field_count,
                        }
                    }

                    // TypeAlias — resolve through to the target type
                    // For now, treat as unknown (aliases should be resolved by inference)
                    _ => TypeShape::Unknown,
                }
            }

            // Function types, type params, errors — can't enumerate constructors
            _ => TypeShape::Unknown,
        }
    }

    /// Get all constructors for this type shape. Returns `None` for infinite types.
    pub fn constructors(&self, _query: &QueryContext<'_>) -> Option<Vec<Constructor>> {
        match self {
            TypeShape::Bool => Some(vec![Constructor::True, Constructor::False]),

            TypeShape::Unit => Some(vec![Constructor::Unit]),

            TypeShape::Never => Some(vec![]), // zero constructors = always exhaustive

            TypeShape::Enum { cases } => Some(
                cases
                    .iter()
                    .map(|&(entity, arity)| Constructor::Variant { entity, arity })
                    .collect(),
            ),

            TypeShape::Tuple(arity) => Some(vec![Constructor::Tuple { arity: *arity }]),

            TypeShape::Struct {
                entity,
                field_count,
            } => Some(vec![Constructor::Struct {
                entity: *entity,
                arity: *field_count,
            }]),

            TypeShape::Infinite | TypeShape::Unknown => None,
        }
    }
}

// ===== Helpers =====

fn entity_name(query: &QueryContext<'_>, entity: Entity) -> String {
    query
        .get::<Name>(entity)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| format!("{:?}", entity))
}

/// Check if an entity is the Bool struct.
fn is_bool_struct(query: &QueryContext<'_>, entity: Entity) -> bool {
    query
        .get::<Name>(entity)
        .is_some_and(|n| n.0 == "Bool")
}

/// Check if an entity is the Array struct (has name "Array" and 1 type param).
fn is_array_struct(query: &QueryContext<'_>, entity: Entity) -> bool {
    query.get::<Name>(entity).is_some_and(|n| n.0 == "Array")
        && query
            .get::<TypeParams>(entity)
            .is_some_and(|tp| tp.0.len() == 1)
}

/// Check if a ResolvedTy is an Array[T] struct.
fn is_array_type(query: &QueryContext<'_>, ty: &ResolvedTy) -> bool {
    matches!(ty, ResolvedTy::Named { entity, .. } if is_array_struct(query, *entity))
}

/// Collect Field children of an entity, in declaration order.
pub(super) fn collect_fields(query: &QueryContext<'_>, entity: Entity) -> Vec<Entity> {
    query
        .children_of(entity)
        .iter()
        .filter(|&&child| matches!(query.get::<NodeKind>(child), Some(NodeKind::Field)))
        .copied()
        .collect()
}

/// Resolve an enum case parameter type, substituting the enum's type args.
///
/// Builds a name→type map from the enum's TypeParams + the concrete type args,
/// then looks up each case param's AstType. If the param type is a simple
/// named reference to a type parameter, substitutes it. Otherwise falls back
/// to the raw AstType resolution.
fn resolve_case_param_type(
    query: &QueryContext<'_>,
    enum_entity: Entity,
    type_args: &[ResolvedTy],
    param: &kestrel_ast_builder::AstParam,
) -> ResolvedTy {
    let subs = build_type_param_subs(query, enum_entity, type_args);
    resolve_ast_type_with_subs(query, param.ty.as_ref(), &subs)
}

/// Resolve a field's type with the parent struct's type arguments.
fn resolve_field_type(
    query: &QueryContext<'_>,
    field_entity: Entity,
    parent_entity: Entity,
    type_args: &[ResolvedTy],
) -> ResolvedTy {
    let subs = build_type_param_subs(query, parent_entity, type_args);
    let ast_ty = query
        .get::<TypeAnnotation>(field_entity)
        .map(|ta| &ta.0);
    resolve_ast_type_with_subs(query, ast_ty, &subs)
}

/// Build a mapping from type parameter names to their concrete types.
///
/// Given `enum Optional[T]` with args `[Int]`, returns `{"T" → Int}`.
fn build_type_param_subs(
    query: &QueryContext<'_>,
    parent_entity: Entity,
    type_args: &[ResolvedTy],
) -> Vec<(String, ResolvedTy)> {
    let Some(type_params) = query.get::<TypeParams>(parent_entity) else {
        return vec![];
    };
    type_params
        .0
        .iter()
        .zip(type_args.iter())
        .filter_map(|(&param_entity, arg_ty)| {
            let name = query.get::<Name>(param_entity)?.0.clone();
            Some((name, arg_ty.clone()))
        })
        .collect()
}

/// Resolve an AstType using a name→type substitution map.
///
/// Handles the common cases: simple named types that reference type params
/// (e.g., `T` in `case Some(T)`), tuples, optionals, arrays. Falls back
/// to Error for complex types that can't be resolved without full name resolution.
fn resolve_ast_type_with_subs(
    query: &QueryContext<'_>,
    ast_ty: Option<&kestrel_ast::AstType>,
    subs: &[(String, ResolvedTy)],
) -> ResolvedTy {
    use kestrel_ast::AstType;

    let Some(ty) = ast_ty else {
        return ResolvedTy::Error;
    };

    match ty {
        // Simple named type — check if it's a type parameter
        AstType::Named { segments, .. } => {
            if segments.len() == 1 && segments[0].type_args.is_empty() {
                // Single-segment, no type args → might be a type parameter
                let name = &segments[0].name;
                if let Some((_, resolved)) = subs.iter().find(|(n, _)| n == name) {
                    return resolved.clone();
                }
            }
            // Not a type parameter — can't fully resolve without name resolution.
            // Return Error as a conservative fallback.
            ResolvedTy::Error
        }

        AstType::Tuple(elems, _) => {
            let resolved: Vec<_> = elems
                .iter()
                .map(|e| resolve_ast_type_with_subs(query, Some(e), subs))
                .collect();
            ResolvedTy::Tuple(resolved)
        }

        AstType::Unit(_) => ResolvedTy::Tuple(vec![]),

        AstType::Never(_) => ResolvedTy::Never,

        // For Optional, Array, etc. we'd need the builtin entities to construct
        // Named types. Fall back to Error — these rarely appear as enum case
        // params or struct fields in practice.
        _ => ResolvedTy::Error,
    }
}

/// Check if two optional i64 ranges overlap.
fn ranges_overlap_i64(
    s1: Option<i64>,
    e1: Option<i64>,
    s2: Option<i64>,
    e2: Option<i64>,
) -> bool {
    let start_ok = match (s1, e2) {
        (Some(s), Some(e)) => s <= e,
        _ => true,
    };
    let end_ok = match (s2, e1) {
        (Some(s), Some(e)) => s <= e,
        _ => true,
    };
    start_ok && end_ok
}

/// Check if two optional char ranges overlap.
fn ranges_overlap_char(
    s1: Option<char>,
    e1: Option<char>,
    s2: Option<char>,
    e2: Option<char>,
) -> bool {
    let start_ok = match (s1, e2) {
        (Some(s), Some(e)) => s <= e,
        _ => true,
    };
    let end_ok = match (s2, e1) {
        (Some(s), Some(e)) => s <= e,
        _ => true,
    };
    start_ok && end_ok
}

/// Find missing array constructors given covered patterns.
fn missing_array_constructors(covered: &HashSet<Constructor>) -> Option<Vec<Constructor>> {
    let mut has_rest = false;
    let mut min_len_for_rest = usize::MAX;
    let mut fixed_lengths: HashSet<usize> = HashSet::new();

    for ctor in covered {
        if let Constructor::Array {
            prefix_len,
            suffix_len,
            has_rest: rest,
        } = ctor
        {
            let min_len = prefix_len + suffix_len;
            if *rest {
                has_rest = true;
                min_len_for_rest = min_len_for_rest.min(min_len);
            } else {
                fixed_lengths.insert(min_len);
            }
        }
    }

    if has_rest {
        // Rest pattern covers all lengths >= min_len_for_rest.
        // Check that lengths 0..min_len_for_rest are all covered.
        let missing: Vec<_> = (0..min_len_for_rest)
            .filter(|len| !fixed_lengths.contains(len))
            .map(|len| Constructor::Array {
                prefix_len: len,
                suffix_len: 0,
                has_rest: false,
            })
            .collect();
        Some(missing)
    } else {
        // No rest pattern — can't cover infinite lengths
        Some(vec![Constructor::NonExhaustive])
    }
}
