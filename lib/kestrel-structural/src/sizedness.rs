//! Sizedness: does a type have a size known at compile time? (existentials
//! design §2).
//!
//! Every type is **Sized** except the raw existential an `any P` box holds,
//! and a struct whose *last* stored field is an unsized type parameter (an
//! "unsized tail", like `RcBoxStorage[T]`) instantiated at an unsized type.

use std::borrow::Cow;

use kestrel_hecs::Entity;

/// Per-nominal classification. There is no unconditional "unsized"
/// nominal: a struct is unsized only through its tail type argument.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Sizedness {
    Sized,
    /// Sized iff the type args at these positions (indices into the
    /// nominal's type params, sorted, deduped) are Sized. Non-empty only for
    /// a struct whose last stored field is (transitively) one of its own
    /// `not Sized` params.
    ConditionalOn(Vec<usize>),
    /// Never Sized: `lang.slice[T]`, and a struct whose tail always is one.
    Unsized,
}

/// One type form's view of a type, as far as sizedness cares. Each layer
/// only maps its own type onto this; what each shape *means* is decided
/// once, in [`ty_is_sized`].
pub enum SizedShape<'t, T: Clone> {
    /// A raw existential: unsized, its size read from its table.
    RawExistential,
    /// `lang.slice[T]`: unsized, its size is count × the element's.
    Slice,
    /// A struct instantiation: the per-instance rule (its tail may be a
    /// relaxed param).
    Struct { entity: Entity, args: Cow<'t, [T]> },
    /// Exactly the type it stands for (a bounded alias's expansion).
    Alias(&'t T),
    /// A type parameter: Sized unless its bound was relaxed.
    Param(Entity),
    /// An associated-type projection: Sized unless declared `not Sized`.
    AssocProjection(Entity),
    /// Always Sized: enums, tuples, functions, refs (two words even to an
    /// unsized pointee), primitives, protocols, opaques, `Self`, and
    /// recovery leaves.
    Leaf,
}

/// Layer hooks for the sizedness rule.
pub trait SizedLayer {
    type Ty: Clone;
    fn nominal_sizedness(&self, entity: Entity) -> Sizedness;
    /// This layer's type, seen as a [`SizedShape`].
    fn shape<'t>(&self, ty: &'t Self::Ty) -> SizedShape<'t, Self::Ty>;
    /// Does this type parameter carry the (implicit) Sized bound? Layers
    /// differ in the context they judge it from.
    fn param_is_sized(&self, param: Entity) -> bool;
    /// Is this associated type Sized (not declared `type X: not Sized`)?
    fn assoc_is_sized(&self, assoc: Entity) -> bool;
    /// Is a component (type arg, alias expansion) Sized? Layers with
    /// recursion bookkeeping (the solver's depth) override this.
    fn component_is_sized(&self, ty: &Self::Ty) -> bool
    where
        Self: Sized,
    {
        ty_is_sized(self, ty)
    }
}

/// THE sizedness rule over any layer's types: what each [`SizedShape`]
/// means is written here once, for every layer.
pub fn ty_is_sized<L: SizedLayer>(layer: &L, ty: &L::Ty) -> bool {
    match layer.shape(ty) {
        SizedShape::RawExistential | SizedShape::Slice => false,
        SizedShape::Struct { entity, args } => instance_is_sized(layer, entity, &args),
        SizedShape::Alias(expanded) => layer.component_is_sized(expanded),
        SizedShape::Param(param) => layer.param_is_sized(param),
        SizedShape::AssocProjection(assoc) => layer.assoc_is_sized(assoc),
        SizedShape::Leaf => true,
    }
}

/// THE per-instantiation sizedness rule for every layer: a Sized base wins;
/// a conditional base ANDs the args at its tail positions. A missing arg at
/// a gating position is unprovable ⇒ false.
pub fn instance_is_sized<L: SizedLayer>(layer: &L, entity: Entity, args: &[L::Ty]) -> bool {
    match layer.nominal_sizedness(entity) {
        Sizedness::Sized => true,
        Sizedness::Unsized => false,
        Sizedness::ConditionalOn(positions) => positions
            .iter()
            .all(|&i| args.get(i).is_some_and(|arg| layer.component_is_sized(arg))),
    }
}
