//! Validator for extension method conflicts
//!
//! This validator checks that extensions targeting the same type at the same
//! specificity level don't have methods with the same signature. Extensions with
//! different specificity levels (e.g., Box[T] vs Box[Int]) are not considered
//! conflicting - the more specific one will be preferred at method resolution time.
//!
//! This validator also checks that extension methods don't duplicate methods already
//! defined on the struct itself (struct method vs extension method conflict).

use std::collections::HashMap;
use std::sync::Mutex;

use kestrel_reporting::{Diagnostic, DiagnosticContext, IntoDiagnostic, Label};
use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

use kestrel_semantic_model::SemanticModel;

use crate::validation::{SymbolContext, Validator};

/// Validator that checks for conflicting methods across extensions
pub struct ExtensionConflictValidator {
    /// Collected extensions during the walk, grouped by (target_id, specificity)
    extensions_by_target: Mutex<HashMap<(SymbolId, usize), Vec<CollectedExtension>>>,
    /// Struct methods for each struct, keyed by struct ID: (method_name, method_span)
    struct_methods: Mutex<HashMap<SymbolId, Vec<(String, Span)>>>,
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
            struct_methods: Mutex::new(HashMap::new()),
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

        // Get target struct ID, struct symbol, and substitutions from the target type
        let (target_id, struct_symbol, substitutions) = match target_ty.kind() {
            kestrel_semantic_tree::ty::TyKind::Struct {
                symbol,
                substitutions,
            } => (symbol.metadata().id(), symbol.clone(), substitutions),
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

        // Collect struct methods (only once per struct)
        {
            let mut struct_methods = self.struct_methods.lock().unwrap();
            if !struct_methods.contains_key(&target_id) {
                let methods: Vec<(String, Span)> = struct_symbol
                    .metadata()
                    .children()
                    .into_iter()
                    .filter(|c| c.metadata().kind() == KestrelSymbolKind::Function)
                    .map(|c| (c.metadata().name().value.clone(), c.metadata().name().span.clone()))
                    .collect();
                struct_methods.insert(target_id, methods);
            }
        }

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

    fn finalize(&self, _model: &SemanticModel, diagnostics: &mut DiagnosticContext) {
        let extensions_by_target = self.extensions_by_target.lock().unwrap();
        let struct_methods = self.struct_methods.lock().unwrap();

        // For each (target_id, specificity) group with extensions
        for ((target_id, _specificity), extensions) in extensions_by_target.iter() {
            // Check for struct method vs extension method conflicts
            // This applies to all extensions, even if there's only one
            if let Some(struct_method_list) = struct_methods.get(target_id) {
                // Build a set of struct method names for quick lookup
                let struct_method_names: HashMap<&str, &Span> = struct_method_list
                    .iter()
                    .map(|(name, span)| (name.as_str(), span))
                    .collect();

                // Check each extension's methods against struct methods
                for ext in extensions {
                    for (method_name, ext_method_span) in &ext.methods {
                        if let Some(&struct_method_span) = struct_method_names.get(method_name.as_str()) {
                            // Conflict: extension method duplicates struct method
                            let error = StructExtensionMethodConflictError {
                                method_name: method_name.clone(),
                                struct_method_span: struct_method_span.clone(),
                                extension_method_span: ext_method_span.clone(),
                            };
                            diagnostics.add_diagnostic(error.into_diagnostic());
                        }
                    }
                }
            }

            // Check for extension-to-extension conflicts (existing logic)
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
                    diagnostics.add_diagnostic(error.into_diagnostic());
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
    fn into_diagnostic(&self) -> Diagnostic<usize> {
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
                Label::primary(0, span.range()).with_message(msg)
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

/// Error for extension method conflicting with struct method
#[derive(Debug, Clone)]
pub struct StructExtensionMethodConflictError {
    pub method_name: String,
    pub struct_method_span: Span,
    pub extension_method_span: Span,
}

impl IntoDiagnostic for StructExtensionMethodConflictError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "duplicate method '{}': extension cannot redefine struct method",
                self.method_name
            ))
            .with_labels(vec![
                Label::primary(0, self.struct_method_span.range())
                    .with_message("method defined here on struct"),
                Label::secondary(0, self.extension_method_span.range())
                    .with_message("conflicting extension method here"),
            ])
            .with_notes(vec![
                "Extensions cannot define methods that already exist on the struct".to_string(),
                "Consider renaming the extension method or removing it".to_string(),
            ])
    }
}
