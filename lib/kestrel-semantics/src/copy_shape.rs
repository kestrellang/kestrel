//! The front-end copy-semantics rule, written once over a shared type shape.
//!
//! `kestrel-copy-fold` owns the decision tree for a nominal instantiation
//! (`instance_semantics`) and the aggregate fold. What each *kind* of type
//! means (a param, `Self`, an opaque, a closure, a raw existential) used to
//! be written out again in every front-end layer — HIR here, the solver's
//! `TyVar`, analyze's `ResolvedTy`. Each of those now maps its own type onto
//! a [`CopyShape`], and [`ty_copy_semantics`] decides what the shape means,
//! once. The MIR layers keep their own classifiers: their native value is
//! `CopyBehavior`, whose `Clone` payloads this rule cannot produce.

use std::borrow::Cow;

use kestrel_ast::FnTypeKind;
use kestrel_copy_fold::{CopyLayer, CopySemantics, fold_members, instance_semantics};
use kestrel_hecs::{Entity, QueryContext};

use crate::{TypeParamCopyRequirement, raw_existential_copy_semantics, self_type_copy_semantics};

/// One front-end type form's view of a type, as far as copy semantics cares.
pub enum CopyShape<'t, T: Clone> {
    /// A struct / enum instantiation: the shared per-instance rule.
    Nominal { entity: Entity, args: Cow<'t, [T]> },
    /// `Self`: a protocol's abstract `Self` is as copyable as its
    /// refinements guarantee.
    SelfType(Entity),
    /// A type parameter: what its bound requires.
    Param(Entity),
    /// Folds its elements.
    Tuple(Cow<'t, [T]>),
    /// Exactly the type it stands for (a bounded alias's expansion).
    Alias(&'t T),
    /// An abstract associated type, judged by its declaration alone.
    BaseOnly(Entity),
    /// A raw existential with these protocol bounds.
    RawExistential(Vec<Entity>),
    /// `some P` (`not_copyable`: `some P and not Copyable`, which may hide a
    /// move-only underlier; the plain form guarantees a duplicable one).
    Opaque { not_copyable: bool },
    /// A closure value: its class is fixed by its KIND, not by its captures
    /// (plan D6 / docs/design/closures.md §"Copy and Drop").
    Function(FnTypeKind),
    /// A fixed answer, with a comment at the layer's arm saying why.
    Leaf(CopySemantics),
}

/// Front-end layer hooks for [`ty_copy_semantics`], on top of the
/// `CopyLayer` hooks the per-instance kernel uses. A layer's
/// `CopyLayer::member_semantics` is its recursion point (the solver threads
/// its depth there), and every component of a shape goes through it.
pub trait CopyShapeLayer: CopyLayer<Sem = CopySemantics>
where
    Self::Ty: Clone,
{
    fn query(&self) -> &QueryContext<'_>;
    fn root(&self) -> Entity;
    /// This layer's type, seen as a [`CopyShape`].
    fn copy_shape<'t>(&self, ty: &'t Self::Ty) -> CopyShape<'t, Self::Ty>;
    /// The context a type param's bound is looked up from.
    fn param_context(&self, param: Entity) -> Entity;
    /// The context a protocol's abstract `Self` is judged from (in-scope
    /// `where Self:` bounds).
    fn self_context(&self) -> Entity;
}

/// THE front-end copy-semantics rule: what each [`CopyShape`] means,
/// written once for every front-end layer.
pub fn ty_copy_semantics<L: CopyShapeLayer>(layer: &L, ty: &L::Ty) -> CopySemantics
where
    L::Ty: Clone,
{
    match layer.copy_shape(ty) {
        CopyShape::Nominal { entity, args } => instance_semantics(layer, entity, &args),
        CopyShape::SelfType(entity) => {
            self_type_copy_semantics(layer.query(), entity, layer.self_context(), layer.root())
        },
        CopyShape::Param(param) => layer
            .query()
            .query(TypeParamCopyRequirement {
                param,
                context: layer.param_context(param),
                root: layer.root(),
            })
            .into(),
        CopyShape::Tuple(elems) => fold_members(elems.iter().map(|e| layer.member_semantics(e))),
        CopyShape::Alias(expanded) => layer.member_semantics(expanded),
        CopyShape::BaseOnly(entity) => layer.base_semantics(entity),
        CopyShape::RawExistential(bounds) => {
            raw_existential_copy_semantics(layer.query(), bounds, layer.root())
        },
        CopyShape::Opaque { not_copyable } => {
            if not_copyable {
                CopySemantics::NotCopyable
            } else {
                CopySemantics::Copyable
            }
        },
        CopyShape::Function(kind) => kestrel_copy_fold::fn_kind_semantics(kind),
        CopyShape::Leaf(sem) => sem,
    }
}
