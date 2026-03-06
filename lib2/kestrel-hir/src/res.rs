//! Name resolution results and local variable tracking.
//!
//! `Res` captures what a name resolves to — produced by name resolution,
//! consumed by HIR lowering. `Local` represents a local variable slot
//! allocated during HIR lowering.

use kestrel_ast::arena::Idx;
use kestrel_hecs::Entity;
use kestrel_span2::Span;

// ===== Local variables =====

/// Index into the locals table in `HirBody`.
pub type LocalId = Idx<Local>;

/// A local variable slot. Allocated during HIR lowering for every `let`/`var`
/// binding, function parameter, pattern binding, and synthetic variable
/// (from desugaring like for-loop iterators).
#[derive(Clone, Debug)]
pub struct Local {
    pub name: String,
    pub is_mut: bool,
    pub span: Span,
}

// ===== Resolution =====

/// What a name resolves to. Produced by name resolution,
/// consumed by HIR lowering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Res {
    /// Local variable
    Local(LocalId),
    /// ECS entity: function, struct, enum, enum case, field, protocol,
    /// type alias, type parameter, etc. NodeKind tells you what it is.
    Def(Entity),
    /// `self` value
    SelfValue,
    /// Unresolved (error)
    Err,
}
