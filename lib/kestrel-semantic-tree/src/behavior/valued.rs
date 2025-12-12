use kestrel_span::Span;
use semantic_tree::behavior::Behavior;

use crate::{behavior::KestrelBehaviorKind, language::KestrelLanguage, ty::Ty};

/// ValueBehavior indicates that a symbol can be evaluated to a value.
///
/// This behavior is used for:
/// - Fields (static and instance)
/// - Functions (as first-class values)
/// - Global variables (module-level let/var)
///
/// A symbol with ValueBehavior can appear in value position in expressions,
/// e.g., as the target of a path expression like `module.myFunction`.
#[derive(Debug, Clone)]
pub struct ValueBehavior {
    /// The type of the value this symbol evaluates to
    ty: Ty,
    /// The span where this value is defined
    span: Span,
}

impl Behavior<KestrelLanguage> for ValueBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::Valued
    }
}

impl ValueBehavior {
    /// Create a new ValueBehavior with the given type and span
    pub fn new(ty: Ty, span: Span) -> Self {
        ValueBehavior { ty, span }
    }

    /// Get the type of the value
    pub fn ty(&self) -> &Ty {
        &self.ty
    }

    /// Get the span where this value is defined
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Get a mutable reference to the type
    /// This is useful during semantic analysis when resolving types
    pub fn ty_mut(&mut self) -> &mut Ty {
        &mut self.ty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_span::Span;

    #[test]
    fn test_value_behavior_simple() {
        use crate::ty::IntBits;
        let ty = Ty::int(IntBits::I64, Span::from(5..8));
        let behavior = ValueBehavior::new(ty, Span::from(0..10));

        assert!(behavior.ty().is_int());
        assert_eq!(behavior.span().range(), 0..10);
    }

    #[test]
    fn test_value_behavior_function_type() {
        use crate::ty::IntBits;
        let param = Ty::int(IntBits::I64, Span::from(1..4));
        let return_ty = Ty::int(IntBits::I64, Span::from(9..12));
        let fn_ty = Ty::function(vec![param], return_ty, Span::from(0..12));

        let behavior = ValueBehavior::new(fn_ty, Span::from(0..20));

        assert!(behavior.ty().is_function());
    }
}
