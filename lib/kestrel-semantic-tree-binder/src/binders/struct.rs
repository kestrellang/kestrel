use std::sync::Arc;

use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;
use kestrel_semantic_tree::behavior::copy_semantics::CopySemanticsBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::Symbol;

use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::diagnostics::NotAProtocolContext;
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
        Self::process_builtin_attribute(symbol, &attributes_behavior, &source, context);

        // Extract type parameters and resolve where clause bounds
        let generics_behavior = crate::binders::utils::generics::resolve_generics(
            syntax, &source, file_id, symbol_id, context,
        );

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

        // Note: CopySemanticsBehavior is computed in bind_body after fields are bound
        // Note: Protocol method linking happens in the ConformanceValidator
        // during the VALIDATE phase, after all children are bound
    }

    fn bind_body(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        _syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Compute and attach CopySemanticsBehavior via query (unified for struct/enum)
        let semantics = context
            .model
            .query(kestrel_semantic_model::CopySemanticsFor {
                symbol_id: symbol.metadata().id(),
            });
        symbol
            .metadata()
            .add_behavior(CopySemanticsBehavior::new(semantics));

        // Diagnostic validations (cloneable field, parent protocol conformances,
        // protocol field conformances) are now in analyzers.
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
        let registry = context.model.builtin_registry().clone();
        crate::binders::utils::attributes::validate_builtin_attribute(
            symbol,
            attributes,
            source,
            context,
            "struct",
            |k| k.is_struct(),
            |f| registry.builtin_struct(f),
        );
    }
}
