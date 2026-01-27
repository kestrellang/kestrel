//! Computed member access behavior for computed properties.
//!
//! This behavior is attached to computed property symbols that can be accessed
//! through the dot operator on a parent expression (e.g., `obj.computedProp`).
//! Unlike MemberAccessBehavior (for stored fields), this tracks the getter
//! and optional setter symbols.

use kestrel_span::Span;
use semantic_tree::behavior::Behavior;
use semantic_tree::symbol::SymbolId;

use crate::{behavior::KestrelBehaviorKind, expr::Expression, language::KestrelLanguage, ty::Ty};

/// Behavior for computed property member access.
/// Unlike MemberAccessBehavior (for stored fields), this tracks
/// the getter and optional setter symbols.
#[derive(Debug, Clone)]
pub struct ComputedMemberAccessBehavior {
    member_name: String,
    member_type: Ty,
    getter: SymbolId,
    setter: Option<SymbolId>,
}

impl Behavior<KestrelLanguage> for ComputedMemberAccessBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::ComputedMemberAccess
    }
}

impl ComputedMemberAccessBehavior {
    pub fn new(
        member_name: String,
        member_type: Ty,
        getter: SymbolId,
        setter: Option<SymbolId>,
    ) -> Self {
        Self {
            member_name,
            member_type,
            getter,
            setter,
        }
    }

    pub fn member_name(&self) -> &str {
        &self.member_name
    }

    pub fn member_type(&self) -> &Ty {
        &self.member_type
    }

    pub fn getter(&self) -> SymbolId {
        self.getter
    }

    pub fn setter(&self) -> Option<SymbolId> {
        self.setter
    }

    pub fn has_setter(&self) -> bool {
        self.setter.is_some()
    }

    /// Create a field access expression for this computed property.
    /// The actual getter call will be generated during lowering.
    pub fn access(&self, parent: Expression, span: Span) -> Expression {
        // For computed properties, we still create a FieldAccess expression.
        // The lowering phase will detect it's computed and generate a getter call.
        // Computed properties with setters are assignable (mutable)
        Expression::field_access(
            parent,
            self.member_name.clone(),
            self.has_setter(), // assignable if it has a setter
            self.member_type.clone(),
            span,
        )
    }
}
