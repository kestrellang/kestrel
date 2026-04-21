//! Witness-table lookup for associated-type resolution.
//!
//! Given a concrete self-type and a protocol, find the witness that proves
//! conformance and read a bound associated type (e.g. `ArrayIterator<Int64>:
//! Iterator` → `Item = Int64`). This lives in `kestrel-mir` rather than
//! codegen because witness tables are MIR data — the lookup has no target-
//! code-generation concerns.

use crate::ty::MirTy;
use crate::MirModule;
use kestrel_hecs::Entity;
use std::collections::HashMap;

impl MirModule {
    /// Resolve `<self_type as protocol>::name` via the witness table.
    ///
    /// Returns `None` if no witness matches or the witness has no binding for
    /// `name` — callers decide whether that's an error (monomorphization) or
    /// just "leave the projection unreduced" (layout during partial lowering).
    pub fn resolve_associated_type(
        &self,
        protocol: Entity,
        self_type: &MirTy,
        name: &str,
    ) -> Option<MirTy> {
        for witness in &self.witnesses {
            if witness.protocol != protocol {
                continue;
            }
            let mut bindings = HashMap::new();
            if !witness_match_pattern(&witness.implementing_type, self_type, &mut bindings) {
                continue;
            }
            let bound = witness.type_bindings.get(name)?;
            return Some(substitute_params_pure(bound, &bindings));
        }
        None
    }
}

/// Structural pattern match: a `TypeParam` on the pattern side is a wildcard
/// that binds to the concrete counterpart. Everything else must be structurally
/// equal. Mirrors the method-dispatch `match_pattern` in
/// `kestrel-codegen-cranelift/src/monomorphize/witness.rs`; that copy is
/// specialized for method-type-arg filtering and stays there, but the shared
/// shape-match logic lives here.
pub(crate) fn witness_match_pattern(
    pattern: &MirTy,
    concrete: &MirTy,
    bindings: &mut HashMap<Entity, MirTy>,
) -> bool {
    match (pattern, concrete) {
        (MirTy::TypeParam(entity), _) => match bindings.get(entity) {
            Some(existing) => existing == concrete,
            None => {
                bindings.insert(*entity, concrete.clone());
                true
            },
        },
        (
            MirTy::Named {
                entity: e1,
                type_args: a1,
            },
            MirTy::Named {
                entity: e2,
                type_args: a2,
            },
        ) => {
            e1 == e2
                && a1.len() == a2.len()
                && a1
                    .iter()
                    .zip(a2)
                    .all(|(p, c)| witness_match_pattern(p, c, bindings))
        },
        (MirTy::Ref(a), MirTy::Ref(b))
        | (MirTy::RefMut(a), MirTy::RefMut(b))
        | (MirTy::Pointer(a), MirTy::Pointer(b)) => witness_match_pattern(a, b, bindings),
        (MirTy::Tuple(a), MirTy::Tuple(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(p, c)| witness_match_pattern(p, c, bindings))
        },
        (
            MirTy::FuncThin {
                params: p1,
                ret: r1,
            },
            MirTy::FuncThin {
                params: p2,
                ret: r2,
            },
        )
        | (
            MirTy::FuncThick {
                params: p1,
                ret: r1,
            },
            MirTy::FuncThick {
                params: p2,
                ret: r2,
            },
        ) => {
            p1.len() == p2.len()
                && p1
                    .iter()
                    .zip(p2)
                    .all(|(p, c)| witness_match_pattern(p, c, bindings))
                && witness_match_pattern(r1, r2, bindings)
        },
        _ => pattern == concrete,
    }
}

/// Pure TypeParam substitution — no witness lookup, no SelfType handling.
/// Used inside the witness resolver to apply a witness's implementation-type-
/// param bindings to its associated-type result (e.g. `T → Int64` → the bound
/// type `Array[T]` becomes `Array[Int64]`). Kept minimal and local because the
/// full witness-aware substitution lives in `kestrel-codegen`.
fn substitute_params_pure(ty: &MirTy, subst: &HashMap<Entity, MirTy>) -> MirTy {
    if subst.is_empty() {
        return ty.clone();
    }
    let rec = |t: &MirTy| substitute_params_pure(t, subst);
    match ty {
        MirTy::TypeParam(entity) => match subst.get(entity) {
            Some(concrete) => concrete.clone(),
            None => ty.clone(),
        },
        MirTy::Pointer(inner) => MirTy::Pointer(Box::new(rec(inner))),
        MirTy::Ref(inner) => MirTy::Ref(Box::new(rec(inner))),
        MirTy::RefMut(inner) => MirTy::RefMut(Box::new(rec(inner))),
        MirTy::Tuple(elems) => MirTy::Tuple(elems.iter().map(&rec).collect()),
        MirTy::Named { entity, type_args } => MirTy::Named {
            entity: *entity,
            type_args: type_args.iter().map(&rec).collect(),
        },
        MirTy::AssociatedProjection {
            base,
            protocol,
            name,
        } => MirTy::AssociatedProjection {
            base: Box::new(rec(base)),
            protocol: *protocol,
            name: name.clone(),
        },
        MirTy::FuncThin { params, ret } => MirTy::FuncThin {
            params: params.iter().map(&rec).collect(),
            ret: Box::new(rec(ret)),
        },
        MirTy::FuncThick { params, ret } => MirTy::FuncThick {
            params: params.iter().map(&rec).collect(),
            ret: Box::new(rec(ret)),
        },
        MirTy::I8
        | MirTy::I16
        | MirTy::I32
        | MirTy::I64
        | MirTy::F16
        | MirTy::F32
        | MirTy::F64
        | MirTy::Bool
        | MirTy::Never
        | MirTy::Str
        | MirTy::SelfType
        | MirTy::Error => ty.clone(),
    }
}

