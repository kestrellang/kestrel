//! Qualified name generation from semantic symbols.

use kestrel_execution_graph::{Id, QualifiedName, QualifiedNameData};
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::subscript::SubscriptBehavior;
use kestrel_semantic_tree::behavior::{ComputedPropertyMarker, NamespaceScopeMarker, StaticBehavior};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::Symbol;
use std::sync::Arc;

use crate::context::LoweringContext;

/// Mangle a type into a string suitable for use in qualified names.
/// This is used to disambiguate overloaded initializers with the same labels
/// but different parameter types (e.g., init(from: Int8) vs init(from: UInt32)).
pub fn mangle_type_name(ty: &Ty) -> String {
    use kestrel_semantic_tree::ty::{FloatBits, IntBits};

    match ty.kind() {
        TyKind::Struct { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::Enum { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::Protocol { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::Int(bits) => match bits {
            IntBits::I8 => "I8".to_string(),
            IntBits::I16 => "I16".to_string(),
            IntBits::I32 => "I32".to_string(),
            IntBits::I64 => "I64".to_string(),
        },
        TyKind::Float(bits) => match bits {
            FloatBits::F16 => "F16".to_string(),
            FloatBits::F32 => "F32".to_string(),
            FloatBits::F64 => "F64".to_string(),
        },
        TyKind::Bool => "Bool".to_string(),
        TyKind::String => "String".to_string(),
        TyKind::Unit => "Unit".to_string(),
        TyKind::Never => "Never".to_string(),
        TyKind::Pointer(inner) => format!("Ptr{}", mangle_type_name(inner)),
        TyKind::TypeParameter(tp) => tp.metadata().name().value.clone(),
        TyKind::SelfType => "Self".to_string(),
        TyKind::Function { .. } => "Fn".to_string(),
        TyKind::Tuple(elements) => {
            let parts: Vec<String> = elements.iter().map(mangle_type_name).collect();
            format!("Tuple{}", parts.join("_"))
        },
        TyKind::TypeAlias { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::Error => "Error".to_string(),
        TyKind::Infer => "Infer".to_string(),
        TyKind::AssociatedType { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::UnresolvedFunction { .. } => "UnresolvedFn".to_string(),
        TyKind::UnresolvedPath { segments } => segments.join("_"),
    }
}

/// Generate the "init" portion of an initializer's qualified name from its callable behavior.
///
/// For most initializers, this uses just labels (e.g., "init$intLiteral").
/// For initializers with "from" label (from Convertible protocol), types are included
/// to disambiguate overloads (e.g., "init$from_UInt32").
///
/// For example:
/// - `init(intLiteral:)` becomes "init$intLiteral"
/// - `init(from: UInt32)` becomes "init$from_UInt32"
/// - `init()` becomes "init"
pub fn init_name_suffix_from_callable(callable: &CallableBehavior) -> String {
    let params = callable.parameters();

    // Check if any parameter has "from" label - this indicates Convertible protocol
    // which has multiple overloads with same label but different types
    let has_from_label = params.iter().any(|p| p.external_label() == Some("from"));

    let name_parts: Vec<String> = params
        .iter()
        .map(|p| {
            let label = p.external_label().unwrap_or_else(|| p.internal_name());
            if has_from_label {
                // Include type for disambiguation
                let type_name = mangle_type_name(&p.ty);
                format!("{}_{}", label, type_name)
            } else {
                // Just use label
                label.to_string()
            }
        })
        .collect();

    if name_parts.is_empty() {
        "init".to_string()
    } else {
        format!("init${}", name_parts.join("$"))
    }
}

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
            // Initializers include parameter labels in the name for overload differentiation.
            // For "from:" initializers (Convertible protocol), types are also included
            // because multiple conformances create overloads with the same label.
            if let Some(callable) = symbol.metadata().get_behavior::<CallableBehavior>() {
                segments.push(init_name_suffix_from_callable(&callable));
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
            // Check if field is explicitly static
            let explicit_static = symbol
                .metadata()
                .get_behavior::<StaticBehavior>()
                .is_some();

            // Check if field is module-level (implicitly static)
            let is_module_level = symbol
                .metadata()
                .parent()
                .map(|p| p.metadata().get_behavior::<NamespaceScopeMarker>().is_some())
                .unwrap_or(false);

            let is_computed = symbol
                .metadata()
                .get_behavior::<ComputedPropertyMarker>()
                .is_some();

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
