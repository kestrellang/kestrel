//! Qualified name generation from semantic symbols.

use kestrel_execution_graph::{Id, QualifiedName, QualifiedNameData};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
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
        | KestrelSymbolKind::TypeAlias
        | KestrelSymbolKind::Extension => {
            let name = symbol.metadata().name();
            segments.push(name.value.clone());
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

        KestrelSymbolKind::Import | KestrelSymbolKind::TypeParameter | KestrelSymbolKind::AssociatedType => {
            // These don't contribute to qualified names
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
