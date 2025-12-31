use std::sync::Arc;

use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::copy_semantics::CopySemanticsBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::cycle::CycleDetector;
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
    }
}

impl EnumBinder {
    /// Compute and attach CopySemanticsBehavior based on conformances and case payload types.
    ///
    /// An enum is NotCopyable if:
    /// 1. It has explicit `not Copyable` in its conformance list, OR
    /// 2. Any of its enum cases has a non-copyable payload type
    ///
    /// Uses cycle detection to handle recursive enum types.
    fn compute_copy_semantics(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        context: &BindingContext,
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
                        callable.parameters().iter().any(|param| !param.ty.is_copyable())
                    })
                    .unwrap_or(false)
            });

        // Attach the appropriate CopySemanticsBehavior
        let behavior = if has_not_copyable || has_non_copyable_payload {
            CopySemanticsBehavior::not_copyable()
        } else {
            CopySemanticsBehavior::copyable()
        };

        symbol.metadata().add_behavior(behavior);
        CycleDetector::exit_ref(context.copy_semantics_cycle_detector);
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
        if !context.model.builtin_registry().register_enum(feature, symbol_id) {
            context.diagnostics.throw(DuplicateBuiltinError {
                span: attr_span,
                feature_name: feature.name().to_string(),
            });
        }
    }
}
