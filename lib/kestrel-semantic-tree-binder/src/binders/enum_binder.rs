use std::sync::Arc;

use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::copy_semantics::CopySemanticsBehavior;
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
    ProtocolDisallowsEnumConformanceError,
};
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
        let symbol_id = symbol.metadata().id();
        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // 2. Resolve attributes
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

        // 3. Resolve generics (type parameters + where clause)
        let generics_behavior = crate::binders::utils::generics::resolve_generics(
            syntax, &source, file_id, symbol_id, context,
        );
        symbol.metadata().add_behavior(generics_behavior);

        // 4. Resolve conformances (protocol conformance)
        resolve_conformance_list(
            syntax,
            &source,
            file_id,
            symbol,
            symbol_id,
            context,
            NotAProtocolContext::Conformance,
        );

        // Note: CopySemanticsBehavior is computed in bind_body after cases are bound
        // Note: Child binding (cases, methods) happens automatically
        // via recursive traversal in SemanticBinder
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

        // Emit diagnostic if enum has cloneable payload but doesn't conform to Cloneable
        crate::binders::copy_semantics_diagnostic::check_cloneable_field_diagnostic(
            symbol, "enum", context,
        );

        // Validate that protocols with disallow_enum_conformance are not conformed to
        Self::validate_disallowed_conformances(symbol, context);

        // Validate that protocols with requires_fields_conform have all payloads conforming
        Self::validate_protocol_field_conformances(symbol, context);
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

        // Registration happens in the pre-pass (register_all_builtins).
        // Here we only check for duplicates (a different symbol claiming the same feature).
        let symbol_id = symbol.metadata().id();
        let existing = context.model.builtin_registry().builtin_enum(feature);
        if existing.is_some() && existing != Some(symbol_id) {
            context.diagnostics.throw(DuplicateBuiltinError {
                span: attr_span,
                feature_name: feature.name().to_string(),
            });
        }
    }

    /// Validate that protocols with disallow_enum_conformance are not conformed to by this enum.
    fn validate_disallowed_conformances(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        context: &mut BindingContext,
    ) {
        let Some(conformances) = symbol.metadata().get_behavior::<ConformancesBehavior>() else {
            return;
        };

        for conformance_ty in conformances.conformances() {
            let TyKind::Protocol {
                symbol: protocol_sym,
                ..
            } = conformance_ty.kind()
            else {
                continue;
            };

            let protocol_id = protocol_sym.metadata().id();

            // Check if this is a builtin protocol with disallow_enum_conformance
            let Some(feature) = context
                .model
                .builtin_registry()
                .protocol_feature(protocol_id)
            else {
                continue;
            };

            let definition = feature.definition();
            if let BuiltinKind::Protocol {
                disallow_enum_conformance: true,
                ..
            } = definition.kind
            {
                context
                    .diagnostics
                    .throw(ProtocolDisallowsEnumConformanceError {
                        span: symbol.metadata().span().clone(),
                        enum_name: symbol.metadata().name().value.clone(),
                        protocol_name: protocol_sym.metadata().name().value.clone(),
                    });
            }
        }
    }

    /// Validate that protocols with requires_fields_conform have all case payloads conforming.
    ///
    /// For each protocol that the enum conforms to, if the protocol has the
    /// `requires_fields_conform` flag set, all enum case payloads must also conform to that protocol.
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

            // Collect non-conforming case payloads
            let mut non_conforming: Vec<NonConformingField> = Vec::new();

            for case in symbol
                .metadata()
                .children()
                .iter()
                .filter(|c| c.metadata().kind() == KestrelSymbolKind::EnumCase)
            {
                // Enum cases with payloads have a CallableBehavior with parameter types
                if let Some(callable) = case.metadata().get_behavior::<CallableBehavior>() {
                    for param in callable.parameters() {
                        if !context.model.conforms_to(&param.ty, protocol_id) {
                            non_conforming.push(NonConformingField {
                                field_name: format!(
                                    "{}.{}",
                                    case.metadata().name().value,
                                    param.bind_name.value
                                ),
                                field_ty: param.ty.to_string(),
                                span: case.metadata().span().clone(),
                            });
                        }
                    }
                }
            }

            if !non_conforming.is_empty() {
                context
                    .diagnostics
                    .throw(FieldsNotConformingToProtocolError {
                        type_span: symbol.metadata().span().clone(),
                        type_name: symbol.metadata().name().value.clone(),
                        type_kind: "enum",
                        protocol_name: protocol_sym.metadata().name().value.clone(),
                        non_conforming_fields: non_conforming,
                    });
            }
        }
    }
}
