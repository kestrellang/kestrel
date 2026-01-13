use std::sync::Arc;

use kestrel_span::{Name, Span};
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};

use crate::{language::KestrelLanguage, symbol::kind::KestrelSymbolKind, ty::Ty};

/// Variance describes how a type parameter behaves with respect to subtyping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Variance {
    /// Covariant: If A <: B, then F[A] <: F[B]
    /// Type parameter appears only in output positions (e.g., return types)
    Covariant,

    /// Contravariant: If A <: B, then F[B] <: F[A]
    /// Type parameter appears only in input positions (e.g., function parameters)
    Contravariant,

    /// Invariant: F[A] and F[B] have no subtyping relationship
    /// Type parameter appears in both input and output positions
    #[default]
    Invariant,

    /// Bivariant: F[A] <: F[B] for any A, B (rare, usually indicates unused parameter)
    Bivariant,
}

/// Represents a type parameter symbol in a generic declaration.
///
/// Type parameters are declared in type parameter lists, e.g., `struct Box[T]` or
/// `func identity[T](value: T) -> T`.
#[derive(Debug)]
pub struct TypeParameterSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
    /// Default type for this parameter, if specified
    default: Option<Ty>,
    /// Variance of this type parameter (computed during struct building)
    variance: Variance,
}

impl Symbol<KestrelLanguage> for TypeParameterSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl TypeParameterSymbol {
    /// Create a new TypeParameterSymbol with a name, span, and optional parent
    pub fn new(name: Name, span: Span, parent: Option<Arc<dyn Symbol<KestrelLanguage>>>) -> Self {
        let mut builder = SymbolMetadataBuilder::new(KestrelSymbolKind::TypeParameter)
            .with_name(name.clone())
            .with_declaration_span(name.span.clone())
            .with_span(span);

        if let Some(p) = parent {
            builder = builder.with_parent(Arc::downgrade(&p));
        }

        TypeParameterSymbol {
            metadata: builder.build(),
            default: None,
            variance: Variance::default(),
        }
    }

    /// Create a new TypeParameterSymbol with a default type
    pub fn with_default(
        name: Name,
        span: Span,
        default: Ty,
        parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    ) -> Self {
        let mut symbol = Self::new(name, span, parent);
        symbol.default = Some(default);
        symbol
    }

    /// Get the default type for this type parameter, if any
    pub fn default(&self) -> Option<&Ty> {
        self.default.as_ref()
    }

    /// Check if this type parameter has a default
    pub fn has_default(&self) -> bool {
        self.default.is_some()
    }

    /// Get the variance of this type parameter.
    pub fn variance(&self) -> Variance {
        self.variance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_span::Span;
    use kestrel_span::Spanned;

    #[test]
    fn test_type_parameter_basic() {
        let name = Spanned::new("T".to_string(), Span::new(0, 0..1));
        let param = TypeParameterSymbol::new(name, Span::new(0, 0..1), None);

        assert_eq!(param.metadata().name().value, "T");
        assert!(!param.has_default());
        assert_eq!(param.variance(), Variance::Invariant);
    }

    #[test]
    fn test_type_parameter_with_default() {
        use crate::ty::IntBits;
        let name = Spanned::new("T".to_string(), Span::new(0, 0..1));
        let default_ty = Ty::int(IntBits::I64, Span::new(0, 4..7));
        let param = TypeParameterSymbol::with_default(name, Span::new(0, 0..7), default_ty, None);

        assert!(param.has_default());
        assert!(param.default().unwrap().is_int());
    }

    #[test]
    fn test_variance_default() {
        assert_eq!(Variance::default(), Variance::Invariant);
    }
}
