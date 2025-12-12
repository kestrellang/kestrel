use std::sync::{Arc, RwLock};

use kestrel_span::{Name, Span};
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    behavior::callable::{CallableBehavior, CallableSignature, SignatureType},
    behavior::function_data::FunctionDataBehavior,
    behavior::generics::GenericsBehavior,
    behavior::visibility::VisibilityBehavior,
    language::KestrelLanguage,
    symbol::kind::KestrelSymbolKind,
    symbol::local::{Local, LocalId},
    symbol::type_parameter::TypeParameterSymbol,
    ty::{Ty, WhereClause},
};

// Re-export CallableParameter as Parameter for backwards compatibility
pub use crate::behavior::callable::CallableParameter as Parameter;

/// Represents a function declaration in the semantic tree.
///
/// Functions are callable entities with parameters, return types, and a body.
/// They can be:
/// - Standalone functions (at module level)
/// - Methods (within structs/classes) - indicated by having a parent
/// - Static functions (don't receive `self`)
///
/// # Callable/Overloading System
///
/// Functions use `CallableBehavior` for overload resolution:
/// - Labels enable overloading by external parameter names
/// - Parameter types enable type-based overloading
/// - Parameter count enables overloading by arity
///
/// Two functions with the same `CallableSignature` are duplicates (error).
/// Functions with different signatures can coexist as overloads.
///
/// # Type Resolution
///
/// During build phase, basic symbol information is captured but `CallableBehavior` is not yet added.
/// During bind phase, `CallableBehavior` and `GenericsBehavior` are added with resolved types.
/// Query methods like `return_type()` and `signature()` return `None`/defaults until bind occurs.
#[derive(Debug)]
pub struct FunctionSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
    is_static: bool,
    has_body: bool,
    /// Local variables within this function (populated during body resolution)
    /// This includes function parameters and any let/var declarations.
    /// Variables with the same name due to shadowing have different LocalIds.
    locals: RwLock<Vec<Local>>,
}

impl Symbol<KestrelLanguage> for FunctionSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl FunctionSymbol {
    /// Create a new FunctionSymbol
    pub fn new(
        name: Name,
        span: Span,
        visibility: VisibilityBehavior,
        is_static: bool,
        has_body: bool,
        parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    ) -> Self {
        // Create the function data behavior
        let function_data = FunctionDataBehavior::new(has_body, is_static);

        // Note: CallableBehavior and GenericsBehavior are added during bind phase
        let mut builder = SymbolMetadataBuilder::new(KestrelSymbolKind::Function)
            .with_name(name.clone())
            .with_declaration_span(name.span.clone())
            .with_span(span)
            .with_behavior(Arc::new(visibility))
            .with_behavior(Arc::new(function_data));

        if let Some(p) = parent {
            builder = builder.with_parent(Arc::downgrade(&p));
        }

        FunctionSymbol {
            metadata: builder.build(),
            is_static,
            has_body,
            locals: RwLock::new(Vec::new()),
        }
    }

    /// Check if this function is static
    pub fn is_static(&self) -> bool {
        self.is_static
    }

    /// Check if this function has a body
    pub fn has_body(&self) -> bool {
        self.has_body
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

    /// Get the function's return type
    ///
    /// Returns the resolved return type if bind has occurred.
    pub fn return_type(&self) -> Ty {
        self.get_callable()
            .map(|c| c.return_type().clone())
            .unwrap_or_else(|| Ty::error(Span::from(0..0)))
    }

    /// Get the function's parameters
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
    /// Two functions with the same signature are considered duplicates.
    pub fn signature(&self) -> CallableSignature {
        self.get_callable()
            .map(|c| c.signature(&self.metadata.name().value))
            .unwrap_or_else(|| {
                CallableSignature::new(
                    self.metadata.name().value.clone(),
                    vec![],
                    vec![],
                    SignatureType::Unit,
                )
            })
    }

    /// Get the function signature as a Ty::Function
    ///
    /// This is useful for type checking and storing the function's type.
    pub fn function_type(&self) -> Ty {
        self.get_callable()
            .map(|c| c.function_type())
            .unwrap_or_else(|| Ty::error(Span::from(0..0)))
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

    /// Get the type parameters for this function.
    ///
    /// During BUILD phase (before GenericsBehavior is attached), this gets
    /// TypeParameter children directly. After BIND, it uses the GenericsBehavior.
    pub fn type_parameters(&self) -> Vec<Arc<TypeParameterSymbol>> {
        // First try GenericsBehavior (available after BIND)
        if let Some(g) = self.metadata.get_behavior::<GenericsBehavior>() {
            return g.type_parameters().to_vec();
        }

        // Fallback: get TypeParameter children (available during BUILD)
        self.metadata
            .children()
            .into_iter()
            .filter_map(|c| {
                if c.metadata().kind() == KestrelSymbolKind::TypeParameter {
                    c.downcast_arc::<TypeParameterSymbol>().ok()
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if this function is generic (has type parameters)
    pub fn is_generic(&self) -> bool {
        self.metadata
            .get_behavior::<GenericsBehavior>()
            .map(|g| g.is_generic())
            .unwrap_or(false)
    }

    /// Get the number of type parameters
    ///
    /// During BUILD phase (before GenericsBehavior is attached), this counts
    /// TypeParameter children. After BIND, it uses the GenericsBehavior.
    pub fn type_parameter_count(&self) -> usize {
        // First try GenericsBehavior (available after BIND)
        if let Some(g) = self.metadata.get_behavior::<GenericsBehavior>() {
            return g.type_parameter_count();
        }

        // Fallback: count TypeParameter children (available during BUILD)
        self.metadata
            .children()
            .iter()
            .filter(|c| c.metadata().kind() == KestrelSymbolKind::TypeParameter)
            .count()
    }

    /// Get the where clause for this function.
    ///
    /// Delegates to GenericsBehavior. Returns empty where clause if not yet bound.
    pub fn where_clause(&self) -> WhereClause {
        self.metadata
            .get_behavior::<GenericsBehavior>()
            .map(|g| g.where_clause().clone())
            .unwrap_or_else(WhereClause::new)
    }

    /// Add a new local variable to this function.
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

    /// Get all locals in this function
    pub fn locals(&self) -> Vec<Local> {
        self.locals.read().unwrap().clone()
    }

    /// Get the number of locals
    pub fn local_count(&self) -> usize {
        self.locals.read().unwrap().len()
    }
}
