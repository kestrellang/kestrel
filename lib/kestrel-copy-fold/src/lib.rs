//! The copy-semantics decision tree — ONE kernel, N data sources.
//!
//! Every layer that classifies a type as Copyable / Cloneable / NotCopyable
//! (kestrel-semantics over `HirTy`, the inference solver over `TyKind`,
//! kestrel-analyze move tracking over `ResolvedTy`, kestrel-mir `ty_query` and
//! `mono` over `TyId`) shares exactly two pieces of logic:
//!
//! - the *member fold* (`fold_members`): NotCopyable dominates, else any
//!   Cloneable → Cloneable, else Copyable; and
//! - the *nominal-instance rule* (`instance_semantics`): an unconditional base
//!   wins; a `not Copyable` base with conditional-Copyable gating positions
//!   folds the gating type args.
//!
//! Those live here and ONLY here. Everything else — type-shape classification,
//! query plumbing, caches, recursion guards, context choice — stays per layer
//! behind the `CopyLayer` hook trait. Any intentional per-layer divergence
//! must carry a `TODO(copy-drift #n)` comment at its classifier arm and never
//! be converged silently (#1-#5 were adjudicated and resolved 2026-06-10).
//!
//! This crate is a dependency-graph leaf (only `kestrel-hecs`, for `Entity`)
//! so both the frontend (via kestrel-semantics re-exports) and kestrel-mir can
//! reach it without coupling their build closures.

use std::borrow::Cow;

use kestrel_hecs::Entity;

/// Tri-state copy classification. Moved from kestrel-semantics (re-exported
/// there); the shared vocabulary of every layer's copy question.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CopySemantics {
    Copyable,
    Cloneable,
    NotCopyable,
}

/// What a type-param bound demands. Moved from kestrel-semantics (re-exported).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CopyRequirement {
    RequiresCopyable,
    RequiresCloneable,
    MayBeNonCopyable,
}

/// Dedups the identical 3-arm mapping previously triplicated in
/// `hir_type_copy_semantics`, `type_conforms_copyable`, and
/// `resolved_ty_copy_semantics`.
impl From<CopyRequirement> for CopySemantics {
    fn from(r: CopyRequirement) -> Self {
        match r {
            CopyRequirement::RequiresCloneable => CopySemantics::Cloneable,
            CopyRequirement::RequiresCopyable => CopySemantics::Copyable,
            CopyRequirement::MayBeNonCopyable => CopySemantics::NotCopyable,
        }
    }
}

/// A layer-native semantics value projecting to the shared tri-state.
/// `CopySemantics` itself for frontend layers; `CopyBehavior` for MIR, whose
/// `Clone(Entity)` payloads vary by producer (type entity from lowering,
/// Cloneable proto from the clone-shim pass) and MUST survive the
/// base-passthrough path of `instance_semantics` untouched.
pub trait CopySem {
    fn class(&self) -> CopySemantics;
}

impl CopySem for CopySemantics {
    fn class(&self) -> CopySemantics {
        *self
    }
}

/// One copy-semantics layer (HirTy / TyKind / ResolvedTy / MirTy). Implementors
/// supply data access and per-shape classification; the DECISION TREE lives in
/// this crate and only here.
///
/// ADDING A TYPE VARIANT? Each layer's `member_semantics` is an exhaustive
/// match over its type enum — the compiler forces a decision per variant.
/// Every arm must be one of:
///   nominal           -> `instance_semantics(self, entity, args)`
///   aggregate         -> `fold_members(...)` (or a documented layer quirk + TODO(copy-drift))
///   leaf policy       -> an explicit `CopySemantics::X` with a comment saying why
/// Never add a `_ =>` catch-all.
pub trait CopyLayer {
    /// The layer's type representation (HirTy, TyVar, ResolvedTy, TyId).
    type Ty;
    /// The layer's semantics value (CopySemantics; CopyBehavior in MIR).
    type Sem: CopySem;

    /// Base (generic-entity) semantics. Hook because plumbing differs:
    /// NominalCopySemantics query — with the semantics layer's re-entrancy
    /// guard — vs MIR's precomputed `type_info.copy`.
    fn base_semantics(&self, entity: Entity) -> Self::Sem;

    /// Gating positions for conditional copyability (ConditionalCopyableParams
    /// query / precomputed `conditionally_copyable`). Cow: Borrowed for MIR,
    /// Owned for query layers. Invariant (enforced by the query): non-empty
    /// only when the base is NotCopyable.
    fn gating_positions(&self, entity: Entity) -> Cow<'_, [usize]>;

    /// Classify one member/arg type — the layer's exhaustive match, including
    /// its own recursion bookkeeping (solver depth, MIR where_clause threading).
    fn member_semantics(&self, ty: &Self::Ty) -> Self::Sem;

    /// Build the layer's native value for a kernel-decided class on `entity`
    /// (MIR: Cloneable -> Clone(container entity); frontend: identity).
    fn sem_from_class(&self, entity: Entity, class: CopySemantics) -> Self::Sem;
}

/// Canonical aggregate fold: NotCopyable dominates; else Cloneable if any;
/// else Copyable. Used by every layer's gating-arg fold and tuple rule
/// (drift #2-#4 converged 2026-06-10; MIR folds keep their native Clone
/// payload at the classifier, so they inline this rule rather than call it).
pub fn fold_members(parts: impl IntoIterator<Item = CopySemantics>) -> CopySemantics {
    let mut saw_cloneable = false;
    for p in parts {
        match p {
            CopySemantics::NotCopyable => return CopySemantics::NotCopyable,
            CopySemantics::Cloneable => saw_cloneable = true,
            CopySemantics::Copyable => {},
        }
    }
    if saw_cloneable {
        CopySemantics::Cloneable
    } else {
        CopySemantics::Copyable
    }
}

/// THE shared decision tree: per-instantiation copy semantics of nominal
/// `entity[args]`. The single source of truth replacing the former
/// per-crate folds (`nominal_instance_copy_semantics` ×2,
/// `nominal_conforms_copyable`, `instantiated_copy_behavior`,
/// `conditional_copy`).
pub fn instance_semantics<L: CopyLayer>(layer: &L, entity: Entity, args: &[L::Ty]) -> L::Sem {
    // 1. Unconditional base wins — returned NATIVELY (see CopySem docs:
    //    MIR Clone payloads must not be normalized through the tri-state).
    let base = layer.base_semantics(entity);
    if base.class() != CopySemantics::NotCopyable {
        return base;
    }
    // 2. A `not Copyable` base is conditionally Copyable only with gating positions.
    let positions = layer.gating_positions(entity);
    if positions.is_empty() {
        return layer.sem_from_class(entity, CopySemantics::NotCopyable);
    }
    // 3. Fold gating args; a missing/out-of-range position is unprovable -> NotCopyable.
    let class = fold_members(positions.iter().map(|&pos| {
        args.get(pos)
            .map_or(CopySemantics::NotCopyable, |a| layer.member_semantics(a).class())
    }));
    layer.sem_from_class(entity, class)
}

#[cfg(test)]
mod tests {
    use super::*;
    use CopySemantics::{Cloneable, Copyable, NotCopyable};

    #[test]
    fn fold_members_table() {
        // (parts, expected)
        let cases: &[(&[CopySemantics], CopySemantics)] = &[
            (&[], Copyable), // empty fold is vacuously Copyable
            (&[Copyable, Copyable], Copyable),
            (&[Copyable, Cloneable], Cloneable),
            (&[Cloneable, Copyable], Cloneable), // order-independent
            (&[Cloneable, NotCopyable], NotCopyable),
            (&[NotCopyable, Cloneable], NotCopyable), // NC dominates Cloneable
            (&[NotCopyable], NotCopyable),
        ];
        for (parts, want) in cases {
            assert_eq!(fold_members(parts.iter().copied()), *want, "parts: {parts:?}");
        }
    }

    /// Toy layer: payload-bearing Sem proving the base-passthrough path
    /// returns the layer's native value untouched.
    #[derive(Clone, Debug, PartialEq)]
    enum ToySem {
        Plain(CopySemantics),
        /// A payload (like MIR's `Clone(Entity)`) that must survive passthrough.
        Tagged(CopySemantics, u32),
    }
    impl CopySem for ToySem {
        fn class(&self) -> CopySemantics {
            match self {
                ToySem::Plain(c) | ToySem::Tagged(c, _) => *c,
            }
        }
    }

    struct ToyLayer {
        base: ToySem,
        gating: Vec<usize>,
    }
    impl CopyLayer for ToyLayer {
        type Ty = CopySemantics;
        type Sem = ToySem;
        fn base_semantics(&self, _: Entity) -> ToySem {
            self.base.clone()
        }
        fn gating_positions(&self, _: Entity) -> Cow<'_, [usize]> {
            Cow::Borrowed(&self.gating)
        }
        fn member_semantics(&self, ty: &CopySemantics) -> ToySem {
            ToySem::Plain(*ty)
        }
        fn sem_from_class(&self, _: Entity, c: CopySemantics) -> ToySem {
            ToySem::Plain(c)
        }
    }

    #[test]
    fn unconditional_base_passes_through_natively() {
        let layer = ToyLayer {
            base: ToySem::Tagged(Cloneable, 42),
            gating: vec![0],
        };
        let e = Entity::from_raw(1);
        // Payload preserved — NOT normalized through the tri-state.
        assert_eq!(
            instance_semantics(&layer, e, &[NotCopyable]),
            ToySem::Tagged(Cloneable, 42)
        );
    }

    #[test]
    fn not_copyable_base_without_gating_is_not_copyable() {
        let layer = ToyLayer {
            base: ToySem::Plain(NotCopyable),
            gating: vec![],
        };
        let e = Entity::from_raw(1);
        assert_eq!(
            instance_semantics(&layer, e, &[Copyable]),
            ToySem::Plain(NotCopyable)
        );
    }

    #[test]
    fn gating_args_fold() {
        let e = Entity::from_raw(1);
        let layer = ToyLayer {
            base: ToySem::Plain(NotCopyable),
            gating: vec![0, 2],
        };
        // Non-gating position 1 is ignored.
        assert_eq!(
            instance_semantics(&layer, e, &[Copyable, NotCopyable, Copyable]),
            ToySem::Plain(Copyable)
        );
        assert_eq!(
            instance_semantics(&layer, e, &[Copyable, Copyable, Cloneable]),
            ToySem::Plain(Cloneable)
        );
        assert_eq!(
            instance_semantics(&layer, e, &[NotCopyable, Copyable, Copyable]),
            ToySem::Plain(NotCopyable)
        );
    }

    #[test]
    fn missing_gating_arg_is_not_copyable() {
        let e = Entity::from_raw(1);
        let layer = ToyLayer {
            base: ToySem::Plain(NotCopyable),
            gating: vec![1],
        };
        // args too short for gating position 1 → unprovable → NotCopyable.
        assert_eq!(
            instance_semantics(&layer, e, &[Copyable]),
            ToySem::Plain(NotCopyable)
        );
    }
}
