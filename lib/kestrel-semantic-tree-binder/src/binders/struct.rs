use std::sync::Arc;

use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::Symbol;

use crate::binders::utils::attributes::{parse_builtin_attribute, BuiltinParseResult};
use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::diagnostics::{BuiltinWrongKindError, DuplicateBuiltinError, NotAProtocolContext};
use crate::syntax::helpers::resolve_conformance_list;

/// Binder for struct declarations
pub struct StructBinder;

impl DeclarationBinder for StructBinder {
    fn bind_signature(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Only process struct symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Struct {
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
        Self::process_builtin_attribute(symbol, &attributes_behavior, &source, context);

        // Extract type parameters and resolve where clause bounds
        let generics_behavior =
            crate::binders::utils::generics::resolve_generics(syntax, &source, file_id, symbol_id, context);

        // Add GenericsBehavior
        symbol.metadata().add_behavior(generics_behavior);

        // Resolve conformances from syntax and store them
        resolve_conformance_list(
            syntax,
            &source,
            file_id,
            symbol,
            symbol_id,
            context,
            NotAProtocolContext::Conformance,
        );

        // Note: Protocol method linking happens in the ConformanceValidator
        // during the VALIDATE phase, after all children are bound
    }
}

impl StructBinder {
    /// Process @builtin attribute on a struct.
    fn process_builtin_attribute(
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

        // Validate: feature must expect a struct
        if !definition.kind.is_struct() {
            context.diagnostics.throw(BuiltinWrongKindError {
                span: attr_span,
                feature_name: feature.name().to_string(),
                expected_kind: definition.kind.kind_name().to_string(),
                actual_kind: "struct".to_string(),
            });
            return;
        }

        // Register the builtin
        let symbol_id = symbol.metadata().id();
        if !context.model.builtin_registry().register_struct(feature, symbol_id) {
            context.diagnostics.throw(DuplicateBuiltinError {
                span: attr_span,
                feature_name: feature.name().to_string(),
            });
        }
    }
}
