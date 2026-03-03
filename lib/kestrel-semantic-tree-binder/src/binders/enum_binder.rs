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
use semantic_tree::cycle::CycleDetector;
use semantic_tree::symbol::Symbol;

use crate::binders::utils::attributes::{BuiltinParseResult, parse_builtin_attribute};
use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::diagnostics::{
    BuiltinWrongKindError, CloneableFieldRequiresCloneableConformance, DuplicateBuiltinError,
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
        // 1. Guard: Only process enum symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Enum {
            return;
        }

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
        // Only process enum symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Enum {
            return;
        }

        // Compute and attach CopySemanticsBehavior based on conformances and case payload types
        // This is done in bind_body because enum cases are bound after the enum's signature
        Self::compute_copy_semantics(symbol, context);

        // Validate that protocols with disallow_enum_conformance are not conformed to
        Self::validate_disallowed_conformances(symbol, context);

        // Validate that protocols with requires_fields_conform have all payloads conforming
        Self::validate_protocol_field_conformances(symbol, context);
    }
}

impl EnumBinder {
    /// Compute and attach CopySemanticsBehavior based on conformances and case payload types.
    ///
    /// An enum's copy semantics are determined as follows:
    /// 1. If it has explicit `not Copyable` in its conformance list → NotCopyable
    /// 2. If any of its enum cases has a non-copyable payload type → NotCopyable
    /// 3. If it conforms to `Cloneable` → Cloneable
    /// 4. If any case payload is Cloneable but enum doesn't conform to Cloneable → ERROR
    /// 5. Otherwise → Copyable
    ///
    /// Uses cycle detection to handle recursive enum types.
    fn compute_copy_semantics(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        context: &mut BindingContext,
    ) {
        let symbol_id = symbol.metadata().id();

        // Use cycle detector to handle recursive types
        if CycleDetector::enter_ref(context.copy_semantics_cycle_detector, symbol_id).is_err() {
            // Cycle detected - just return, don't attach behavior
            return;
        }

        // Check if the Copyable protocol is registered
        let copyable_id = match context.model.builtin_registry().copyable_protocol() {
            Some(id) => id,
            None => {
                // Copyable protocol not registered yet; default to copyable
                symbol
                    .metadata()
                    .add_behavior(CopySemanticsBehavior::copyable());
                CycleDetector::exit_ref(context.copy_semantics_cycle_detector);
                return;
            },
        };

        // Get the conformances behavior for checking protocol conformances
        let conformances = symbol.metadata().get_behavior::<ConformancesBehavior>();

        // Check if enum has explicit `not Copyable`
        let has_not_copyable = conformances
            .as_ref()
            .map(|c| c.has_negative_conformance_to(copyable_id))
            .unwrap_or(false);

        // Check if any enum case has a non-copyable payload type
        // Enum cases with payloads have a CallableBehavior with parameter types
        let has_non_copyable_payload = symbol
            .metadata()
            .children()
            .iter()
            .filter(|child| child.metadata().kind() == KestrelSymbolKind::EnumCase)
            .any(|case| {
                case.metadata()
                    .get_behavior::<CallableBehavior>()
                    .map(|callable| {
                        callable
                            .parameters()
                            .iter()
                            .any(|param| !param.ty.is_copyable())
                    })
                    .unwrap_or(false)
            });

        // Rule 1 & 2: If explicitly not copyable or has non-copyable payload → NotCopyable
        if has_not_copyable || has_non_copyable_payload {
            symbol
                .metadata()
                .add_behavior(CopySemanticsBehavior::not_copyable());
            CycleDetector::exit_ref(context.copy_semantics_cycle_detector);
            return;
        }

        // Check if enum conforms to Cloneable
        let conforms_to_cloneable = context
            .model
            .builtin_registry()
            .cloneable_protocol()
            .map(|cloneable_id| {
                conformances
                    .as_ref()
                    .map(|c| Self::has_conformance_to(c, cloneable_id))
                    .unwrap_or(false)
            })
            .unwrap_or(false);

        // Check if any case payload has a cloneable type
        let has_cloneable_payload = symbol
            .metadata()
            .children()
            .iter()
            .filter(|child| child.metadata().kind() == KestrelSymbolKind::EnumCase)
            .any(|case| {
                case.metadata()
                    .get_behavior::<CallableBehavior>()
                    .map(|callable| {
                        callable
                            .parameters()
                            .iter()
                            .any(|param| param.ty.is_cloneable())
                    })
                    .unwrap_or(false)
            });

        // Rule 3: If conforms to Cloneable → Cloneable
        if conforms_to_cloneable {
            symbol
                .metadata()
                .add_behavior(CopySemanticsBehavior::cloneable());
            CycleDetector::exit_ref(context.copy_semantics_cycle_detector);
            return;
        }

        // Rule 4: If any case payload is Cloneable but enum doesn't conform to Cloneable → ERROR
        if has_cloneable_payload {
            // Find the first cloneable case and parameter for the diagnostic
            for case in symbol
                .metadata()
                .children()
                .iter()
                .filter(|child| child.metadata().kind() == KestrelSymbolKind::EnumCase)
            {
                if let Some(callable) = case.metadata().get_behavior::<CallableBehavior>()
                    && let Some(param) = callable.parameters().iter().find(|p| p.ty.is_cloneable())
                {
                    context
                        .diagnostics
                        .throw(CloneableFieldRequiresCloneableConformance {
                            type_span: symbol.metadata().span().clone(),
                            type_name: symbol.metadata().name().value.clone(),
                            field_name: param.bind_name.value.clone(),
                            field_span: case.metadata().span().clone(),
                            type_kind: "enum",
                        });
                    break;
                }
            }

            // Make it NotCopyable to be safe (can't implicitly copy cloneable payloads)
            symbol
                .metadata()
                .add_behavior(CopySemanticsBehavior::not_copyable());
            CycleDetector::exit_ref(context.copy_semantics_cycle_detector);
            return;
        }

        // Rule 5: Otherwise → Copyable
        symbol
            .metadata()
            .add_behavior(CopySemanticsBehavior::copyable());
        CycleDetector::exit_ref(context.copy_semantics_cycle_detector);
    }

    /// Check if conformances include a specific protocol by symbol ID
    fn has_conformance_to(
        conformances: &ConformancesBehavior,
        protocol_id: semantic_tree::symbol::SymbolId,
    ) -> bool {
        conformances.conformances().iter().any(|ty| {
            if let TyKind::Protocol { symbol, .. } = ty.kind() {
                symbol.metadata().id() == protocol_id
            } else {
                false
            }
        })
    }

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
        if !context
            .model
            .builtin_registry()
            .register_enum(feature, symbol_id)
        {
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
