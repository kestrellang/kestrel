use std::sync::Arc;

use kestrel_span::{Span, Spanned};
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    behavior_ext::BehaviorExt,
    language::KestrelLanguage,
    symbol::kind::KestrelSymbolKind,
    symbol::type_parameter::TypeParameterSymbol,
    ty::{Ty, WhereClause},
};

/// Represents an extension declaration in the semantic tree.
///
/// Extensions add methods and protocol conformances to existing types.
/// Unlike structs, extensions don't have a name - they are identified by their target type.
///
/// # Type Parameters
///
/// Extensions reference type parameters from their target type rather than declaring new ones.
/// For example, `extend Box[T, Int]` references Box's T, with Int as a concrete specialization.
///
/// # Resolution Phases
///
/// - BUILD: Creates the extension symbol with syntax span
/// - BIND: Adds ExtensionTargetBehavior with resolved target type and where clause
#[derive(Debug)]
pub struct ExtensionSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl Symbol<KestrelLanguage> for ExtensionSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl ExtensionSymbol {
    /// Create a new ExtensionSymbol with a span and parent
    ///
    /// Extensions use a synthetic name "(extension)" since they don't have
    /// user-defined names like structs or functions.
    pub fn new(
        span: Span,
        parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    ) -> Self {
        // Use a synthetic name for extensions since they don't have user-defined names
        let synthetic_name = Spanned::new("(extension)".to_string(), span.clone());

        let mut builder = SymbolMetadataBuilder::new(KestrelSymbolKind::Extension)
            .with_span(span.clone())
            .with_declaration_span(span)
            .with_name(synthetic_name);

        if let Some(p) = parent {
            builder = builder.with_parent(Arc::downgrade(&p));
        }

        ExtensionSymbol {
            metadata: builder.build(),
        }
    }

    /// Get the target type this extension extends.
    ///
    /// Available after BIND phase when ExtensionTargetBehavior is attached.
    pub fn target_type(&self) -> Option<Ty> {
        self.metadata
            .extension_target_behavior()
            .map(|b| b.target_type().clone())
    }

    /// Get the type arguments used to specialize the target type.
    ///
    /// For `extend Box[T, Int]`, returns [TypeParameter(T), Concrete(Int)]
    pub fn type_arguments(&self) -> Vec<Ty> {
        self.metadata
            .extension_target_behavior()
            .map(|b| b.type_arguments().to_vec())
            .unwrap_or_default()
    }

    /// Get the type parameters referenced by this extension.
    ///
    /// These are not declared by the extension but are references to the target struct's
    /// type parameters. Used for scope resolution within extension methods.
    pub fn referenced_type_parameters(&self) -> Vec<Arc<TypeParameterSymbol>> {
        self.metadata
            .extension_target_behavior()
            .map(|b| b.referenced_type_parameters().to_vec())
            .unwrap_or_default()
    }

    /// Get the where clause for this extension.
    ///
    /// Extensions inherit constraints from their target type and can add additional ones.
    pub fn where_clause(&self) -> WhereClause {
        self.metadata
            .extension_target_behavior()
            .map(|b| b.where_clause().clone())
            .unwrap_or_else(WhereClause::new)
    }

    /// Calculate the specificity of this extension.
    ///
    /// More concrete type arguments = higher specificity.
    /// Used to resolve conflicts when multiple extensions could apply.
    pub fn specificity(&self) -> usize {
        self.type_arguments()
            .iter()
            .filter(|ty| !ty.is_type_parameter())
            .count()
    }

    /// Check if this extension is generic (has any type parameter references).
    pub fn is_generic(&self) -> bool {
        self.type_arguments()
            .iter()
            .any(|ty| ty.is_type_parameter())
    }

    /// Check if this extension is fully specialized (no type parameters).
    pub fn is_specialized(&self) -> bool {
        !self.is_generic()
    }
}
