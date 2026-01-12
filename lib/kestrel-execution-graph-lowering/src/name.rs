//! Qualified name generation from semantic symbols.

use kestrel_execution_graph::{Id, QualifiedName, QualifiedNameData};
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
        KestrelSymbolKind::SourceFile => {}

        // Module contributes its name
        KestrelSymbolKind::Module => {
            let name = symbol.metadata().name();
            segments.push(name.value.clone());
        }

        // Types contribute their name
        KestrelSymbolKind::Struct
        | KestrelSymbolKind::Enum
        | KestrelSymbolKind::Protocol
        | KestrelSymbolKind::TypeAlias => {
            let name = symbol.metadata().name();
            segments.push(name.value.clone());
        }

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
                        }
                        TyKind::Enum {
                            symbol: target_sym, ..
                        } => {
                            let name = target_sym.metadata().name();
                            segments.push(name.value.clone());
                        }
                        TyKind::Protocol {
                            symbol: target_sym, ..
                        } => {
                            let name = target_sym.metadata().name();
                            segments.push(name.value.clone());
                        }
                        // For primitive types, use their string representation
                        TyKind::Int(_) => segments.push("Int".to_string()),
                        TyKind::Float(_) => segments.push("Float".to_string()),
                        TyKind::Bool => segments.push("Bool".to_string()),
                        TyKind::String => segments.push("String".to_string()),
                        TyKind::Unit => segments.push("Unit".to_string()),
                        // For other types, fall back to the synthetic name
                        _ => {
                            segments.push("(extension)".to_string());
                        }
                    }
                } else {
                    // No target type available, use synthetic name
                    segments.push("(extension)".to_string());
                }
            } else {
                // Couldn't downcast, use synthetic name
                segments.push("(extension)".to_string());
            }
        }

        // Functions and initializers contribute their name
        KestrelSymbolKind::Function => {
            let name = symbol.metadata().name();
            segments.push(name.value.clone());
        }

        KestrelSymbolKind::Initializer => {
            // Initializers are named "init"
            segments.push("init".to_string());
        }

        // Enum cases contribute their name
        KestrelSymbolKind::EnumCase => {
            let name = symbol.metadata().name();
            segments.push(name.value.clone());
        }

        // Fields, imports, type parameters don't typically form part of qualified names
        // for items, but we include them for completeness
        KestrelSymbolKind::Field => {
            let name = symbol.metadata().name();
            segments.push(name.value.clone());
        }

        KestrelSymbolKind::Import
        | KestrelSymbolKind::TypeParameter
        | KestrelSymbolKind::AssociatedType => {
            // These don't contribute to qualified names
        }

        KestrelSymbolKind::Deinit => {
            // Deinit blocks are named "deinit"
            segments.push("deinit".to_string());
        }

        KestrelSymbolKind::Getter | KestrelSymbolKind::Setter => {
            // Getters and setters use their synthetic name (e.g., "get:fieldName", "set:fieldName")
            let name = symbol.metadata().name();
            segments.push(name.value.clone());
        }

        KestrelSymbolKind::Subscript => {
            // Subscripts use the synthetic name "subscript" since they're identified by signature
            segments.push("subscript".to_string());
        }
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
