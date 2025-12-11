//! Member access behavior for symbols that can be accessed as members.
//!
//! This behavior is attached to symbols (like fields) that can be accessed
//! through the dot operator on a parent expression (e.g., `obj.field`).

use kestrel_span::Span;
use semantic_tree::behavior::Behavior;

use crate::{behavior::KestrelBehaviorKind, expr::Expression, language::KestrelLanguage, ty::Ty};

/// Behavior for symbols that can be accessed as members of a parent expression.
///
/// When you write `obj.field`, the field symbol has a `MemberAccessBehavior`
/// that knows how to produce the resulting expression given the parent `obj`.
#[derive(Debug, Clone)]
pub struct MemberAccessBehavior {
    /// The name of the member (for producing FieldAccess expressions)
    member_name: String,
    /// The type of the member when accessed
    member_type: Ty,
    /// Whether the member is mutable (var vs let for fields)
    member_mutable: bool,
}

impl Behavior<KestrelLanguage> for MemberAccessBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::MemberAccess
    }
}

impl MemberAccessBehavior {
    /// Create a new MemberAccessBehavior for a field
    pub fn new(member_name: String, member_type: Ty, member_mutable: bool) -> Self {
        MemberAccessBehavior {
            member_name,
            member_type,
            member_mutable,
        }
    }

    /// Get the member name
    pub fn member_name(&self) -> &str {
        &self.member_name
    }

    /// Get the member type
    pub fn member_type(&self) -> &Ty {
        &self.member_type
    }

    /// Check if the member is mutable
    pub fn is_mutable(&self) -> bool {
        self.member_mutable
    }

    /// Produce an expression for accessing this member on the given parent expression.
    ///
    /// For a field, this produces `Expression::field_access(parent, field_name, field_mutable, field_type, span)`.
    /// The resulting expression's mutability is: field_mutable AND parent.mutable
    pub fn access(&self, parent: Expression, span: Span) -> Expression {
        Expression::field_access(
            parent,
            self.member_name.clone(),
            self.member_mutable,
            self.member_type.clone(),
            span,
        )
    }
}

#[cfg(test)]
mod tests {
    use kestrel_span::Span;
    use super::*;
    use crate::ty::IntBits;

    #[test]
    fn test_member_access_mutable_field() {
        let field_ty = Ty::int(IntBits::I64, Span::from(0..3));
        let behavior = MemberAccessBehavior::new("x".to_string(), field_ty.clone(), true);

        assert_eq!(behavior.member_name(), "x");
        assert!(behavior.member_type().is_int());
        assert!(behavior.is_mutable());

        // Create a mutable parent expression (simulating a var binding)
        let parent = Expression::local_ref(
            crate::symbol::local::LocalId::new(0),
            Ty::int(IntBits::I64, Span::from(0..1)),
            true, // mutable
            Span::from(0..2),
        );
        let result = behavior.access(parent, Span::from(0..4));

        // Result should be a mutable field access
        assert!(result.is_mutable());
        match &result.kind {
            crate::expr::ExprKind::FieldAccess { field, .. } => {
                assert_eq!(field, "x");
            }
            _ => panic!("Expected FieldAccess"),
        }
    }

    #[test]
    fn test_member_access_immutable_field() {
        let field_ty = Ty::int(IntBits::I64, Span::from(0..3));
        let behavior = MemberAccessBehavior::new("x".to_string(), field_ty.clone(), false);

        assert!(!behavior.is_mutable());

        // Even with mutable parent, immutable field = immutable access
        let parent = Expression::local_ref(
            crate::symbol::local::LocalId::new(0),
            Ty::int(IntBits::I64, Span::from(0..1)),
            true, // mutable parent
            Span::from(0..2),
        );
        let result = behavior.access(parent, Span::from(0..4));

        // Result should be immutable (field is let)
        assert!(!result.is_mutable());
    }

    #[test]
    fn test_member_access_mutable_field_immutable_parent() {
        let field_ty = Ty::int(IntBits::I64, Span::from(0..3));
        let behavior = MemberAccessBehavior::new("x".to_string(), field_ty.clone(), true);

        assert!(behavior.is_mutable());

        // Immutable parent, mutable field = immutable access
        let parent = Expression::local_ref(
            crate::symbol::local::LocalId::new(0),
            Ty::int(IntBits::I64, Span::from(0..1)),
            false, // immutable parent
            Span::from(0..2),
        );
        let result = behavior.access(parent, Span::from(0..4));

        // Result should be immutable (parent is let)
        assert!(!result.is_mutable());
    }
}
