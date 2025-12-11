//! Validator for duplicate symbols
//!
//! Ensures no duplicate symbols exist within a scope:
//! - No duplicate type names (struct, protocol, type alias)
//! - No duplicate member names (field, function) within a type
//!
//! Note: Function overloading (same name, different signature) is allowed
//! and handled separately by the existing `check_duplicate_signatures`.

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;

use crate::diagnostics::{DuplicateSymbolDifferentKindError, DuplicateSymbolError};
use crate::syntax::get_file_id_for_symbol;
use crate::validation::{SymbolContext, Validator};

/// Validator that ensures no duplicate symbols exist
pub struct DuplicateSymbolValidator;

impl DuplicateSymbolValidator {
    const NAME: &'static str = "duplicate_symbol";

    pub fn new() -> Self {
        Self
    }
}

impl Default for DuplicateSymbolValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for DuplicateSymbolValidator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn validate_symbol(&self, ctx: &SymbolContext<'_>) {
        let kind = ctx.symbol.metadata().kind();

        // Check for duplicate types in scopes that can contain types
        if matches!(
            kind,
            KestrelSymbolKind::Module | KestrelSymbolKind::SourceFile
        ) {
            check_duplicate_types(ctx);
        }

        // Check for duplicate members in types
        if matches!(
            kind,
            KestrelSymbolKind::Struct | KestrelSymbolKind::Protocol
        ) {
            check_duplicate_members(ctx);
        }
    }
}

/// Check for duplicate type names within a scope
fn check_duplicate_types(ctx: &SymbolContext<'_>) {
    // Map from name to (first symbol, kind description)
    let mut types: HashMap<String, (Arc<dyn Symbol<KestrelLanguage>>, &'static str)> =
        HashMap::new();

    for child in ctx.symbol.metadata().children() {
        let child_kind = child.metadata().kind();

        // Only check type-like symbols
        let kind_desc = match child_kind {
            KestrelSymbolKind::Struct => "struct",
            KestrelSymbolKind::Protocol => "protocol",
            KestrelSymbolKind::TypeAlias => "type alias",
            _ => continue,
        };

        let name = child.metadata().name().value.clone();

        if let Some((first, first_kind)) = types.get(&name) {
            // Duplicate found
            let file_id = get_file_id_for_symbol(&child, &mut *ctx.diagnostics().get());
            let first_file_id = get_file_id_for_symbol(first, &mut *ctx.diagnostics().get());

            if kind_desc == *first_kind {
                ctx.diagnostics().get().throw(
                    DuplicateSymbolError {
                        name: name.clone(),
                        kind: kind_desc.to_string(),
                        original_span: first.metadata().declaration_span().clone(),
                        original_file_id: first_file_id,
                        duplicate_span: child.metadata().declaration_span().clone(),
                    });
            } else {
                ctx.diagnostics().get().throw(
                    DuplicateSymbolDifferentKindError {
                        name: name.clone(),
                        new_kind: kind_desc.to_string(),
                        original_kind: first_kind.to_string(),
                        original_span: first.metadata().declaration_span().clone(),
                        original_file_id: first_file_id,
                        duplicate_span: child.metadata().declaration_span().clone(),
                    });
            }
        } else {
            types.insert(name, (child.clone(), kind_desc));
        }
    }
}

/// Check for duplicate member names within a type (struct, protocol)
fn check_duplicate_members(ctx: &SymbolContext<'_>) {
    // Map from name to (first symbol, kind description)
    // For functions, we only store the first one - signature duplicates are handled elsewhere
    let mut members: HashMap<String, (Arc<dyn Symbol<KestrelLanguage>>, &'static str)> =
        HashMap::new();

    for child in ctx.symbol.metadata().children() {
        let child_kind = child.metadata().kind();

        let kind_desc = match child_kind {
            KestrelSymbolKind::Field => "field",
            KestrelSymbolKind::Function => "function",
            _ => continue,
        };

        let name = child.metadata().name().value.clone();

        if let Some((first, first_kind)) = members.get(&name) {
            // For function-to-function duplicates, skip - handled by signature check
            if child_kind == KestrelSymbolKind::Function
                && first.metadata().kind() == KestrelSymbolKind::Function
            {
                continue;
            }

            // Duplicate found (field-field, field-function, or function-field)
            let file_id = get_file_id_for_symbol(&child, &mut *ctx.diagnostics().get());
            let first_file_id = get_file_id_for_symbol(first, &mut *ctx.diagnostics().get());

            if kind_desc == *first_kind {
                ctx.diagnostics().get().throw(
                    DuplicateSymbolError {
                        name: name.clone(),
                        kind: kind_desc.to_string(),
                        original_span: first.metadata().declaration_span().clone(),
                        original_file_id: first_file_id,
                        duplicate_span: child.metadata().declaration_span().clone(),
                    });
            } else {
                ctx.diagnostics().get().throw(
                    DuplicateSymbolDifferentKindError {
                        name: name.clone(),
                        new_kind: kind_desc.to_string(),
                        original_kind: first_kind.to_string(),
                        original_span: first.metadata().declaration_span().clone(),
                        original_file_id: first_file_id,
                        duplicate_span: child.metadata().declaration_span().clone(),
                    });
            }
        } else {
            members.insert(name, (child.clone(), kind_desc));
        }
    }
}
