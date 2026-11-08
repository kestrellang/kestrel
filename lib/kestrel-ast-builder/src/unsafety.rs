//! Which declarations need an unsafe context to use.
//!
//! The one source of truth for `unsafe`: the unsafe-use analyzer, the
//! witness rule and the LSP all read [`UnsafetyOf`]. Unsafety is part of a
//! declaration's signature, so editing a body never changes it.

use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::attrs::attrs_of;
use crate::components::{IsUnsafe, NodeKind};

/// Why using a declaration needs an unsafe context.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unsafety {
    /// `unsafe` written on the declaration, or an unsafe `lang` intrinsic.
    Declared,
    /// An `@extern` function: the compiler can't check foreign code.
    Extern,
}

/// Whether using `entity` (calling it, reading it, naming it as a value)
/// needs an unsafe context. Accessors answer for their property or
/// subscript. A protocol is never unsafe to *use*; an unsafe protocol
/// constrains its conformances instead ([`is_unsafe_protocol`]).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct UnsafetyOf {
    pub entity: Entity,
}

impl QueryFn for UnsafetyOf {
    type Output = Option<Unsafety>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Unsafety> {
        let mut entity = self.entity;
        match ctx.get::<NodeKind>(entity) {
            Some(NodeKind::Setter | NodeKind::RefAccessor) => entity = ctx.parent_of(entity)?,
            Some(NodeKind::Protocol) => return None,
            _ => {},
        }
        if ctx.get::<IsUnsafe>(entity).is_some() {
            return Some(Unsafety::Declared);
        }
        attrs_of(ctx, entity)
            .is_extern()
            .then_some(Unsafety::Extern)
    }
}

/// `unsafe protocol P`: every conformance must be written `unsafe P`.
pub fn is_unsafe_protocol(ctx: &QueryContext<'_>, protocol: Entity) -> bool {
    ctx.get::<NodeKind>(protocol) == Some(&NodeKind::Protocol)
        && ctx.get::<IsUnsafe>(protocol).is_some()
}
