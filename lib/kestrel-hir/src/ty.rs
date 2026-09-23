//! Resolved type representations for HIR.
//!
//! Type-level sugar is resolved before reaching HIR:
//! `Int?` → `Struct(Optional, [Struct(Int)])`, `[Int]` → `Struct(Array, [Struct(Int)])`, etc.
//!
//! The `Named` variant is split into explicit Struct/Enum/Protocol/AliasUse variants
//! so that consumers can't silently forget to disambiguate. Abstract associated types
//! have their own variant (`AssocProjection`) that carries the receiver explicitly.

use kestrel_hecs::Entity;
use kestrel_span::Span;

/// A resolved type in HIR. All syntactic sugar has been expanded:
/// Optional, Array, Dictionary, Result are just `Struct` with the
/// appropriate entity and type arguments.
#[derive(Clone, Debug, Hash)]
pub enum HirTy {
    /// Struct type (includes Optional, Array, Dictionary, Result sugar).
    Struct {
        entity: Entity,
        args: Vec<HirTy>,
        span: Span,
    },
    /// Enum type.
    Enum {
        entity: Entity,
        args: Vec<HirTy>,
        span: Span,
    },
    /// Protocol type (used as a bound or, eventually, as an existential).
    Protocol {
        entity: Entity,
        args: Vec<HirTy>,
        span: Span,
    },
    /// Tuple type: `(Int, String)`
    Tuple(Vec<HirTy>, Span),
    /// Function type: `(Int, String) -> Bool` or `(mutating Grid) -> Unit`.
    /// `param_conventions` is parallel to `params`; `MutBorrow` marks a
    /// `mutating` parameter, otherwise `Consuming` (the pre-#106 default).
    /// `kind` is the closure tier named by an optional keyword prefix
    /// (`escaping (Int) -> Bool`) and is orthogonal to `param_conventions`.
    Function {
        kind: kestrel_ast::FnTypeKind,
        params: Vec<HirTy>,
        param_conventions: Vec<kestrel_ast::ParamConvention>,
        ret: Box<HirTy>,
        span: Span,
    },
    /// Use of a regular (non-associated) type alias. Inference reduces this
    /// to the substituted definition and emits any bound obligations.
    AliasUse {
        entity: Entity,
        args: Vec<HirTy>,
        span: Span,
    },
    /// Type parameter resolved to its declaring entity.
    Param(Entity, Span),
    /// `Self` inside a protocol declaration or `extend <protocol>` — the
    /// abstract implementing type. Resolves to a concrete type at
    /// monomorphization time via `MirTy::SelfType`. Only used in contexts
    /// where the enclosing "Self" is a protocol; extensions on concrete
    /// types emit `Struct`/`Enum` directly. Carries the owning protocol
    /// entity so every lowering site can substitute it without relying on
    /// ambient context (e.g. where-clause RHS lowering from an outer body).
    SelfType(Entity, Span),
    /// Abstract associated-type projection: `base.assoc` (e.g. `T.Item`, `Self.Output`).
    /// `base` is the receiver type; `assoc` is the TypeAlias entity on the protocol.
    /// Nested projections chain naturally: `T.Next.Next` is AssocProjection over AssocProjection.
    AssocProjection {
        base: Box<HirTy>,
        assoc: Entity,
        span: Span,
    },
    /// Opaque type: `some P`, `some P and Q`. Bounds are protocol types.
    /// Lowered from `AstType::Some`. Carries resolved protocol bounds.
    /// `not_copyable` records an `and not Copyable` negative bound: the
    /// concrete underlier may be move-only, and use sites must treat the
    /// opaque value as NotCopyable.
    Opaque {
        bounds: Vec<HirTy>,
        not_copyable: bool,
        span: Span,
    },
    /// Never type (diverging expressions, e.g. `panic()`)
    Never(Span),
    /// Inferred type (user wrote `_` or omitted)
    Infer(Span),
    /// Error recovery
    Error(Span),
    /// Reference type: `&T` / `&mutating T`. Survives HIR lowering only at
    /// the positions `reject_ref_types` accepts — return types (stage 1)
    /// and field / enum-payload / tuple-element / generic-argument slots
    /// (stage 2b, `RefPolicy::AllowAggregate` entries); every other
    /// occurrence is rewritten to `Error` with a diagnostic at the
    /// lowering-query boundaries. Bare params (E480), bindings (E482),
    /// fn-type returns (E486), nesting (E487), alias RHS / protocol args /
    /// where-clause types (Strict entries) still reject.
    Ref {
        inner: Box<HirTy>,
        mutating: bool,
        span: Span,
    },
}

impl HirTy {
    /// Span-insensitive structural equality: do `self` and `other` denote the
    /// same type as written?
    ///
    /// Deliberately **not** a derived `PartialEq`: every variant carries a
    /// `Span`, so a derive would call two identical clauses written on
    /// different lines unequal. Every other field is compared exactly.
    ///
    /// `Infer` and `Error` never equal anything, themselves included: neither
    /// names a type, so "the same" is unknowable, and answering `false` is the
    /// conservative direction for every caller (G25 step 4 entailment).
    pub fn same_type(&self, other: &HirTy) -> bool {
        fn all(a: &[HirTy], b: &[HirTy]) -> bool {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.same_type(y))
        }
        match (self, other) {
            (
                HirTy::Struct {
                    entity: e1,
                    args: a1,
                    ..
                },
                HirTy::Struct {
                    entity: e2,
                    args: a2,
                    ..
                },
            )
            | (
                HirTy::Enum {
                    entity: e1,
                    args: a1,
                    ..
                },
                HirTy::Enum {
                    entity: e2,
                    args: a2,
                    ..
                },
            )
            | (
                HirTy::Protocol {
                    entity: e1,
                    args: a1,
                    ..
                },
                HirTy::Protocol {
                    entity: e2,
                    args: a2,
                    ..
                },
            )
            | (
                HirTy::AliasUse {
                    entity: e1,
                    args: a1,
                    ..
                },
                HirTy::AliasUse {
                    entity: e2,
                    args: a2,
                    ..
                },
            ) => e1 == e2 && all(a1, a2),
            (HirTy::Tuple(a1, _), HirTy::Tuple(a2, _)) => all(a1, a2),
            (
                HirTy::Function {
                    kind: k1,
                    params: p1,
                    param_conventions: c1,
                    ret: r1,
                    ..
                },
                HirTy::Function {
                    kind: k2,
                    params: p2,
                    param_conventions: c2,
                    ret: r2,
                    ..
                },
            ) => k1 == k2 && c1 == c2 && all(p1, p2) && r1.same_type(r2),
            (HirTy::Param(e1, _), HirTy::Param(e2, _))
            | (HirTy::SelfType(e1, _), HirTy::SelfType(e2, _)) => e1 == e2,
            (
                HirTy::AssocProjection {
                    base: b1,
                    assoc: s1,
                    ..
                },
                HirTy::AssocProjection {
                    base: b2,
                    assoc: s2,
                    ..
                },
            ) => s1 == s2 && b1.same_type(b2),
            (
                HirTy::Opaque {
                    bounds: b1,
                    not_copyable: n1,
                    ..
                },
                HirTy::Opaque {
                    bounds: b2,
                    not_copyable: n2,
                    ..
                },
            ) => n1 == n2 && all(b1, b2),
            (HirTy::Never(_), HirTy::Never(_)) => true,
            (
                HirTy::Ref {
                    inner: i1,
                    mutating: m1,
                    ..
                },
                HirTy::Ref {
                    inner: i2,
                    mutating: m2,
                    ..
                },
            ) => m1 == m2 && i1.same_type(i2),
            _ => false,
        }
    }
}
