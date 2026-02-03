//! Qualified name generation from semantic symbols.

use kestrel_execution_graph::{Id, QualifiedName, QualifiedNameData};
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::subscript::SubscriptBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::Symbol;
use std::sync::Arc;

use crate::context::LoweringContext;

/// Build a qualified name for a symbol by walking up the parent chain.
///
/// This generates names like:
/// - `["module", "function"]` for a top-level function
/// - `["module", "Struct", "method"]` for a method
/// - `["module", "Struct", "init"]` for an initializer
pub fn qualified_name_for_symbol(
    ctx: &mut LoweringContext,
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Id<QualifiedName> {
    let mut segments = Vec::new();
    collect_name_segments(symbol, &mut segments);
    let name_data = QualifiedNameData::new(segments);
    ctx.mir.intern_name(name_data)
}

/// Recursively collect name segments from a symbol and its parents.
fn collect_name_segments(symbol: &Arc<dyn Symbol<KestrelLanguage>>, segments: &mut Vec<String>) {
    // First, collect parent segments
    if let Some(parent) = symbol.metadata().parent() {
        collect_name_segments(&parent, segments);
    }

    // Then add this symbol's name (if it has one that should appear in the path)
    let kind = symbol.metadata().kind();

    // Skip the root symbol (named "<root>")
    let name_value = &symbol.metadata().name().value;
    if name_value == "<root>" {
        return;
    }

    match kind {
        // Skip source files - they don't contribute to the qualified name
        KestrelSymbolKind::SourceFile => {},

        // Module contributes its name
        KestrelSymbolKind::Module => {
            let name = symbol.metadata().name();
            segments.push(name.value.clone());
        },

        // Types contribute their name
        KestrelSymbolKind::Struct
        | KestrelSymbolKind::Enum
        | KestrelSymbolKind::Protocol
        | KestrelSymbolKind::TypeAlias => {
            let name = symbol.metadata().name();
            segments.push(name.value.clone());
        },

        // Extensions contribute the name of their target type
        KestrelSymbolKind::Extension => {
            // Try to get the target type symbol from the extension
            if let Ok(ext_symbol) = symbol.clone().downcast_arc::<ExtensionSymbol>() {
                if let Some(target_type) = ext_symbol.target_type() {
                    // Extract the symbol from the target type and use its name
                    match target_type.kind() {
                        TyKind::Struct {
                            symbol: target_sym, ..
                        } => {
                            let name = target_sym.metadata().name();
                            segments.push(name.value.clone());
                        },
                        TyKind::Enum {
                            symbol: target_sym, ..
                        } => {
                            let name = target_sym.metadata().name();
                            segments.push(name.value.clone());
                        },
                        TyKind::Protocol {
                            symbol: target_sym, ..
                        } => {
                            let name = target_sym.metadata().name();
                            segments.push(name.value.clone());
                        },
                        // For primitive types, use their string representation
                        TyKind::Int(_) => segments.push("Int".to_string()),
                        TyKind::Float(_) => segments.push("Float".to_string()),
                        TyKind::Bool => segments.push("Bool".to_string()),
                        TyKind::String => segments.push("String".to_string()),
                        TyKind::Unit => segments.push("Unit".to_string()),
                        // For other types, fall back to the synthetic name
                        _ => {
                            segments.push("(extension)".to_string());
                        },
                    }
                } else {
                    // No target type available, use synthetic name
                    segments.push("(extension)".to_string());
                }
            } else {
                // Couldn't downcast, use synthetic name
                segments.push("(extension)".to_string());
            }
        },

        // Functions contribute their name, with labels for overload differentiation.
        // Uses external labels when available. For required parameters without labels,
        // uses internal names to differentiate overloads like parse(string:) vs parse(string:radix:).
        // Parameters with defaults are excluded to avoid naming changes when defaults are used.
        KestrelSymbolKind::Function => {
            let name = symbol.metadata().name();
            if let Some(callable) = symbol.metadata().get_behavior::<CallableBehavior>() {
                // Collect differentiating names:
                // - External label if present
                // - Internal name only if no label AND no default (required unlabeled param)
                let param_names: Vec<&str> = callable
                    .parameters()
                    .iter()
                    .filter_map(|p| {
                        if let Some(label) = p.external_label() {
                            Some(label)
                        } else if !p.has_default() {
                            // Required parameter without label - use internal name
                            Some(p.internal_name())
                        } else {
                            // Optional parameter without label - skip it
                            None
                        }
                    })
                    .collect();

                if param_names.is_empty() {
                    segments.push(name.value.clone());
                } else {
                    segments.push(format!("{}${}", name.value, param_names.join("$")));
                }
            } else {
                segments.push(name.value.clone());
            }
        },

        KestrelSymbolKind::Initializer => {
            // Initializers include parameter labels in the name for overload differentiation
            // e.g., init(intLiteral:) becomes "init$intLiteral", init() becomes "init"
            // For unlabeled params, we use internal names (e.g., init$ptr$len$cap)
            if let Some(callable) = symbol.metadata().get_behavior::<CallableBehavior>() {
                // Use external labels if present, otherwise fall back to internal names
                let name_parts: Vec<&str> = callable
                    .parameters()
                    .iter()
                    .map(|p| p.external_label().unwrap_or_else(|| p.internal_name()))
                    .collect();

                if name_parts.is_empty() {
                    segments.push("init".to_string());
                } else {
                    segments.push(format!("init${}", name_parts.join("$")));
                }
            } else {
                segments.push("init".to_string());
            }
        },

        // Enum cases contribute their name
        KestrelSymbolKind::EnumCase => {
            let name = symbol.metadata().name();
            segments.push(name.value.clone());
        },

        // Fields contribute their name for static stored fields (needed for global variable access).
        // Instance fields don't contribute since they use getters/setters (e.g., "get:fieldName").
        KestrelSymbolKind::Field => {
            use kestrel_semantic_tree::symbol::field::FieldSymbol;

            // Check if field is explicitly static
            let explicit_static = symbol
                .as_ref()
                .downcast_ref::<FieldSymbol>()
                .map(|f| f.is_static())
                .unwrap_or(false);

            // Check if field is module-level (implicitly static)
            let is_module_level = symbol
                .metadata()
                .parent()
                .map(|p| {
                    let kind = p.metadata().kind();
                    kind == KestrelSymbolKind::Module || kind == KestrelSymbolKind::SourceFile
                })
                .unwrap_or(false);

            let is_computed = symbol
                .as_ref()
                .downcast_ref::<FieldSymbol>()
                .map(|f| f.is_computed())
                .unwrap_or(false);

            // Add field name if it's a static/module-level stored field
            if (explicit_static || is_module_level) && !is_computed {
                let name = symbol.metadata().name();
                segments.push(name.value.clone());
            }
        },

        KestrelSymbolKind::Import
        | KestrelSymbolKind::TypeParameter
        | KestrelSymbolKind::AssociatedType => {
            // These don't contribute to qualified names
        },

        KestrelSymbolKind::Deinit => {
            // Deinit blocks are named "deinit"
            segments.push("deinit".to_string());
        },

        KestrelSymbolKind::Getter | KestrelSymbolKind::Setter => {
            // Getters and setters use their synthetic name (e.g., "get:fieldName", "set:fieldName")
            // For subscript getters/setters, include parameter labels to differentiate overloads
            let name = symbol.metadata().name();
            if let Some(callable) = symbol.metadata().get_behavior::<CallableBehavior>() {
                // Get labels from non-receiver parameters (skip self)
                let labels: Vec<&str> = callable
                    .parameters()
                    .iter()
                    .filter_map(|p| p.external_label())
                    .collect();

                if labels.is_empty() {
                    segments.push(name.value.clone());
                } else {
                    segments.push(format!("{}${}", name.value, labels.join("$")));
                }
            } else {
                segments.push(name.value.clone());
            }
        },

        KestrelSymbolKind::Subscript => {
            // Subscripts include parameter labels for overload differentiation
            // e.g., subscript(index:) becomes "subscript$index", subscript(range:) becomes "subscript$range"
            if let Some(subscript_beh) = symbol.metadata().get_behavior::<SubscriptBehavior>() {
                let labels: Vec<&str> = subscript_beh
                    .parameters()
                    .iter()
                    .filter_map(|p| p.external_label())
                    .collect();

                if labels.is_empty() {
                    segments.push("subscript".to_string());
                } else {
                    segments.push(format!("subscript${}", labels.join("$")));
                }
            } else {
                segments.push("subscript".to_string());
            }
        },
    }
}

/// Build a qualified name for a struct's implicit memberwise initializer.
#[allow(dead_code)]
pub fn qualified_name_for_struct_init(
    ctx: &mut LoweringContext,
    struct_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Id<QualifiedName> {
    let mut segments = Vec::new();
    collect_name_segments(struct_symbol, &mut segments);
    segments.push("init".to_string());
    let name_data = QualifiedNameData::new(segments);
    ctx.mir.intern_name(name_data)
}

/// Build a qualified name from string parts.
#[allow(dead_code)]
pub fn qualified_name_from_parts(ctx: &mut LoweringContext, parts: &[&str]) -> Id<QualifiedName> {
    ctx.mir.intern_name_parts(parts)
}
