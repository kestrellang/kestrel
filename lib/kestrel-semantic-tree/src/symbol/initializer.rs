use std::sync::{Arc, RwLock};

use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    behavior::callable::{CallableBehavior, CallableSignature, SignatureType},
    behavior::visibility::VisibilityBehavior,
    language::KestrelLanguage,
    symbol::kind::KestrelSymbolKind,
    symbol::local::{Local, LocalId},
    ty::Ty,
};

// Re-export CallableParameter as Parameter for convenience
pub use crate::behavior::callable::CallableParameter as Parameter;

/// Represents an initializer declaration in the semantic tree.
///
/// Initializers are special callable entities that construct instances of structs.
/// They differ from functions in several key ways:
/// - They don't have a name (always called through the struct type)
/// - They have a special `self` receiver with "initializing" semantics
/// - Field assignments must happen before `self` can be used
/// - They implicitly return `Self`
///
/// # Callable/Overloading System
///
/// Initializers use `CallableBehavior` for overload resolution, similar to functions.
/// Multiple initializers can coexist if they have different parameter signatures.
///
/// # Implicit vs Explicit Initializers
///
/// - **Implicit**: Compiler-generated memberwise initializer (no symbol exists)
/// - **Explicit**: User-defined `init` declarations (represented by this symbol)
///
/// When any explicit initializer exists, the implicit one is suppressed.
#[derive(Debug)]
pub struct InitializerSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
    /// Local variables within this initializer (populated during body resolution)
    /// This includes initializer parameters and any let/var declarations.
    locals: RwLock<Vec<Local>>,
}

impl Symbol<KestrelLanguage> for InitializerSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl InitializerSymbol {
    /// Create a new InitializerSymbol
    ///
    /// NOTE: CallableBehavior is NOT added here. It will be added during the bind phase
    /// when types are resolved.
    pub fn new(
        span: Span,
        declaration_span: Span,
        visibility: VisibilityBehavior,
        parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    ) -> Self {
        // Initializers use "init" as their name for display purposes
        let name = kestrel_span::Name::new("init".to_string(), declaration_span.clone());

        let mut builder = SymbolMetadataBuilder::new(KestrelSymbolKind::Initializer)
            .with_name(name)
            .with_declaration_span(declaration_span)
            .with_span(span)
            .with_behavior(Arc::new(visibility));

        if let Some(p) = parent {
            builder = builder.with_parent(Arc::downgrade(&p));
        }

        InitializerSymbol {
            metadata: builder.build(),
            locals: RwLock::new(Vec::new()),
        }
    }

    /// Get the callable behavior from metadata.
    ///
    /// Returns `None` if bind phase hasn't occurred yet (CallableBehavior is added during bind).
    fn get_callable(&self) -> Option<CallableBehavior> {
        self.metadata
            .get_behavior::<CallableBehavior>()
            .map(|b| (*b).clone())
    }

    /// Get the callable behavior (cloned)
    pub fn callable(&self) -> Option<CallableBehavior> {
        self.get_callable()
    }

    /// Get the initializer's parameters
    pub fn parameters(&self) -> Vec<Parameter> {
        self.get_callable()
            .map(|c| c.parameters().to_vec())
            .unwrap_or_default()
    }

    /// Get the number of parameters (arity)
    pub fn arity(&self) -> usize {
        self.get_callable().map(|c| c.arity()).unwrap_or(0)
    }

    /// Get the callable signature for overload resolution and duplicate detection.
    ///
    /// Two initializers with the same signature are considered duplicates.
    /// Note: name is always "init" for initializers.
    pub fn signature(&self) -> CallableSignature {
        self.get_callable()
            .map(|c| c.signature("init"))
            .unwrap_or_else(|| {
                CallableSignature::new("init".to_string(), vec![], vec![], SignatureType::Unit)
            })
    }

    /// Get parameter labels for display/debugging
    ///
    /// Returns a list of external labels (or None for unlabeled parameters).
    pub fn parameter_labels(&self) -> Vec<Option<String>> {
        self.get_callable()
            .map(|c| {
                c.parameters()
                    .iter()
                    .map(|p| p.external_label().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Add a new local variable to this initializer.
    /// Returns the LocalId assigned to the new local.
    pub fn add_local(&self, name: String, ty: Ty, mutable: bool, span: Span) -> LocalId {
        let mut locals = self.locals.write().unwrap();
        let id = LocalId::new(locals.len());
        locals.push(Local::new(id, name, ty, mutable, span));
        id
    }

    /// Get a local by its ID
    pub fn get_local(&self, id: LocalId) -> Option<Local> {
        let locals = self.locals.read().unwrap();
        locals.get(id.index()).cloned()
    }

    /// Get all locals in this initializer
    pub fn locals(&self) -> Vec<Local> {
        self.locals.read().unwrap().clone()
    }

    /// Get the number of locals
    pub fn local_count(&self) -> usize {
        self.locals.read().unwrap().len()
    }
}
