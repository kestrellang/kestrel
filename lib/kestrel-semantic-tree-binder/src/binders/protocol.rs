use std::sync::Arc;

use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;
use kestrel_semantic_tree::builtins::BuiltinKind;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::Symbol;

use crate::binders::flatten_protocol;
use crate::binders::utils::attributes::{parse_builtin_attribute, BuiltinParseResult};
use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::diagnostics::{
    BuiltinMustBeMarkerError, BuiltinWrongKindError, DuplicateBuiltinError, NotAProtocolContext,
};
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
        // Only process protocol symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Protocol {
            return;
        }

        let symbol_id = symbol.metadata().id();

        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // Resolve attributes
        let attributes_behavior =
            crate::binders::utils::attributes::resolve_attributes(syntax, &source, context.diagnostics);
        symbol.metadata().add_behavior(attributes_behavior.clone());

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
        let generics_behavior =
            crate::binders::utils::generics::resolve_generics(syntax, &source, file_id, symbol_id, context);

        // Add GenericsBehavior
        symbol.metadata().add_behavior(generics_behavior);

        // Flatten protocol inheritance hierarchy
        if let Ok(protocol_symbol) = symbol.clone().downcast_arc::<ProtocolSymbol>() {
            if let Some(flattened) = flatten_protocol(&protocol_symbol, context) {
                symbol.metadata().add_behavior(flattened);
            }
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
        let feature = match parse_builtin_attribute(attributes, source, context.diagnostics) {
            BuiltinParseResult::Success(f) => f,
            BuiltinParseResult::NotBuiltin | BuiltinParseResult::Error => return,
        };

        let definition = feature.definition();
        let attr_span = attributes
            .get_kind(kestrel_semantic_tree::attributes::AttributeKind::Builtin)
            .map(|a| a.span.clone())
            .unwrap_or_else(|| symbol.metadata().span().clone());

        // Validate: feature must expect a protocol
        if !definition.kind.is_protocol() {
            context.diagnostics.throw(BuiltinWrongKindError {
                span: attr_span,
                feature_name: feature.name().to_string(),
                expected_kind: definition.kind.kind_name().to_string(),
                actual_kind: "protocol".to_string(),
            });
            return;
        }

        // Validate: if must_be_marker, check that protocol has no required members
        if let BuiltinKind::Protocol { must_be_marker, .. } = &definition.kind {
            if *must_be_marker && !self.is_marker_protocol(symbol) {
                context.diagnostics.throw(BuiltinMustBeMarkerError {
                    span: attr_span,
                    feature_name: feature.name().to_string(),
                });
                return;
            }
        }

        // Register the builtin
        let symbol_id = symbol.metadata().id();
        if !context.model.builtin_registry().register_protocol(feature, symbol_id) {
            context.diagnostics.throw(DuplicateBuiltinError {
                span: attr_span,
                feature_name: feature.name().to_string(),
            });
        }
    }

    /// Check if a protocol is a marker protocol (no required methods or associated types).
    fn is_marker_protocol(&self, symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> bool {
        for child in symbol.metadata().children() {
            let kind = child.metadata().kind();
            // If protocol has functions or associated types, it's not a marker
            if kind == KestrelSymbolKind::Function || kind == KestrelSymbolKind::AssociatedType {
                return false;
            }
        }
        true
    }
}
