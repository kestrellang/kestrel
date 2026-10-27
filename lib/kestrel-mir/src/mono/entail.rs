//! Mono's view for the `kestrel-entail` kernel (bidi design §7.2, "MIR
//! adapter").
//!
//! Mono keeps its own structural matcher (`witness::match_pattern`) for
//! implementing types — it understands MIR-only shapes such as `Pointer` and
//! function types. The *where clauses* of a matched witness, equality clauses
//! included, are decided by the kernel: before this, `Equality` was dropped at
//! lowering and an unbound param was permitted by a private evaluator.
//!
//! Everything at mono is ground, so `Unknown` can only come from a clause on a
//! param the implementing type never binds (a free extension param) or from
//! an associated type this layer cannot reduce. Those keep mono's old
//! conservative answer: permit.

use std::cell::RefCell;
use std::collections::HashMap;

use kestrel_entail::{ETy, EntailLayer, Goal, Source, Supply, Tri};
use kestrel_hecs::Entity;

use crate::TyId;
use crate::item::function::WhereConstraint;
use crate::item::witness::WitnessDef;
use crate::ty::{MirTy, TyArena};

use super::witness::match_pattern;

pub(crate) struct MonoLayer<'a> {
    arena: &'a TyArena,
    witnesses: &'a [WitnessDef],
    /// Kernel type → the arena type it was converted from, so `supply` can
    /// run mono's own matcher on it.
    origin: RefCell<HashMap<ETy, TyId>>,
}

impl<'a> MonoLayer<'a> {
    pub(crate) fn new(arena: &'a TyArena, witnesses: &'a [WitnessDef]) -> Self {
        Self {
            arena,
            witnesses,
            origin: RefCell::new(HashMap::new()),
        }
    }

    /// Kernel view of an arena type, with the witness pattern's bindings
    /// substituted for its params.
    fn ety(&self, ty: TyId, bindings: &HashMap<Entity, TyId>) -> ETy {
        let out = match self.arena.get(ty) {
            MirTy::TypeParam(e) => match bindings.get(e) {
                Some(&bound) => return self.ety(bound, &HashMap::new()),
                None => ETy::Param(*e),
            },
            MirTy::Named { entity, type_args } => ETy::Nominal {
                entity: *entity,
                args: type_args.iter().map(|&a| self.ety(a, bindings)).collect(),
            },
            MirTy::Tuple(elems) => {
                ETy::Tuple(elems.iter().map(|&a| self.ety(a, bindings)).collect())
            },
            MirTy::Ref { pointee, mutating } => ETy::Ref {
                inner: Box::new(self.ety(*pointee, bindings)),
                mutating: *mutating,
            },
            MirTy::AssociatedProjection {
                base, assoc_type, ..
            } => ETy::Proj {
                base: Box::new(self.ety(*base, bindings)),
                assoc: *assoc_type,
            },
            MirTy::Never => ETy::Never,
            MirTy::Error => ETy::Poison,
            // Primitives, pointers, function types: interned leaves. Equal
            // handles ⇔ equal types (see `equal_abstract`).
            _ => ETy::Abstract(ty.raw()),
        };
        if !matches!(out, ETy::Param(_)) {
            self.origin.borrow_mut().insert(out.clone(), ty);
        }
        out
    }

    /// A witness's where clauses as kernel goals, with its pattern bindings
    /// substituted. Pass bindings from matching its implementing type.
    fn goals(&self, witness: &WitnessDef, bindings: &HashMap<Entity, TyId>) -> Vec<Goal> {
        witness
            .constraints
            .iter()
            .map(|c| match c {
                WhereConstraint::Implements {
                    type_param,
                    protocol,
                    ..
                } => Goal::Conforms {
                    ty: self.param_ety(*type_param, bindings),
                    protocol: *protocol,
                },
                WhereConstraint::NotImplements {
                    type_param,
                    protocol,
                } => Goal::NotConforms {
                    ty: self.param_ety(*type_param, bindings),
                    protocol: *protocol,
                },
                WhereConstraint::Equal { subject, rhs } => {
                    Goal::Equal(self.ety(*subject, bindings), self.ety(*rhs, bindings))
                },
            })
            .collect()
    }

    fn param_ety(&self, param: Entity, bindings: &HashMap<Entity, TyId>) -> ETy {
        match bindings.get(&param) {
            Some(&bound) => self.ety(bound, &HashMap::new()),
            None => ETy::Param(param),
        }
    }
}

impl EntailLayer for MonoLayer<'_> {
    fn supply(&self, ty: &ETy, protocol: Entity) -> Supply {
        // A param the implementing type never bound cannot be disproven.
        let Some(&concrete) = self.origin.borrow().get(ty) else {
            return Supply::Decided(Tri::Holds);
        };
        let sources = self
            .witnesses
            .iter()
            .filter(|w| w.protocol == protocol)
            .filter_map(|w| {
                let mut bindings = HashMap::new();
                match_pattern(self.arena, w.implementing_type, concrete, &mut bindings).then(|| {
                    Source::Extension {
                        target_args: Vec::new(),
                        clauses: self.goals(w, &bindings),
                    }
                })
            })
            .collect();
        Supply::Sources(sources)
    }

    fn assoc_binding(&self, base: &ETy, assoc: Entity) -> Option<ETy> {
        let &base_ty = self.origin.borrow().get(base)?;
        // The witness for the matching implementing type binds the alias; only
        // a ground binding is usable here (no arena to substitute into).
        let found: Vec<TyId> = self
            .witnesses
            .iter()
            .filter_map(|w| {
                let mut bindings = HashMap::new();
                if !match_pattern(self.arena, w.implementing_type, base_ty, &mut bindings) {
                    return None;
                }
                w.type_bindings
                    .iter()
                    .find(|(e, _)| *e == assoc)
                    .map(|&(_, t)| t)
            })
            .collect();
        match found.as_slice() {
            [one] => Some(self.ety(*one, &HashMap::new())),
            _ => None,
        }
    }

    /// Interned leaves are equal iff their handles are; a param is unbound.
    fn equal_abstract(&self, a: &ETy, b: &ETy) -> Tri {
        match (a, b) {
            (ETy::Param(_), _) | (_, ETy::Param(_)) => Tri::Unknown,
            (ETy::Proj { .. }, _) | (_, ETy::Proj { .. }) => Tri::Unknown,
            _ => Tri::Fails,
        }
    }
}

/// Do all of `witness`'s `where` constraints hold for the concrete self,
/// given the `bindings` from matching its implementing type? `Unknown` (an
/// unbound or unreducible position) permits — mono's conservative rule.
pub(crate) fn witness_constraints_hold(
    arena: &TyArena,
    witnesses: &[WitnessDef],
    witness: &WitnessDef,
    bindings: &HashMap<Entity, TyId>,
) -> bool {
    if witness.constraints.is_empty() {
        return true;
    }
    let layer = MonoLayer::new(arena, witnesses);
    let goals = layer.goals(witness, bindings);
    let verdict = Tri::all(goals.iter().map(|g| kestrel_entail::entails(&layer, g)));
    if verdict == Tri::Unknown {
        kestrel_debug::ktrace!("entail", "mono: witness clause undecidable, permitted");
    }
    verdict != Tri::Fails
}
