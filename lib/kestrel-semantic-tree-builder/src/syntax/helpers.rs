//! Syntax node helper functions
//!
//! Utilities for extracting information from syntax nodes.

use std::sync::Arc;

use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::visibility::Visibility;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::diagnostics::{NotAProtocolContext, NotAProtocolError};
use crate::resolver::BindingContext;

/// Find a child node with the specified kind
pub fn find_child(syntax: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    kestrel_syntax_tree::utils::find_child(syntax, kind)
}

/// Extract name from a Name node
pub fn extract_name(syntax: &SyntaxNode) -> Option<String> {
    kestrel_syntax_tree::utils::extract_name(syntax)
}

/// Extract visibility modifier from a node with a Visibility child
pub fn extract_visibility(syntax: &SyntaxNode) -> Option<String> {
    kestrel_syntax_tree::utils::extract_visibility(syntax)
}

/// Get the span of a syntax node, excluding leading trivia
pub fn get_node_span(node: &SyntaxNode, _source: &str) -> Span {
    kestrel_syntax_tree::utils::get_node_span(node, _source)
}

/// Get the span of the visibility node
pub fn get_visibility_span(syntax: &SyntaxNode, source: &str) -> Option<Span> {
    kestrel_syntax_tree::utils::get_visibility_span(syntax, source)
}

/// Find an ancestor symbol of the specified kind
pub fn find_ancestor_of_kind(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    kind: KestrelSymbolKind,
) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
    let mut current = Some(symbol.clone());

    while let Some(s) = current {
        if s.metadata().kind() == kind {
            return Some(s);
        }
        current = s.metadata().parent();
    }

    None
}

/// Information about a symbol's source file
pub struct SourceFileInfo {
    pub file_id: usize,
    pub file_name: String,
}

/// Get source file info for a symbol by walking up to its SourceFile parent
pub fn get_source_file_info(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    diagnostics: &DiagnosticContext,
) -> Option<SourceFileInfo> {
    let mut current = symbol.clone();
    loop {
        if current.metadata().kind() == KestrelSymbolKind::SourceFile {
            let file_name = current.metadata().name().value.clone();
            let file_id = diagnostics.get_file_id(&file_name)?;
            return Some(SourceFileInfo { file_id, file_name });
        }
        match current.metadata().parent() {
            Some(parent) => current = parent,
            None => return None,
        }
    }
}

/// Get the file_id for a symbol by walking up to its SourceFile parent
pub fn get_file_id_for_symbol(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    diagnostics: &DiagnosticContext,
) -> usize {
    get_source_file_info(symbol, diagnostics)
        .map(|info| info.file_id)
        .unwrap_or(0)
}

/// Extract path segments from a Path syntax node
pub fn extract_path_segments(path_node: &SyntaxNode) -> Vec<String> {
    kestrel_syntax_tree::utils::extract_path_segments(path_node)
}

/// Extract identifier text from a Name syntax node
pub fn extract_identifier_from_name(name_node: &SyntaxNode) -> Option<String> {
    kestrel_syntax_tree::utils::extract_identifier_from_name(name_node)
}

/// Resolve conformances/inheritance from syntax and add as ConformancesBehavior
pub fn resolve_conformance_list(
    syntax: &SyntaxNode,
    source: &str,
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    context_id: SymbolId,
    ctx: &mut BindingContext,
    file_id: usize,
    error_context: NotAProtocolContext,
) {
    use crate::resolution::type_resolver::{TypeSyntaxContext, resolve_type_from_ty_node};

    let conformance_list = match find_child(syntax, SyntaxKind::ConformanceList) {
        Some(node) => node,
        None => return,
    };

    let mut resolved = Vec::new();

    for item in conformance_list.children() {
        if item.kind() != SyntaxKind::ConformanceItem {
            continue;
        }

        let ty_node = match find_child(&item, SyntaxKind::Ty) {
            Some(node) => node,
            None => continue,
        };

        let span = get_node_span(&ty_node, source);

        // Use full type resolution (handles type arguments like Add[MyInt])
        let mut type_ctx =
            TypeSyntaxContext::new(ctx.model, ctx.diagnostics, file_id, source, context_id);
        let resolved_ty = resolve_type_from_ty_node(&ty_node, &mut type_ctx);

        // Validate that it's a protocol
        match resolved_ty.kind() {
            TyKind::Protocol { .. } => {
                resolved.push(resolved_ty);
            }
            TyKind::Struct {
                symbol: struct_sym, ..
            } => {
                ctx.diagnostics.throw(NotAProtocolError {
                    span: span.clone(),
                    name: struct_sym.metadata().name().value.clone(),
                    context: error_context,
                });
                resolved.push(Ty::error(span));
            }
            TyKind::Error => {
                // Error already reported by type resolver
                resolved.push(resolved_ty);
            }
            _ => {
                let type_name = format!("{:?}", resolved_ty.kind());
                ctx.diagnostics.throw(NotAProtocolError {
                    span: span.clone(),
                    name: type_name,
                    context: error_context,
                });
                resolved.push(Ty::error(span));
            }
        }
    }

    let conformances_behavior = ConformancesBehavior::new(resolved);
    symbol.metadata().add_behavior(conformances_behavior);
}
