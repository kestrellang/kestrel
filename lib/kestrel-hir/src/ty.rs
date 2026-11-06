//! Resolved type representations for HIR.
//!
//! Type-level sugar is resolved before reaching HIR:
//! `Int?` → `Struct(Optional, [Struct(Int)])`, `[Int]` → `Struct(Array, [Struct(Int)])`, etc.
//!
//! The `Named` variant is split into explicit Struct/Enum/Protocol variants
//! so that consumers can't silently forget to disambiguate. Abstract associated types
//! have their own variants (`AssocProjection` carries the receiver explicitly,
//! `AbstractAssoc` is the receiverless form). Type aliases are expanded during
//! lowering: there is no variant for an unexpanded alias; `Alias` (bounded
//! aliases only) always carries its expansion.

use kestrel_hecs::Entity;
use kestrel_span::Span;

/// `Item = Int64` in `any Iterator[Item = Int64]`: `assoc` is the associated
/// type's declaration (a protocol's `TypeAlias` child).
#[derive(Clone, Debug, Hash)]
pub struct HirAssocBinding {
    pub assoc: Entity,
    pub ty: HirTy,
}

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
    /// An abstract associated type named without a receiver (a protocol's
    /// `type Item` with no definition, as type resolution reports it).
    /// Never a regular alias: those are expanded during lowering.
    AbstractAssoc { entity: Entity, span: Span },
    /// A use of a bounded type alias
    /// (`Keyed[Int64]` with `type Keyed[K] where K: Equatable = Array[K]`).
    /// `expanded` is the definition with `args` substituted and is always
    /// present, so an alias cannot reach later stages unexpanded: every
    /// consumer uses `expanded`. `alias` and `args` remain only for the bound
    /// check. An unbounded alias is lowered to its expansion directly.
    Alias {
        alias: Entity,
        args: Vec<HirTy>,
        expanded: Box<HirTy>,
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
    /// The raw existential `any P and Q`: some unknown type conforming to
    /// every bound, with the associated types in `bindings` pinned. Unsized
    /// and unspellable — it only ever appears as a box's type argument, since
    /// `any[B] P` lowers to `Struct { B, [RawExistential] }`. Bounds are
    /// `Protocol`s in canonical (entity) order, so `any P and Q` and
    /// `any Q and P` are the same type.
    RawExistential {
        bounds: Vec<HirTy>,
        bindings: Vec<HirAssocBinding>,
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
                HirTy::Alias {
                    alias: e1,
                    args: a1,
                    ..
                },
                HirTy::Alias {
                    alias: e2,
                    args: a2,
                    ..
                },
            ) => e1 == e2 && all(a1, a2),
            (HirTy::AbstractAssoc { entity: e1, .. }, HirTy::AbstractAssoc { entity: e2, .. }) => {
                e1 == e2
            },
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
            (
                HirTy::RawExistential {
                    bounds: b1,
                    bindings: p1,
                    ..
                },
                HirTy::RawExistential {
                    bounds: b2,
                    bindings: p2,
                    ..
                },
            ) => {
                all(b1, b2)
                    && p1.len() == p2.len()
                    && p1
                        .iter()
                        .zip(p2)
                        .all(|(x, y)| x.assoc == y.assoc && x.ty.same_type(&y.ty))
            },
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

    /// Rebuild this type with every `Param(e)` renamed to `rename(e)`, at any
    /// depth. `rename` returning `None` aborts the whole rewrite with `None`.
    ///
    /// Renaming only: a parameter can become another parameter, never a
    /// concrete type. That keeps the result exact — no lossy bridge from an
    /// inference-side type is involved — which is what entailment's
    /// `same_type` comparison needs (G29, `substitute_clause`). Spans, and
    /// every non-`Param` leaf (`SelfType` included), are kept as written.
    pub fn try_rename_params(
        &self,
        rename: &mut impl FnMut(Entity) -> Option<Entity>,
    ) -> Option<HirTy> {
        self.try_map_params(&mut |e, span| Some(HirTy::Param(rename(e)?, span.clone())))
    }

    /// Replace each `Param(e)` that `subs` maps with its type (an alias's
    /// parameters with the use site's arguments); other params stay.
    pub fn substitute_params(&self, subs: &[(Entity, HirTy)]) -> HirTy {
        self.try_map_params(&mut |e, span| {
            Some(
                subs.iter()
                    .find(|(p, _)| *p == e)
                    .map_or_else(|| HirTy::Param(e, span.clone()), |(_, t)| t.clone()),
            )
        })
        .expect("substitution never fails")
    }

    /// Rebuild this type with every `Param(e)` replaced by `map(e)`, at any
    /// depth; `None` from `map` aborts the whole rebuild.
    pub fn try_map_params(
        &self,
        map: &mut impl FnMut(Entity, &Span) -> Option<HirTy>,
    ) -> Option<HirTy> {
        fn list(
            tys: &[HirTy],
            map: &mut impl FnMut(Entity, &Span) -> Option<HirTy>,
        ) -> Option<Vec<HirTy>> {
            tys.iter().map(|t| t.try_map_params(map)).collect()
        }
        Some(match self {
            HirTy::Param(e, span) => map(*e, span)?,
            HirTy::Struct { entity, args, span } => HirTy::Struct {
                entity: *entity,
                args: list(args, map)?,
                span: span.clone(),
            },
            HirTy::Enum { entity, args, span } => HirTy::Enum {
                entity: *entity,
                args: list(args, map)?,
                span: span.clone(),
            },
            HirTy::Protocol { entity, args, span } => HirTy::Protocol {
                entity: *entity,
                args: list(args, map)?,
                span: span.clone(),
            },
            HirTy::Alias {
                alias,
                args,
                expanded,
                span,
            } => HirTy::Alias {
                alias: *alias,
                args: list(args, map)?,
                expanded: Box::new(expanded.try_map_params(map)?),
                span: span.clone(),
            },
            HirTy::AbstractAssoc { entity, span } => HirTy::AbstractAssoc {
                entity: *entity,
                span: span.clone(),
            },
            HirTy::Tuple(elems, span) => HirTy::Tuple(list(elems, map)?, span.clone()),
            HirTy::Function {
                kind,
                params,
                param_conventions,
                ret,
                span,
            } => HirTy::Function {
                kind: *kind,
                params: list(params, map)?,
                param_conventions: param_conventions.clone(),
                ret: Box::new(ret.try_map_params(map)?),
                span: span.clone(),
            },
            HirTy::AssocProjection { base, assoc, span } => HirTy::AssocProjection {
                base: Box::new(base.try_map_params(map)?),
                assoc: *assoc,
                span: span.clone(),
            },
            HirTy::Opaque {
                bounds,
                not_copyable,
                span,
            } => HirTy::Opaque {
                bounds: list(bounds, map)?,
                not_copyable: *not_copyable,
                span: span.clone(),
            },
            HirTy::RawExistential {
                bounds,
                bindings,
                span,
            } => HirTy::RawExistential {
                bounds: list(bounds, map)?,
                bindings: bindings
                    .iter()
                    .map(|b| {
                        Some(HirAssocBinding {
                            assoc: b.assoc,
                            ty: b.ty.try_map_params(map)?,
                        })
                    })
                    .collect::<Option<_>>()?,
                span: span.clone(),
            },
            HirTy::Ref {
                inner,
                mutating,
                span,
            } => HirTy::Ref {
                inner: Box::new(inner.try_map_params(map)?),
                mutating: *mutating,
                span: span.clone(),
            },
            leaf @ (HirTy::SelfType(..) | HirTy::Never(_) | HirTy::Infer(_) | HirTy::Error(_)) => {
                leaf.clone()
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(n: u32) -> Entity {
        Entity::from_raw(n)
    }

    fn sp() -> Span {
        Span::synthetic(0)
    }

    /// `Array[(A, B.Item)]` with A→X, B→Y renames at every depth and keeps
    /// the non-param entities as written.
    #[test]
    fn try_rename_params_renames_nested_params() {
        let (a, b, x, y, array, item) = (e(1), e(2), e(3), e(4), e(10), e(11));
        let ty = HirTy::Struct {
            entity: array,
            args: vec![HirTy::Tuple(
                vec![
                    HirTy::Param(a, sp()),
                    HirTy::AssocProjection {
                        base: Box::new(HirTy::Param(b, sp())),
                        assoc: item,
                        span: sp(),
                    },
                ],
                sp(),
            )],
            span: sp(),
        };
        let renamed = ty
            .try_rename_params(&mut |p| {
                Some(if p == a {
                    x
                } else if p == b {
                    y
                } else {
                    p
                })
            })
            .expect("every param renames");
        let expected = HirTy::Struct {
            entity: array,
            args: vec![HirTy::Tuple(
                vec![
                    HirTy::Param(x, sp()),
                    HirTy::AssocProjection {
                        base: Box::new(HirTy::Param(y, sp())),
                        assoc: item,
                        span: sp(),
                    },
                ],
                sp(),
            )],
            span: sp(),
        };
        assert!(renamed.same_type(&expected));
    }

    /// One param that cannot be renamed aborts the whole rewrite.
    #[test]
    fn try_rename_params_aborts_on_none() {
        let (a, b) = (e(1), e(2));
        let ty = HirTy::Tuple(vec![HirTy::Param(a, sp()), HirTy::Param(b, sp())], sp());
        assert!(
            ty.try_rename_params(&mut |p| (p == a).then_some(p))
                .is_none()
        );
    }
}
