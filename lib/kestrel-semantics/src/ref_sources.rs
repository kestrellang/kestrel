//! Which arguments a call's ref-carrying result can borrow from.
//!
//! A callee can only hand back a borrow of something its arguments hold, so
//! an argument whose type cannot contain any type the result points at is
//! never a source: in `xs.slice(from: 0, count: 2)`, an `Int64` can't hold
//! an `Array`, so the result borrows `xs` alone.
//!
//! Decided on the call's INSTANCE types (a generic subscript's `I.SeqOutput`
//! is only known there). One algorithm, run over each layer's own type
//! representation through [`RefSourceLayer`]: HIR types for the analyzers
//! (E509) and MIR types for lowering (E494 roots), so the two checks agree.

use std::collections::HashSet;

use kestrel_ast_builder::TypeParams;
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::HirTy;

use crate::collect_child_types;

/// How [`ref_source_mask`] sees one type.
pub enum RefShape<T> {
    /// A reference to `T`.
    Ref(T),
    /// A nominal instance. `fields` are its stored fields / payloads with the
    /// type args substituted (`None` when its storage isn't visible, e.g. a
    /// builtin); `args` are the type args.
    Nominal {
        entity: Entity,
        fields: Option<Vec<T>>,
        args: Vec<T>,
    },
    Tuple(Vec<T>),
    /// A raw pointer to `T`: may reach a `T`, but carries no reference.
    Pointer(T),
    /// Generic, opaque, existential or closure: may hold or carry anything.
    Opaque,
    /// A primitive or other type holding nothing else.
    Leaf,
}

/// One compiler layer's view of its types, for [`ref_source_mask`].
pub trait RefSourceLayer {
    type Ty: Clone;
    fn shape(&mut self, ty: &Self::Ty) -> RefShape<Self::Ty>;
    /// Structural equality (used for leaves).
    fn same(&self, a: &Self::Ty, b: &Self::Ty) -> bool;
}

/// Per argument: may a call returning `result` hand back a borrow of it?
/// Over-approximates (every `true` when the result's pointees are unknown).
pub fn ref_source_mask<L: RefSourceLayer>(layer: &mut L, result: &L::Ty, args: &[L::Ty]) -> Vec<bool> {
    let mut pointees = Vec::new();
    if !ref_pointees(layer, result, &mut pointees, &mut HashSet::new()) {
        return vec![true; args.len()];
    }
    args.iter()
        .map(|a| {
            pointees
                .iter()
                .any(|p| may_hold(layer, a, p, &mut HashSet::new()))
        })
        .collect()
}

/// Collects the pointee of every reference a `ty` value carries. `false` =
/// unknowable (a closure's captures, an opaque or generic pointee).
fn ref_pointees<L: RefSourceLayer>(
    layer: &mut L,
    ty: &L::Ty,
    out: &mut Vec<L::Ty>,
    expanding: &mut HashSet<Entity>,
) -> bool {
    match layer.shape(ty) {
        RefShape::Ref(inner) => {
            if matches!(layer.shape(&inner), RefShape::Opaque) {
                return false;
            }
            out.push(inner.clone());
            // `&View` also reaches whatever `View` borrows.
            ref_pointees(layer, &inner, out, expanding)
        },
        RefShape::Nominal { entity, fields, args } => {
            if !expanding.insert(entity) {
                return true; // a cycle adds no new reference on its own
            }
            // Args matter only through the fields they're substituted into.
            let members = fields.unwrap_or(args);
            let ok = members.iter().all(|m| ref_pointees(layer, m, out, expanding));
            expanding.remove(&entity);
            ok
        },
        RefShape::Tuple(elems) => elems.iter().all(|e| ref_pointees(layer, e, out, expanding)),
        RefShape::Opaque => false,
        RefShape::Pointer(_) | RefShape::Leaf => true,
    }
}

/// Whether a value of `ty` may hold (inline, behind a pointer, or through a
/// stored reference) a value of type `target`. Nominals match by head
/// (`Array[Int64]` may be `Array[T]`).
fn may_hold<L: RefSourceLayer>(
    layer: &mut L,
    ty: &L::Ty,
    target: &L::Ty,
    expanding: &mut HashSet<Entity>,
) -> bool {
    let target_head = match layer.shape(target) {
        RefShape::Nominal { entity, .. } => Some(entity),
        RefShape::Opaque => return true,
        _ => None,
    };
    match layer.shape(ty) {
        RefShape::Nominal { entity, fields, args } => {
            if target_head == Some(entity) {
                return true;
            }
            if !expanding.insert(entity) {
                return false;
            }
            let found = fields
                .into_iter()
                .flatten()
                .chain(args)
                .any(|m| may_hold(layer, &m, target, expanding));
            expanding.remove(&entity);
            found
        },
        RefShape::Ref(inner) | RefShape::Pointer(inner) => may_hold(layer, &inner, target, expanding),
        RefShape::Tuple(elems) => {
            layer.same(ty, target) || elems.iter().any(|e| may_hold(layer, e, target, expanding))
        },
        RefShape::Opaque => true,
        RefShape::Leaf => layer.same(ty, target),
    }
}

/// [`RefSourceLayer`] over `HirTy` (the analyzers' instance types).
pub struct HirRefSources<'a, 'q> {
    pub ctx: &'a QueryContext<'q>,
    pub root: Entity,
}

impl RefSourceLayer for HirRefSources<'_, '_> {
    type Ty = HirTy;

    fn shape(&mut self, ty: &HirTy) -> RefShape<HirTy> {
        match ty {
            HirTy::Ref { inner, .. } => RefShape::Ref((**inner).clone()),
            HirTy::Struct { entity, args, .. } | HirTy::Enum { entity, args, .. } => {
                let params = self
                    .ctx
                    .get::<TypeParams>(*entity)
                    .map(|tp| tp.0.clone())
                    .unwrap_or_default();
                let subs: Vec<(Entity, HirTy)> = params.into_iter().zip(args.iter().cloned()).collect();
                let fields: Vec<HirTy> = collect_child_types(self.ctx, *entity, self.root)
                    .into_iter()
                    .map(|(_, t)| t.substitute_params(&subs))
                    .collect();
                RefShape::Nominal {
                    entity: *entity,
                    fields: (!fields.is_empty()).then_some(fields),
                    args: args.clone(),
                }
            },
            HirTy::Tuple(elems, _) => RefShape::Tuple(elems.clone()),
            HirTy::Alias { expanded, .. } => self.shape(expanded),
            HirTy::Never(_) => RefShape::Leaf,
            _ => RefShape::Opaque,
        }
    }

    fn same(&self, a: &HirTy, b: &HirTy) -> bool {
        a.same_type(b)
    }
}
