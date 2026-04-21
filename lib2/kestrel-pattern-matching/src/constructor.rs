//! # Constructor and Type Classification
//!
//! A "constructor" is the head of a pattern — the way to build a value of a type.
//! For exhaustiveness, we need to enumerate all constructors of a type.
//!
//! ## Constructors by Type
//!
//! | Type | Constructors | Exhaustive? |
//! |------|-------------|-------------|
//! | `Bool` | `True`, `False` | Yes (2) |
//! | `enum E { A, B(T) }` | `Variant(A, 0)`, `Variant(B, 1)` | Yes (N cases) |
//! | `(T, U)` | `Tuple(2)` | Yes (1) |
//! | `struct S { x, y }` | `Struct(S, 2)` | Yes (1) |
//! | `()` | `Unit` | Yes (1) |
//! | `Never` | (none) | Yes (0) |
//! | `Int64`, `String`, `Float` | infinite | No — needs `_` |
//! | `Array[T]` | variable length | No — needs `[..]` |
//!
//! ## TypeShape — Extensibility Point
//!
//! `TypeShape::classify` is the single place where type → constructor-space
//! mapping is defined. To make a new type exhaustively matchable (e.g.,
//! single-variant enums), add a match arm there.
//!
//! ## Key Methods (each exists once, no duplicates)
//!
//! - `Constructor::matches()` — compatibility check for specialization
//! - `Constructor::field_types()` — sub-pattern types for a constructor
//! - `Constructor::all_for_type()` — enumerate all constructors (via TypeShape)

use std::collections::HashSet;

use kestrel_ast_builder::{Callable, Intrinsic, Name, NodeKind, TypeAnnotation, TypeParams};
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::Builtin;
use kestrel_name_res::{ConformingProtocols, ResolveBuiltin};
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
    IntRange {
        start: Option<i64>,
        end: Option<i64>,
    },
    /// Character literal
    CharLiteral(char),
    /// Character range (both ends inclusive, None = unbounded)
    CharRange {
        start: Option<char>,
        end: Option<char>,
    },
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
            },
            (Constructor::Struct { entity: e1, .. }, Constructor::Struct { entity: e2, .. }) => {
                e1 == e2
            },
            (Constructor::Tuple { arity: a1 }, Constructor::Tuple { arity: a2 }) => a1 == a2,

            // Integer comparisons
            (Constructor::IntLiteral(v1), Constructor::IntLiteral(v2)) => v1 == v2,
            (Constructor::IntLiteral(v), Constructor::IntRange { start, end }) => {
                start.map_or(true, |s| *v >= s) && end.map_or(true, |e| *v <= e)
            },
            (
                Constructor::IntRange { start: s1, end: e1 },
                Constructor::IntRange { start: s2, end: e2 },
            ) => ranges_overlap_i64(*s1, *e1, *s2, *e2),

            // Character comparisons
            (Constructor::CharLiteral(v1), Constructor::CharLiteral(v2)) => v1 == v2,
            (Constructor::CharLiteral(v), Constructor::CharRange { start, end }) => {
                start.map_or(true, |s| *v >= s) && end.map_or(true, |e| *v <= e)
            },
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
            },

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
            },

            (Constructor::Struct { entity, arity }, ResolvedTy::Named { args, .. }) => {
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
            },

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
            },

            _ => vec![parent_ty.clone(); self.arity()],
        }
    }

    /// Enumerate all constructors for a type. Returns `None` for infinite types.
    pub fn all_for_type(
        query: &QueryContext<'_>,
        root: Entity,
        ty: &ResolvedTy,
    ) -> Option<Vec<Constructor>> {
        let shape = TypeShape::classify(query, root, ty);
        shape.constructors(query)
    }

    /// Find constructors not covered by `covered`. Returns `None` if impossible.
    pub fn missing(
        query: &QueryContext<'_>,
        root: Entity,
        ty: &ResolvedTy,
        covered: &HashSet<Constructor>,
    ) -> Option<Vec<Constructor>> {
        if covered.contains(&Constructor::Wildcard) {
            return Some(vec![]);
        }

        match Constructor::all_for_type(query, root, ty) {
            Some(all) => {
                let missing: Vec<_> = all.into_iter().filter(|c| !covered.contains(c)).collect();
                Some(missing)
            },
            None => {
                // Infinite type — check for special cases like arrays
                if is_array_type(query, root, ty) {
                    return missing_array_constructors(covered);
                }
                // Need a wildcard to cover infinite types
                Some(vec![Constructor::NonExhaustive])
            },
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
            },
            Constructor::Tuple { arity } => {
                let wildcards = vec!["_"; *arity].join(", ");
                format!("({})", wildcards)
            },
            Constructor::Struct { entity, .. } => {
                let name = entity_name(query, *entity);
                format!("{} {{ .. }}", name)
            },
            Constructor::IntLiteral(n) => n.to_string(),
            Constructor::IntRange { start, end } => {
                let s = start.map(|v| v.to_string()).unwrap_or_default();
                let e = end.map(|v| v.to_string()).unwrap_or_default();
                if end.is_none() {
                    format!("{}..", s)
                } else {
                    format!("{}..={}", s, e)
                }
            },
            Constructor::CharLiteral(c) => format!("'{}'", c),
            Constructor::CharRange { start, end } => {
                let s = start.map(|c| format!("'{}'", c)).unwrap_or_default();
                let e = end.map(|c| format!("'{}'", c)).unwrap_or_default();
                if end.is_none() {
                    format!("{}..", s)
                } else {
                    format!("{}..={}", s, e)
                }
            },
            Constructor::StringLiteral(s) => format!("\"{}\"", s),
            Constructor::Unit => "()".into(),
            Constructor::Wildcard | Constructor::NonExhaustive | Constructor::Missing => "_".into(),
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
            },
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
            },
            Constructor::Tuple { arity } => Witness::tuple(vec![Witness::any(); *arity]),
            Constructor::Struct { entity, .. } => {
                let name = entity_name(query, *entity);
                Witness::struct_witness(&name, vec![])
            },
            Constructor::IntLiteral(n) => Witness::integer(*n),
            Constructor::IntRange { start, end } => {
                let s = start.map(|v| v.to_string()).unwrap_or_default();
                let e = end.map(|v| v.to_string()).unwrap_or_default();
                if end.is_none() {
                    Witness::Literal(format!("{}..", s))
                } else {
                    Witness::range(s, e, true)
                }
            },
            Constructor::CharLiteral(c) => Witness::Literal(format!("'{}'", c)),
            Constructor::CharRange { start, end } => {
                let s = start.map(|c| format!("'{}'", c)).unwrap_or_default();
                let e = end.map(|c| format!("'{}'", c)).unwrap_or_default();
                if end.is_none() {
                    Witness::Literal(format!("{}..", s))
                } else {
                    Witness::range(s, e, true)
                }
            },
            Constructor::StringLiteral(s) => Witness::string(s),
            Constructor::Unit => Witness::tuple(vec![]),
            Constructor::Array {
                prefix_len,
                suffix_len,
                ..
            } => Witness::array(vec![Witness::any(); prefix_len + suffix_len]),
            Constructor::Wildcard | Constructor::NonExhaustive | Constructor::Missing => {
                Witness::any()
            },
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
    pub fn classify(query: &QueryContext<'_>, root: Entity, ty: &ResolvedTy) -> Self {
        match ty {
            ResolvedTy::Never => TypeShape::Never,

            ResolvedTy::Tuple(elems) => {
                if elems.is_empty() {
                    TypeShape::Unit // () has one constructor
                } else {
                    TypeShape::Tuple(elems.len())
                }
            },

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
                    },

                    NodeKind::Struct => {
                        // Layer 1: lang.* intrinsic types (identified by Intrinsic marker)
                        if query.has::<Intrinsic>(*entity) {
                            return classify_intrinsic(query, *entity);
                        }

                        // Layer 2: stdlib types (identified by declared conformances)
                        if let Some(shape) = classify_by_conformances(query, root, *entity) {
                            return shape;
                        }

                        // Default: regular struct with single constructor
                        let field_count = collect_fields(query, *entity).len();
                        TypeShape::Struct {
                            entity: *entity,
                            field_count,
                        }
                    },

                    // TypeAlias — resolve through to the target type
                    // For now, treat as unknown (aliases should be resolved by inference)
                    _ => TypeShape::Unknown,
                }
            },

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

/// Classify a lang.* intrinsic type by its compiler-controlled name.
fn classify_intrinsic(query: &QueryContext<'_>, entity: Entity) -> TypeShape {
    let Some(name) = query.get::<Name>(entity) else {
        return TypeShape::Unknown;
    };
    match name.0.as_str() {
        "i1" => TypeShape::Bool,
        "i8" | "i16" | "i32" | "i64" => TypeShape::Infinite,
        "f16" | "f32" | "f64" => TypeShape::Infinite,
        "str" => TypeShape::Infinite,
        _ => TypeShape::Unknown,
    }
}

/// Classify a non-intrinsic struct by its declared protocol conformances.
/// Returns None for regular user structs with no special classification.
fn classify_by_conformances(
    query: &QueryContext<'_>,
    root: Entity,
    entity: Entity,
) -> Option<TypeShape> {
    let conforms_to =
        |builtin: Builtin| -> bool { conforms_to_builtin(query, root, entity, builtin) };

    // Bool-like: two constructors (True/False)
    if conforms_to(Builtin::ExpressibleByBoolLiteral) {
        return Some(TypeShape::Bool);
    }

    // Literal types have infinite value spaces. ExpressibleByArrayLiteral is
    // a stdlib wrapper that inherits from InternalExpressibleByArrayLiteral,
    // so checking the internal protocol suffices.
    if conforms_to(Builtin::ExpressibleByIntegerLiteral)
        || conforms_to(Builtin::ExpressibleByFloatLiteral)
        || conforms_to(Builtin::ExpressibleByCharLiteral)
        || conforms_to(Builtin::ExpressibleByStringLiteral)
        || conforms_to(Builtin::InternalExpressibleByArrayLiteral)
    {
        return Some(TypeShape::Infinite);
    }

    None
}

/// Test whether an entity transitively conforms to a builtin protocol.
/// Resolves the protocol's entity via ResolveBuiltin, then checks membership
/// in ConformingProtocols — no name-based matching.
fn conforms_to_builtin(
    query: &QueryContext<'_>,
    root: Entity,
    entity: Entity,
    builtin: Builtin,
) -> bool {
    let Some(proto) = query.query(ResolveBuiltin { builtin, root }) else {
        return false;
    };
    query
        .query(ConformingProtocols { entity, root })
        .contains(&proto)
}

/// Check if a ResolvedTy is an array-like type (supports variable-length array patterns).
fn is_array_type(query: &QueryContext<'_>, root: Entity, ty: &ResolvedTy) -> bool {
    let ResolvedTy::Named { entity, .. } = ty else {
        return false;
    };
    // Intrinsic types are never arrays
    if query.has::<Intrinsic>(*entity) {
        return false;
    }
    // ExpressibleByArrayLiteral inherits from _ExpressibleByArrayLiteral, so
    // ConformingProtocols surfaces the internal protocol for any conforming type.
    conforms_to_builtin(
        query,
        root,
        *entity,
        Builtin::InternalExpressibleByArrayLiteral,
    )
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
    resolve_ast_type_with_subs(query, param.ty.as_ref(), &subs, enum_entity)
}

/// Resolve a field's type with the parent struct's type arguments.
fn resolve_field_type(
    query: &QueryContext<'_>,
    field_entity: Entity,
    parent_entity: Entity,
    type_args: &[ResolvedTy],
) -> ResolvedTy {
    let subs = build_type_param_subs(query, parent_entity, type_args);
    let ast_ty = query.get::<TypeAnnotation>(field_entity).map(|ta| &ta.0);
    resolve_ast_type_with_subs(query, ast_ty, &subs, parent_entity)
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
/// Handles type parameter substitution (e.g., `T` → `Int`), and falls back
/// to scope-based lookup for concrete named types (e.g., `Point` → entity).
/// `scope_entity` is used to walk up the parent chain for name resolution.
fn resolve_ast_type_with_subs(
    query: &QueryContext<'_>,
    ast_ty: Option<&kestrel_ast::AstType>,
    subs: &[(String, ResolvedTy)],
    scope_entity: Entity,
) -> ResolvedTy {
    use kestrel_ast::AstType;

    let Some(ty) = ast_ty else {
        return ResolvedTy::Error;
    };

    match ty {
        AstType::Named { segments, .. } => {
            if segments.len() == 1 && segments[0].type_args.is_empty() {
                let name = &segments[0].name;
                // Try type parameter substitution first
                if let Some((_, resolved)) = subs.iter().find(|(n, _)| n == name) {
                    return resolved.clone();
                }
                // Try resolving as a sibling type in the parent scope
                if let Some(entity) = resolve_name_in_scope(query, name, scope_entity) {
                    // Recurse to resolve type args on the segments
                    return ResolvedTy::Named {
                        entity,
                        args: vec![],
                    };
                }
            }
            // Multi-segment or unresolved — conservative fallback
            ResolvedTy::Error
        },

        AstType::Tuple(elems, _) => {
            let resolved: Vec<_> = elems
                .iter()
                .map(|e| resolve_ast_type_with_subs(query, Some(e), subs, scope_entity))
                .collect();
            ResolvedTy::Tuple(resolved)
        },

        AstType::Unit(_) => ResolvedTy::Tuple(vec![]),

        AstType::Never(_) => ResolvedTy::Never,

        // For Optional, Array, etc. we'd need the builtin entities to construct
        // Named types. Fall back to Error — these rarely appear as enum case
        // params or struct fields in practice.
        _ => ResolvedTy::Error,
    }
}

/// Resolve a single-segment name by searching up the parent chain.
/// Looks for structs/enums with the given name among siblings of scope_entity's ancestors.
fn resolve_name_in_scope(
    query: &QueryContext<'_>,
    name: &str,
    scope_entity: Entity,
) -> Option<Entity> {
    // Walk up the parent chain looking for a child with this name
    let mut current = Some(scope_entity);
    while let Some(parent) = current {
        for &child in query.children_of(parent) {
            if query.get::<Name>(child).is_some_and(|n| n.0 == name) {
                let kind = query.get::<NodeKind>(child);
                if matches!(kind, Some(NodeKind::Struct | NodeKind::Enum)) {
                    return Some(child);
                }
            }
        }
        current = query.parent_of(parent);
    }
    None
}

/// Check if two optional i64 ranges overlap.
fn ranges_overlap_i64(s1: Option<i64>, e1: Option<i64>, s2: Option<i64>, e2: Option<i64>) -> bool {
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
