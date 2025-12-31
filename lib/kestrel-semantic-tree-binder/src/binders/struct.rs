use std::sync::Arc;

use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::copy_semantics::{CopySemantics, CopySemanticsBehavior};
use kestrel_semantic_tree::behavior::deinit::DeinitBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::cycle::CycleDetector;
use semantic_tree::symbol::Symbol;

use crate::binders::utils::attributes::{parse_builtin_attribute, BuiltinParseResult};
use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::diagnostics::{BuiltinWrongKindError, CopyableWithDeinitWarning, DuplicateBuiltinError, NotAProtocolContext};
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

        // Check for Copyable + deinit combination and emit warning
        Self::check_copyable_with_deinit(symbol, context);
    }
}

impl StructBinder {
    /// Compute and attach CopySemanticsBehavior based on conformances and field types.
    ///
    /// A struct is NotCopyable if:
    /// 1. It has explicit `not Copyable` in its conformance list, OR
    /// 2. Any of its fields has a non-copyable type
    ///
    /// Uses cycle detection to handle recursive struct types - if a cycle is detected
    /// during computation, we just skip (another analyzer will catch the cycle error).
    fn compute_copy_semantics(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        context: &BindingContext,
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
                symbol.metadata().add_behavior(CopySemanticsBehavior::copyable());
                CycleDetector::exit_ref(context.copy_semantics_cycle_detector);
                return;
            }
        };

        // Get the ConformancesBehavior to check for negative conformances
        let has_not_copyable = symbol
            .metadata()
            .get_behavior::<ConformancesBehavior>()
            .map(|conformances| conformances.has_negative_conformance_to(copyable_id))
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

        // Attach the appropriate CopySemanticsBehavior
        let behavior = if has_not_copyable || has_non_copyable_field {
            CopySemanticsBehavior::not_copyable()
        } else {
            CopySemanticsBehavior::copyable()
        };

        symbol.metadata().add_behavior(behavior);
        CycleDetector::exit_ref(context.copy_semantics_cycle_detector);
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
        if !context.model.builtin_registry().register_struct(feature, symbol_id) {
            context.diagnostics.throw(DuplicateBuiltinError {
                span: attr_span,
                feature_name: feature.name().to_string(),
            });
        }
    }

    /// Check for Copyable type with deinit and emit a warning.
    ///
    /// This is allowed but potentially confusing - the deinit will run for each copy
    /// of the value, which may not be the intended behavior.
    fn check_copyable_with_deinit(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        context: &mut BindingContext,
    ) {
        // Check if struct has a deinit
        let Some(deinit_behavior) = symbol.metadata().get_behavior::<DeinitBehavior>() else {
            return;
        };

        // Check if struct is Copyable
        let is_copyable = symbol
            .metadata()
            .get_behavior::<CopySemanticsBehavior>()
            .map(|b| b.semantics() == CopySemantics::Copyable)
            .unwrap_or(true); // Default to copyable if no behavior

        if !is_copyable {
            // Not copyable, no warning needed
            return;
        }

        // Get the deinit span for the warning
        let deinit_span = context
            .model
            .registry()
            .get(deinit_behavior.deinit_symbol())
            .map(|s| s.metadata().span().clone())
            .unwrap_or_else(|| symbol.metadata().span().clone());

        let struct_name = symbol.metadata().name().value.clone();

        context.diagnostics.throw(CopyableWithDeinitWarning {
            deinit_span,
            struct_name,
        });
    }
}
