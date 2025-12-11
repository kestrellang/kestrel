//! Validator for extension method conflicts
//!
//! This validator checks that extensions targeting the same type at the same
//! specificity level don't have methods with the same signature. Extensions with
//! different specificity levels (e.g., Box[T] vs Box[Int]) are not considered
//! conflicting - the more specific one will be preferred at method resolution time.

use std::collections::HashMap;
use std::sync::Mutex;

use kestrel_reporting::{Diagnostic, DiagnosticContext, IntoDiagnostic, Label};
use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::database::SemanticDatabase;
use crate::validation::{SymbolContext, Validator};

/// Validator that checks for conflicting methods across extensions
pub struct ExtensionConflictValidator {
    /// Collected extensions during the walk, grouped by (target_id, specificity)
    extensions_by_target: Mutex<HashMap<(SymbolId, usize), Vec<CollectedExtension>>>,
}

/// Data collected for an extension
struct CollectedExtension {
    extension_id: SymbolId,
    extension_span: Span,
    /// Methods in this extension: (name, span)
    methods: Vec<(String, Span)>,
}

impl ExtensionConflictValidator {
    const NAME: &'static str = "extension_conflict";

    pub fn new() -> Self {
        Self {
            extensions_by_target: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for ExtensionConflictValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for ExtensionConflictValidator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn validate_symbol(&self, ctx: &SymbolContext<'_>) {
        // Only process extensions
        if ctx.symbol.metadata().kind() != KestrelSymbolKind::Extension {
            return;
        }

        // Get the extension symbol
        let Ok(extension) = ctx.symbol.clone().downcast_arc::<ExtensionSymbol>() else {
            return;
        };

        // Get the target type from ExtensionTargetBehavior
        let Some(target_beh) = extension.extension_target_behavior() else {
            return;
        };

        let target_ty = target_beh.target_type();

        // Get target struct ID and substitutions from the target type
        let (target_id, substitutions) = match target_ty.kind() {
            kestrel_semantic_tree::ty::TyKind::Struct {
                symbol,
                substitutions,
            } => (symbol.metadata().id(), substitutions),
            _ => return,
        };

        // Calculate specificity: count non-type-parameter type arguments
        // This matches the logic in filter_applicable_extensions in members.rs
        let specificity = substitutions
            .types()
            .filter(|ty| !ty.is_type_parameter())
            .count();

        // Collect methods from this extension
        let methods: Vec<(String, Span)> = extension
            .metadata()
            .children()
            .into_iter()
            .filter(|c| c.metadata().kind() == KestrelSymbolKind::Function)
            .map(|c| (c.metadata().name().value.clone(), c.metadata().name().span.clone()))
            .collect();

        // Add to collection, grouped by (target_id, specificity)
        let collected = CollectedExtension {
            extension_id: extension.metadata().id(),
            extension_span: extension.metadata().span().clone(),
            methods,
        };

        self.extensions_by_target
            .lock()
            .unwrap()
            .entry((target_id, specificity))
            .or_default()
            .push(collected);
    }

    fn finalize(&self, _db: &SemanticDatabase, diagnostics: &mut DiagnosticContext) {
        let extensions_by_target = self.extensions_by_target.lock().unwrap();

        // For each (target_id, specificity) group with extensions
        for (_key, extensions) in extensions_by_target.iter() {
            // Skip if only one extension at this specificity level
            if extensions.len() <= 1 {
                continue;
            }

            // Check for method conflicts between extensions at the same specificity level
            // Build a map of method_name -> Vec<(extension_id, method_span)>
            let mut method_locations: HashMap<String, Vec<(SymbolId, Span)>> = HashMap::new();

            for ext in extensions {
                for (method_name, method_span) in &ext.methods {
                    method_locations
                        .entry(method_name.clone())
                        .or_default()
                        .push((ext.extension_id, method_span.clone()));
                }
            }

            // Report conflicts (same method in multiple extensions at same specificity)
            for (method_name, locations) in method_locations {
                if locations.len() > 1 {
                    // Conflict found!
                    let error = DuplicateExtensionMethodError {
                        method_name,
                        locations: locations.into_iter().map(|(_, span)| span).collect(),
                    };
                    diagnostics.add_diagnostic(error.into_diagnostic(0));
                }
            }
        }
    }
}

/// Error for duplicate method definitions across extensions
#[derive(Debug, Clone)]
pub struct DuplicateExtensionMethodError {
    pub method_name: String,
    pub locations: Vec<Span>,
}

impl IntoDiagnostic for DuplicateExtensionMethodError {
    fn into_diagnostic(&self, _file_id: usize) -> Diagnostic<usize> {
        let mut labels: Vec<Label<usize>> = self
            .locations
            .iter()
            .enumerate()
            .map(|(i, span)| {
                let msg = if i == 0 {
                    "first definition here"
                } else {
                    "conflicting definition here"
                };
                Label::primary(0, span.clone()).with_message(msg)
            })
            .collect();

        // Make only the first label primary, rest secondary
        for label in labels.iter_mut().skip(1) {
            *label = Label::secondary(0, label.range.clone())
                .with_message(label.message.clone());
        }

        Diagnostic::error()
            .with_message(format!(
                "duplicate method '{}' in extensions with the same specificity",
                self.method_name
            ))
            .with_labels(labels)
            .with_notes(vec![
                "Extensions at the same specificity level cannot define methods with the same name"
                    .to_string(),
                "Extensions with different specificity (e.g., Box[T] vs Box[Int]) can have methods with the same name - the more specific one will be preferred"
                    .to_string(),
            ])
    }
}
