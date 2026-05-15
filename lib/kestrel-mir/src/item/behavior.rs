//! Type-level ownership behaviors.
//!
//! Every nominal type has two independent ownership properties, computed once
//! at MIR lowering time:
//!
//! - [`CopyBehavior`]: how the type is duplicated (or that it cannot be).
//! - [`DeinitBehavior`]: how the type is cleaned up at drop time.
//!
//! These are independent axes: `Rc[T]` is both `Clone` and has a non-trivial
//! deinit (decrement on drop). `FileHandle` is `None` (affine) and has a
//! non-trivial deinit. `Int` is `Bitwise` with trivial deinit.

use crate::id::FieldId;
use kestrel_hecs::Entity;

/// How a value of this type is duplicated.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CopyBehavior {
    /// Affine — cannot be duplicated. Only `Move` reads are legal.
    None,
    /// Trivially duplicable via a bitwise copy (memcpy / register move).
    Bitwise,
    /// Duplicated by calling the named clone method, which must have the
    /// signature `fn clone(self: &Self) -> Self`.
    Clone(Entity),
}

impl CopyBehavior {
    /// True if this type may be copied (either bitwise or via a clone method).
    pub fn is_copyable(&self) -> bool {
        !matches!(self, CopyBehavior::None)
    }
}

/// How a value of this type is destroyed at end of life.
///
/// A type's deinit is the optional user-provided `deinit { ... }` method
/// followed by structural drops of its fields. A type is "trivially
/// destructible" when neither component does any work.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct DeinitBehavior {
    /// User-provided deinit method, if any. Runs *before* structural field
    /// drops.
    pub user_method: Option<Entity>,
    /// Fields that have non-trivial deinit, in declaration order. Drops are
    /// emitted in this order after `user_method` runs.
    pub field_drops: Vec<FieldId>,
}

impl DeinitBehavior {
    /// True if dropping this type does no work — no user method, no field
    /// drops.
    pub fn is_trivial(&self) -> bool {
        self.user_method.is_none() && self.field_drops.is_empty()
    }
}
