use std::sync::Arc;

use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::copy_semantics::CopySemanticsBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::builtins::BuiltinKind;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::TyKind;
use kestrel_semantic_type_inference::TypeOracle;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::Symbol;

use crate::binders::utils::attributes::{BuiltinParseResult, parse_builtin_attribute};
use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::diagnostics::{
    BuiltinWrongKindError, DuplicateBuiltinError,
    FieldsNotConformingToProtocolError, NonConformingField, NotAProtocolContext,
};
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
        let semantics = context.model.query(kestrel_semantic_model::CopySemanticsFor {
            symbol_id: symbol.metadata().id(),
        });
        symbol
            .metadata()
            .add_behavior(CopySemanticsBehavior::new(semantics));

        // Emit diagnostic if struct has cloneable field but doesn't conform to Cloneable
        crate::binders::copy_semantics_diagnostic::check_cloneable_field_diagnostic(
            symbol, "struct", context,
        );

        // Validate that protocols with requires_fields_conform have all fields conforming
        Self::validate_protocol_field_conformances(symbol, context);
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
        if !context
            .model
            .builtin_registry()
            .register_struct(feature, symbol_id)
        {
            context.diagnostics.throw(DuplicateBuiltinError {
                span: attr_span,
                feature_name: feature.name().to_string(),
            });
        }
    }

    /// Validate that protocols with requires_fields_conform have all fields conforming.
    ///
    /// For each protocol that the struct conforms to, if the protocol has the
    /// `requires_fields_conform` flag set, all fields must also conform to that protocol.
    fn validate_protocol_field_conformances(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        context: &mut BindingContext,
    ) {
        let Some(conformances) = symbol.metadata().get_behavior::<ConformancesBehavior>() else {
            return;
        };

        // For each protocol conformance, check if it has requires_fields_conform flag
        for conformance_ty in conformances.conformances() {
            let TyKind::Protocol {
                symbol: protocol_sym,
                ..
            } = conformance_ty.kind()
            else {
                continue;
            };

            let protocol_id = protocol_sym.metadata().id();

            // Check if this is a builtin protocol with requires_fields_conform
            let Some(feature) = context
                .model
                .builtin_registry()
                .protocol_feature(protocol_id)
            else {
                continue;
            };

            let definition = feature.definition();
            let BuiltinKind::Protocol {
                requires_fields_conform: true,
                ..
            } = definition.kind
            else {
                continue;
            };

            // Collect non-conforming fields
            let mut non_conforming: Vec<NonConformingField> = Vec::new();

            for field in symbol
                .metadata()
                .children()
                .iter()
                .filter(|c| c.metadata().kind() == KestrelSymbolKind::Field)
            {
                if let Some(typed) = field.metadata().get_behavior::<TypedBehavior>() {
                    let field_ty = typed.ty();
                    if !context.model.conforms_to(field_ty, protocol_id) {
                        non_conforming.push(NonConformingField {
                            field_name: field.metadata().name().value.clone(),
                            field_ty: field_ty.to_string(),
                            span: field.metadata().span().clone(),
                        });
                    }
                }
            }

            if !non_conforming.is_empty() {
                context
                    .diagnostics
                    .throw(FieldsNotConformingToProtocolError {
                        type_span: symbol.metadata().span().clone(),
                        type_name: symbol.metadata().name().value.clone(),
                        type_kind: "struct",
                        protocol_name: protocol_sym.metadata().name().value.clone(),
                        non_conforming_fields: non_conforming,
                    });
            }
        }
    }
}
