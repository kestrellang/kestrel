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

/// Binder for enum declarations
pub struct EnumBinder;

impl DeclarationBinder for EnumBinder {
    fn bind_signature(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // 1. Guard: Only process enum symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Enum {
            return;
        }

        let symbol_id = symbol.metadata().id();
        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // 2. Resolve attributes
        let attributes_behavior =
            crate::binders::utils::attributes::resolve_attributes(syntax, &source, context.diagnostics);
        symbol.metadata().add_behavior(attributes_behavior.clone());

        // Process @builtin attribute if present
        Self::process_builtin_attribute(symbol, &attributes_behavior, &source, context);

        // 3. Resolve generics (type parameters + where clause)
        let generics_behavior = crate::binders::utils::generics::resolve_generics(
            syntax, &source, file_id, symbol_id, context,
        );
        symbol.metadata().add_behavior(generics_behavior);

        // 3. Resolve conformances (protocol conformance)
        resolve_conformance_list(
            syntax,
            &source,
            file_id,
            symbol,
            symbol_id,
            context,
            NotAProtocolContext::Conformance,
        );

        // Note: Child binding (cases, methods) happens automatically
        // via recursive traversal in SemanticBinder
    }
}

impl EnumBinder {
    /// Process @builtin attribute on an enum.
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

        // Validate: feature must expect an enum
        if !definition.kind.is_enum() {
            context.diagnostics.throw(BuiltinWrongKindError {
                span: attr_span,
                feature_name: feature.name().to_string(),
                expected_kind: definition.kind.kind_name().to_string(),
                actual_kind: "enum".to_string(),
            });
            return;
        }

        // Register the builtin
        let symbol_id = symbol.metadata().id();
        if !context.model.builtin_registry().register_enum(feature, symbol_id) {
            context.diagnostics.throw(DuplicateBuiltinError {
                span: attr_span,
                feature_name: feature.name().to_string(),
            });
        }
    }
}
