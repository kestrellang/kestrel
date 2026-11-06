//! The structural builtin-protocol rules — `Static` and `Sized` — written
//! once, for every layer that judges them.
//!
//! `Static` and `Sized` are computed from a type's structure, never declared,
//! so no witness or conformance table ever supplies them. Every layer that
//! asks (HIR, the solver's `TyVar`, analyze's `ResolvedTy`, mono's `TyId`)
//! maps its own type onto a small shape enum and calls the rule here. What
//! each shape *means* lives only here.
//!
//! Dependency-graph leaf (only `kestrel-hecs` for `Entity`), so mono can use
//! it without pulling in the front end — the `kestrel-copy-fold` precedent.

mod sizedness;
mod staticness;

pub use sizedness::{SizedLayer, SizedShape, Sizedness, instance_is_sized, ty_is_sized};
pub use staticness::{StaticLayer, StaticShape, Staticness, instance_is_static, ty_is_static};
