use std::sync::Arc;

use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::Symbol;

use crate::binders::flatten_protocol;
use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::diagnostics::NotAProtocolContext;
use crate::syntax::helpers::resolve_conformance_list;

/// Binder for protocol declarations
pub struct ProtocolBinder;

impl DeclarationBinder for ProtocolBinder {
    fn bind_signature(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        let symbol_id = symbol.metadata().id();

        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // Resolve attributes
        let attributes_behavior = crate::binders::utils::attributes::resolve_attributes(
            syntax,
            &source,
            file_id,
            context.diagnostics,
        );
        symbol.metadata().add_behavior(attributes_behavior.clone());

        // Extract doc comment
        if let Some(doc) = crate::binders::utils::doc_comment::extract_doc_comment(syntax) {
            symbol.metadata().add_behavior(doc);
        }

        // Process @builtin attribute if present
        self.process_builtin_attribute(symbol, &attributes_behavior, &source, context);

        // Resolve inherited protocols FIRST, before where clause
        // This is needed so that where clause can reference associated types from inherited protocols
        // e.g., protocol SortedIterator: Iterator where Iterator.Item: Comparable { }
        resolve_conformance_list(
            syntax,
            &source,
            file_id,
            symbol,
            symbol_id,
            context,
            NotAProtocolContext::Inheritance,
        );

        // Extract type parameters and resolve where clause bounds
        // Now inherited protocols are available for associated type path resolution
        let generics_behavior = crate::binders::utils::generics::resolve_generics(
            syntax, &source, file_id, symbol_id, context,
        );

        // Add GenericsBehavior
        symbol.metadata().add_behavior(generics_behavior);

        // Flatten protocol inheritance hierarchy
        if let Ok(protocol_symbol) = symbol.clone().downcast_arc::<ProtocolSymbol>()
            && let Some(flattened) = flatten_protocol(&protocol_symbol, context)
        {
            symbol.metadata().add_behavior(flattened);
        }
    }
}

impl ProtocolBinder {
    /// Process @builtin attribute on a protocol.
    ///
    /// Validates that:
    /// 1. The feature expects a protocol
    /// 2. If the feature requires a marker protocol, the protocol has no required members
    /// 3. The feature isn't already registered
    fn process_builtin_attribute(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        attributes: &AttributesBehavior,
        source: &str,
        context: &mut BindingContext,
    ) {
        let registry = context.model.builtin_registry().clone();
        crate::binders::utils::attributes::validate_builtin_attribute(
            symbol, attributes, source, context,
            "protocol",
            |k| k.is_protocol(),
            |f| registry.protocol(f),
        );
    }

}
