//! Analyzer for field validation
//!
//! Validates properties according to Kestrel's semantics:
//! - Computed properties must use 'var', not 'let'
//! - Properties in global context cannot use 'static' modifier
//! - Enums cannot have non-static stored fields
//! - Generic types cannot have static stored properties

use std::sync::Arc;

use kestrel_semantic_tree::behavior::NamespaceScopeMarker;
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::{
    ComputedPropertyMustBeVarError, EnumStoredFieldError, GenericTypeStaticStoredPropertyError,
    GlobalPropertyStaticModifierError,
};

/// Analyzer that validates field properties
pub struct FieldAnalyzer;

impl FieldAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FieldAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for FieldAnalyzer {
    fn name(&self) -> &'static str {
        "field"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        // Only process Field symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Field {
            return;
        }

        let field = symbol
            .as_ref()
            .downcast_ref::<FieldSymbol>()
            .expect("Field symbol should downcast to FieldSymbol");

        // Check 1: computed properties must use 'var'
        if field.is_computed() && !field.is_mutable() {
            ctx.report(ComputedPropertyMustBeVarError {
                span: symbol.metadata().span().clone(),
            });
            // Continue checking other rules
        }

        // Get parent to check context
        let parent = match symbol.metadata().parent() {
            Some(p) => p,
            None => return, // No parent, skip remaining checks
        };

        let parent_kind = parent.metadata().kind();

        // Check 2: static modifier in global context
        if field.is_static()
            && parent
                .metadata()
                .get_behavior::<NamespaceScopeMarker>()
                .is_some()
        {
            ctx.report(GlobalPropertyStaticModifierError {
                span: symbol.metadata().span().clone(),
                is_computed: field.is_computed(),
            });
            return; // Don't check further rules if in global context
        }

        // Check 3: enums cannot have non-static stored fields
        if parent_kind == KestrelSymbolKind::Enum && !field.is_static() && !field.is_computed() {
            ctx.report(EnumStoredFieldError {
                span: symbol.metadata().span().clone(),
            });
            return;
        }

        // Check 4: static stored properties not supported in generic types
        if field.is_static() && !field.is_computed() {
            // Check if parent type is generic
            if let Some(generics) = parent.metadata().get_behavior::<GenericsBehavior>()
                && generics.is_generic()
            {
                let type_name = parent.metadata().name().value.clone();

                ctx.report(GenericTypeStaticStoredPropertyError {
                    span: symbol.metadata().span().clone(),
                    type_name,
                });
            }
        }
    }
}
