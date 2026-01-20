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
use semantic_tree::cycle::CycleDetector;
use semantic_tree::symbol::Symbol;

use crate::binders::utils::attributes::{BuiltinParseResult, parse_builtin_attribute};
use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::diagnostics::{
    BuiltinWrongKindError, CloneableFieldRequiresCloneableConformance, DuplicateBuiltinError,
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
        // Only process struct symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Struct {
            return;
        }

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
        // Only process struct symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Struct {
            return;
        }

        // Compute and attach CopySemanticsBehavior based on conformances and field types
        // This is done in bind_body because fields are bound after the struct's signature
        Self::compute_copy_semantics(symbol, context);

        // Validate that protocols with requires_fields_conform have all fields conforming
        Self::validate_protocol_field_conformances(symbol, context);
    }
}

impl StructBinder {
    /// Compute and attach CopySemanticsBehavior based on conformances and field types.
    ///
    /// A struct's copy semantics are determined as follows:
    /// 1. If it has explicit `not Copyable` in its conformance list → NotCopyable
    /// 2. If any of its fields has a non-copyable type → NotCopyable
    /// 3. If it conforms to `Cloneable` → Cloneable
    /// 4. If any field is Cloneable but struct doesn't conform to Cloneable → ERROR
    /// 5. Otherwise → Copyable
    ///
    /// Uses cycle detection to handle recursive struct types - if a cycle is detected
    /// during computation, we just skip (another analyzer will catch the cycle error).
    fn compute_copy_semantics(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        context: &mut BindingContext,
    ) {
        let symbol_id = symbol.metadata().id();

        // Use cycle detector to handle recursive types (e.g., struct A { b: B }, struct B { a: A })
        // If we're already computing this type's copy semantics, just return.
        // The other analyzer will catch the actual cycle error.
        if CycleDetector::enter_ref(context.copy_semantics_cycle_detector, symbol_id).is_err() {
            // Cycle detected - just return, don't attach behavior
            // The final behavior will be determined when we unwind
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
            }
        };

        // Get the conformances behavior for checking protocol conformances
        let conformances = symbol.metadata().get_behavior::<ConformancesBehavior>();

        // Check if struct has explicit `not Copyable`
        let has_not_copyable = conformances
            .as_ref()
            .map(|c| c.has_negative_conformance_to(copyable_id))
            .unwrap_or(false);

        // Check if any field has a non-copyable type
        let has_non_copyable_field = symbol
            .metadata()
            .children()
            .iter()
            .filter(|child| child.metadata().kind() == KestrelSymbolKind::Field)
            .any(|field| {
                field
                    .metadata()
                    .get_behavior::<TypedBehavior>()
                    .map(|typed| !typed.ty().is_copyable())
                    .unwrap_or(false)
            });

        // Rule 1 & 2: If explicitly not copyable or has non-copyable field → NotCopyable
        if has_not_copyable || has_non_copyable_field {
            symbol
                .metadata()
                .add_behavior(CopySemanticsBehavior::not_copyable());
            CycleDetector::exit_ref(context.copy_semantics_cycle_detector);
            return;
        }

        // Check if struct conforms to Cloneable
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

        // Check if any field has a cloneable type
        let has_cloneable_field = symbol
            .metadata()
            .children()
            .iter()
            .filter(|child| child.metadata().kind() == KestrelSymbolKind::Field)
            .any(|field| {
                field
                    .metadata()
                    .get_behavior::<TypedBehavior>()
                    .map(|typed| typed.ty().is_cloneable())
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

        // Rule 4: If any field is Cloneable but struct doesn't conform to Cloneable → ERROR
        if has_cloneable_field {
            // Find the first cloneable field for the diagnostic
            if let Some(cloneable_field) = symbol
                .metadata()
                .children()
                .iter()
                .filter(|child| child.metadata().kind() == KestrelSymbolKind::Field)
                .find(|field| {
                    field
                        .metadata()
                        .get_behavior::<TypedBehavior>()
                        .map(|typed| typed.ty().is_cloneable())
                        .unwrap_or(false)
                })
            {
                context
                    .diagnostics
                    .throw(CloneableFieldRequiresCloneableConformance {
                        type_span: symbol.metadata().span().clone(),
                        type_name: symbol.metadata().name().value.clone(),
                        field_name: cloneable_field.metadata().name().value.clone(),
                        field_span: cloneable_field.metadata().span().clone(),
                        type_kind: "struct",
                    });
            }

            // Make it NotCopyable to be safe (can't implicitly copy cloneable fields)
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
        use kestrel_semantic_tree::ty::TyKind;
        conformances.conformances().iter().any(|ty| {
            if let TyKind::Protocol { symbol, .. } = ty.kind() {
                symbol.metadata().id() == protocol_id
            } else {
                false
            }
        })
    }

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
