//! Analyzer for duplicate symbols
//!
//! Ensures no duplicate symbols exist within a scope:
//! - No duplicate type names (struct, protocol, type alias)
//! - No duplicate member names (field, function) within a type
//!
//! Note: Function overloading (same name, different signature) is allowed
//! and handled separately by the existing binder pass.

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_model::DeclaredNamesInScope;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::{DuplicateSymbolDifferentKindError, DuplicateSymbolError};

/// Analyzer that ensures no duplicate symbols exist
pub struct DuplicateSymbolAnalyzer;

impl DuplicateSymbolAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DuplicateSymbolAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for DuplicateSymbolAnalyzer {
    fn name(&self) -> &'static str {
        "duplicate_symbol"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        let kind = symbol.metadata().kind();

        // Check for duplicate types in scopes that can contain types
        if matches!(
            kind,
            KestrelSymbolKind::Module | KestrelSymbolKind::SourceFile
        ) {
            check_duplicate_types(symbol, ctx);
        }

        // Check for duplicate members in types
        if matches!(
            kind,
            KestrelSymbolKind::Struct | KestrelSymbolKind::Protocol
        ) {
            check_duplicate_members(symbol, ctx);
        }
    }
}

/// Check for duplicate type names within a scope
fn check_duplicate_types(symbol: &Arc<dyn Symbol<KestrelLanguage>>, ctx: &mut AnalysisContext) {
    // Map from name to (first symbol, kind description)
    let mut types: HashMap<String, (kestrel_span::Span, &'static str)> = HashMap::new();
    let scope_id = symbol.metadata().id();

    for child in ctx.model.query(DeclaredNamesInScope { scope_id }) {
        // Only check type-like symbols
        let kind_desc = match child.kind {
            KestrelSymbolKind::Struct => "struct",
            KestrelSymbolKind::Protocol => "protocol",
            KestrelSymbolKind::TypeAlias => "type alias",
            _ => continue,
        };

        let name = child.name;

        if let Some((first_span, first_kind)) = types.get(&name) {
            // Duplicate found
            if kind_desc == *first_kind {
                ctx.report(DuplicateSymbolError {
                    name: name.clone(),
                    kind: kind_desc.to_string(),
                    original_span: first_span.clone(),
                    duplicate_span: child.declaration_span,
                });
            } else {
                ctx.report(DuplicateSymbolDifferentKindError {
                    name: name.clone(),
                    new_kind: kind_desc.to_string(),
                    original_kind: first_kind.to_string(),
                    original_span: first_span.clone(),
                    duplicate_span: child.declaration_span,
                });
            }
        } else {
            types.insert(name, (child.declaration_span, kind_desc));
        }
    }
}

/// Check for duplicate member names within a type (struct, protocol)
fn check_duplicate_members(symbol: &Arc<dyn Symbol<KestrelLanguage>>, ctx: &mut AnalysisContext) {
    // Map from name to (first symbol, kind description)
    // For functions, we only store the first one - signature duplicates are handled elsewhere
    let mut members: HashMap<String, (KestrelSymbolKind, kestrel_span::Span, &'static str)> =
        HashMap::new();
    let scope_id = symbol.metadata().id();

    for child in ctx.model.query(DeclaredNamesInScope { scope_id }) {
        let child_kind = child.kind;
        let kind_desc = match child_kind {
            KestrelSymbolKind::Field => "field",
            KestrelSymbolKind::Function => "function",
            _ => continue,
        };

        let name = child.name;

        if let Some((first_symbol_kind, first_span, first_kind)) = members.get(&name) {
            // For function-to-function duplicates, skip - handled by signature check
            if child_kind == KestrelSymbolKind::Function
                && *first_symbol_kind == KestrelSymbolKind::Function
            {
                continue;
            }

            // Duplicate found (field-field, field-function, or function-field)
            if kind_desc == *first_kind {
                ctx.report(DuplicateSymbolError {
                    name: name.clone(),
                    kind: kind_desc.to_string(),
                    original_span: first_span.clone(),
                    duplicate_span: child.declaration_span,
                });
            } else {
                ctx.report(DuplicateSymbolDifferentKindError {
                    name: name.clone(),
                    new_kind: kind_desc.to_string(),
                    original_kind: first_kind.to_string(),
                    original_span: first_span.clone(),
                    duplicate_span: child.declaration_span,
                });
            }
        } else {
            members.insert(name, (child_kind, child.declaration_span, kind_desc));
        }
    }
}
