use std::sync::{Arc, RwLock};

use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    behavior::visibility::VisibilityBehavior,
    language::KestrelLanguage,
    symbol::getter::GetterSymbol,
    symbol::kind::KestrelSymbolKind,
    symbol::local::{Local, LocalContainer, LocalId},
    symbol::setter::SetterSymbol,
    ty::Ty,
};

/// Represents a subscript declaration in the semantic tree.
///
/// Subscripts provide indexed or keyed access to collection elements using
/// call-like syntax. They are similar to computed properties but accept parameters.
///
/// # Structure
///
/// A subscript is a parent symbol that contains:
/// - GetterSymbol (always present) - for reading values
/// - SetterSymbol (optional) - for writing values
/// - TypeParameterSymbol children (if generic)
///
/// # Examples
///
/// ```kestrel
/// // Getter-only subscript
/// public subscript(index: Int) -> T {
///     self.storage.buffer(unchecked: index)
/// }
///
/// // Getter and setter
/// public subscript(index: Int) -> T {
///     get { self.storage.buffer(unchecked: index) }
///     set { self.storage.buffer(unchecked: index) = newValue }
/// }
///
/// // Static subscript
/// public static subscript(key: String) -> Optional[Any] {
///     get { Self.cache(key) }
///     set { Self.cache(key) = newValue }
/// }
/// ```
///
/// # Overloading
///
/// Subscripts can be overloaded by parameter labels and types.
/// The `SubscriptBehavior` is used for overload resolution.
#[derive(Debug)]
pub struct SubscriptSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
    is_static: bool,
    /// Local variables within this subscript (parameters)
    /// Populated during binding phase.
    locals: RwLock<Vec<Local>>,
}

impl Symbol<KestrelLanguage> for SubscriptSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl LocalContainer for SubscriptSymbol {
    fn add_local(&self, name: String, ty: Ty, mutable: bool, span: Span) -> LocalId {
        let mut locals = self.locals.write().unwrap();
        let id = LocalId::new(locals.len());
        locals.push(Local::new(id, name, ty, mutable, span));
        id
    }

    fn get_local(&self, id: LocalId) -> Option<Local> {
        let locals = self.locals.read().unwrap();
        locals.get(id.index()).cloned()
    }

    fn locals(&self) -> Vec<Local> {
        self.locals.read().unwrap().clone()
    }

    fn local_count(&self) -> usize {
        self.locals.read().unwrap().len()
    }

    fn update_local_type(&self, id: LocalId, ty: Ty) {
        let mut locals = self.locals.write().unwrap();
        if let Some(local) = locals.get_mut(id.index()) {
            *local.ty_mut() = ty;
        }
    }
}

impl SubscriptSymbol {
    /// Create a new SubscriptSymbol
    ///
    /// # Arguments
    /// * `id` - The unique symbol ID
    /// * `span` - The full span of the subscript declaration
    /// * `declaration_span` - The span of the `subscript` keyword
    /// * `visibility` - The visibility behavior for access control
    /// * `is_static` - Whether this is a static subscript
    /// * `parent` - The parent symbol (struct, enum, protocol, or extension)
    pub fn new(
        id: SymbolId,
        span: Span,
        declaration_span: Span,
        visibility: VisibilityBehavior,
        is_static: bool,
        parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    ) -> Self {
        // Subscripts use a synthetic name since they are identified by their signature
        let name = kestrel_span::Name::new("subscript".to_string(), declaration_span.clone());

        let mut builder = SymbolMetadataBuilder::new(KestrelSymbolKind::Subscript)
            .with_id(id)
            .with_name(name)
            .with_declaration_span(declaration_span)
            .with_span(span)
            .with_behavior(Arc::new(visibility));

        if let Some(p) = parent {
            builder = builder.with_parent(Arc::downgrade(&p));
        }

        SubscriptSymbol {
            metadata: builder.build(),
            is_static,
            locals: RwLock::new(Vec::new()),
        }
    }

    /// Check if this subscript is static
    pub fn is_static(&self) -> bool {
        self.is_static
    }

    /// Get the getter child symbol for this subscript
    ///
    /// Every subscript has a getter for reading values.
    pub fn getter(&self) -> Option<Arc<GetterSymbol>> {
        self.metadata
            .children()
            .into_iter()
            .find(|child| child.metadata().kind() == KestrelSymbolKind::Getter)
            .and_then(|s| s.downcast_arc::<GetterSymbol>().ok())
    }

    /// Get the getter symbol ID for this subscript (if it exists)
    pub fn getter_id(&self) -> Option<SymbolId> {
        self.metadata
            .children()
            .into_iter()
            .find(|child| child.metadata().kind() == KestrelSymbolKind::Getter)
            .map(|s| s.metadata().id())
    }

    /// Get the setter child symbol for this subscript (if it exists)
    ///
    /// Returns None if this is a getter-only subscript.
    pub fn setter(&self) -> Option<Arc<SetterSymbol>> {
        self.metadata
            .children()
            .into_iter()
            .find(|child| child.metadata().kind() == KestrelSymbolKind::Setter)
            .and_then(|s| s.downcast_arc::<SetterSymbol>().ok())
    }

    /// Get the setter symbol ID for this subscript (if it exists)
    pub fn setter_id(&self) -> Option<SymbolId> {
        self.metadata
            .children()
            .into_iter()
            .find(|child| child.metadata().kind() == KestrelSymbolKind::Setter)
            .map(|s| s.metadata().id())
    }

    /// Check if this subscript has a setter (is read-write)
    pub fn has_setter(&self) -> bool {
        self.metadata
            .children()
            .iter()
            .any(|child| child.metadata().kind() == KestrelSymbolKind::Setter)
    }

    /// Check if this subscript is read-only (no setter)
    pub fn is_read_only(&self) -> bool {
        !self.has_setter()
    }
}
