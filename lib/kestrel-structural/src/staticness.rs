//! Staticness: a type is **Static** iff no reference (`&T`) is reachable
//! through its stored fields, enum payloads, tuple elements or type args.
//!
//! DELIBERATE divergence from `kestrel_copy_fold::instance_semantics`: the
//! staticness fold is a 2-state AND whose gating positions are *structural*
//! (which type params appear in stored positions, computed by the front end,
//! not declared via `extend … where`) and are legal on ANY non-`not Static`
//! base. Do not "converge" this into the copy kernel: copy's invariant is
//! "gating positions non-empty only when the base is NotCopyable", and its
//! tri-state Cloneable arm has no Static analog.

use std::borrow::Cow;

use kestrel_hecs::Entity;

/// Per-nominal classification: unconditionally Static, unconditionally not,
/// or Static iff the type args at the given positions are Static.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Staticness {
    Static,
    NotStatic,
    /// Positions are indices into the nominal's type-param list, sorted,
    /// deduped. Unlike conditional copyability, a conditional base is the
    /// NORMAL state for any generic container that stores its param.
    ConditionalOn(Vec<usize>),
}

/// One type form's view of a type, as far as staticness cares. Each layer
/// only maps its own type onto this; what each shape *means* is decided
/// once, in [`ty_is_static`].
pub enum StaticShape<'t, T: Clone> {
    /// A reference: never Static (the axiom).
    Ref,
    /// A nominal instantiation: the per-instance rule.
    Nominal { entity: Entity, args: Cow<'t, [T]> },
    /// `Self` / an abstract associated type, judged by its base alone: a
    /// conditional base counts (Self inside its own definition is covered by
    /// the front end's `NominalStaticness` cycle guard).
    BaseOnly(Entity),
    /// A type parameter: Static unless its bound was relaxed.
    Param(Entity),
    /// Static iff every element is.
    Tuple(Cow<'t, [T]>),
    /// Exactly the type it stands for (a bounded alias's expansion).
    Alias(&'t T),
    /// A raw existential with these protocol bounds.
    RawExistential(Vec<Entity>),
    /// A closure: a view-kind closure holds views into its defining frame,
    /// so it is not Static, exactly like a ref; owning kinds snapshot their
    /// captures (references 2c, docs/design/closure-fields.md).
    Function { is_view: bool },
    /// Conservative never-block leaves (primitives, protocols, opaques,
    /// projections, never, errors, unresolved inference variables).
    Leaf,
}

/// Layer hooks for the staticness rule.
pub trait StaticLayer {
    type Ty: Clone;
    /// The nominal's classification.
    fn nominal_staticness(&self, entity: Entity) -> Staticness;
    /// This layer's type, seen as a [`StaticShape`].
    fn shape<'t>(&self, ty: &'t Self::Ty) -> StaticShape<'t, Self::Ty>;
    /// Does this type parameter carry the (implicit) Static bound? Layers
    /// differ in the lookup context and in dependency tracking.
    fn param_is_static(&self, param: Entity) -> bool;
    /// Is a raw existential with these bounds Static? It is iff one bound
    /// refines `Static`; layers differ in how they read refinement.
    fn raw_existential_is_static(&self, bounds: &[Entity]) -> bool;
    /// Is a component (type arg, tuple element, alias expansion) Static?
    /// Layers with recursion bookkeeping (the solver's depth) override this.
    fn component_is_static(&self, ty: &Self::Ty) -> bool
    where
        Self: Sized,
    {
        ty_is_static(self, ty)
    }
}

/// THE staticness rule over any layer's types: what each [`StaticShape`]
/// means is written here once, for every layer.
pub fn ty_is_static<L: StaticLayer>(layer: &L, ty: &L::Ty) -> bool {
    match layer.shape(ty) {
        StaticShape::Ref => false,
        StaticShape::Nominal { entity, args } => instance_is_static(layer, entity, &args),
        StaticShape::BaseOnly(entity) => {
            !matches!(layer.nominal_staticness(entity), Staticness::NotStatic)
        },
        StaticShape::Param(param) => layer.param_is_static(param),
        StaticShape::Tuple(elems) => elems.iter().all(|e| layer.component_is_static(e)),
        StaticShape::Alias(expanded) => layer.component_is_static(expanded),
        StaticShape::RawExistential(bounds) => layer.raw_existential_is_static(&bounds),
        StaticShape::Function { is_view } => !is_view,
        StaticShape::Leaf => true,
    }
}

/// THE per-instantiation staticness rule, single source of truth for every
/// layer: a Static base wins; a NotStatic base loses; a conditional base
/// ANDs the args at the gating positions. A missing arg at a gating
/// position is unprovable ⇒ false. Recursing ONLY into gating args is
/// load-bearing: `Pointer[T]` / `lang.ptr[T]` store no `T` (raw address
/// payload), so they classify Static for any `T` — the unsafe escape hatch.
pub fn instance_is_static<L: StaticLayer>(layer: &L, entity: Entity, args: &[L::Ty]) -> bool {
    match layer.nominal_staticness(entity) {
        Staticness::Static => true,
        Staticness::NotStatic => false,
        Staticness::ConditionalOn(positions) => positions.iter().all(|&i| {
            args.get(i)
                .is_some_and(|arg| layer.component_is_static(arg))
        }),
    }
}
