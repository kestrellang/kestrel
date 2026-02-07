use std::collections::HashMap;
use std::sync::Arc;

use kestrel_span::{Name, Span};
use semantic_tree::behavior::Behavior;
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};

use crate::{
    behavior::KestrelBehaviorKind, behavior::generics::GenericsBehavior,
    behavior::visibility::VisibilityBehavior, language::KestrelLanguage,
    symbol::kind::KestrelSymbolKind, symbol::type_parameter::TypeParameterSymbol, ty::WhereClause,
};

use super::associated_type::AssociatedTypeSymbol;
use super::field::FieldSymbol;

/// Represents a protocol declaration in the semantic tree.
///
/// Protocols define interfaces that types can conform to.
///
/// # Type Resolution
///
/// During build phase, basic symbol information is captured.
/// During bind phase, `GenericsBehavior` is added with resolved type parameters and where clause.
/// Inherited protocols are added as `ConformancesBehavior`.
#[derive(Debug)]
pub struct ProtocolSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl Symbol<KestrelLanguage> for ProtocolSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl ProtocolSymbol {
    /// Create a new ProtocolSymbol with a name, span, visibility, and optional parent
    pub fn new(
        name: Name,
        span: Span,
        visibility: VisibilityBehavior,
        parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    ) -> Self {
        let mut builder = SymbolMetadataBuilder::new(KestrelSymbolKind::Protocol)
            .with_name(name.clone())
            .with_declaration_span(name.span.clone())
            .with_span(span)
            .with_behavior(Arc::new(visibility));

        if let Some(p) = parent {
            builder = builder.with_parent(Arc::downgrade(&p));
        }

        ProtocolSymbol {
            metadata: builder.build(),
        }
    }

    /// Get the type parameters for this protocol.
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

    /// Check if this protocol is generic (has type parameters)
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

    /// Get the where clause for this protocol.
    ///
    /// Delegates to GenericsBehavior. Returns empty where clause if not yet bound.
    pub fn where_clause(&self) -> WhereClause {
        self.metadata
            .get_behavior::<GenericsBehavior>()
            .map(|g| g.where_clause().clone())
            .unwrap_or_default()
    }
}

/// Flattened view of a protocol's methods, properties, and associated types including inheritance.
/// Computed during BIND phase to enable O(1) member lookups.
#[derive(Debug, Clone)]
pub struct FlattenedProtocolBehavior {
    /// All methods (direct + inherited), grouped by name
    methods: HashMap<String, Vec<FlattenedMethod>>,

    /// All computed property requirements (direct + inherited)
    properties: HashMap<String, FlattenedProperty>,

    /// All associated types (direct + inherited)
    associated_types: HashMap<String, FlattenedAssociatedType>,

    /// Maximum inheritance depth (for metrics/debugging)
    inheritance_depth: usize,
}

#[derive(Debug, Clone)]
pub struct FlattenedMethod {
    /// The method symbol
    pub symbol: Arc<dyn Symbol<KestrelLanguage>>,

    /// Which protocol defined this method (for error messages)
    pub source_protocol_name: String,

    /// Span where the method was defined (for error messages)
    pub definition_span: Span,
}

#[derive(Debug, Clone)]
pub struct FlattenedAssociatedType {
    /// The associated type symbol
    pub symbol: Arc<AssociatedTypeSymbol>,

    /// Which protocol defined this (for error messages)
    pub source_protocol_name: String,

    /// Span where it was defined (for error messages)
    pub definition_span: Span,
}

/// Flattened property requirement from a protocol (computed properties only).
#[derive(Debug, Clone)]
pub struct FlattenedProperty {
    /// The field symbol (computed property)
    pub symbol: Arc<FieldSymbol>,

    /// Which protocol defined this property
    pub source_protocol_name: String,

    /// Span where it was defined (for error messages)
    pub definition_span: Span,

    /// Whether this property has a getter requirement
    pub has_getter: bool,

    /// Whether this property has a setter requirement
    pub has_setter: bool,

    /// Whether this is a static property
    pub is_static: bool,
}

impl FlattenedProtocolBehavior {
    pub fn new(
        methods: HashMap<String, Vec<FlattenedMethod>>,
        properties: HashMap<String, FlattenedProperty>,
        associated_types: HashMap<String, FlattenedAssociatedType>,
        inheritance_depth: usize,
    ) -> Self {
        Self {
            methods,
            properties,
            associated_types,
            inheritance_depth,
        }
    }

    pub fn methods(&self) -> &HashMap<String, Vec<FlattenedMethod>> {
        &self.methods
    }

    pub fn properties(&self) -> &HashMap<String, FlattenedProperty> {
        &self.properties
    }

    pub fn associated_types(&self) -> &HashMap<String, FlattenedAssociatedType> {
        &self.associated_types
    }

    pub fn inheritance_depth(&self) -> usize {
        self.inheritance_depth
    }
}

impl Behavior<KestrelLanguage> for FlattenedProtocolBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::FlattenedProtocol
    }
}
