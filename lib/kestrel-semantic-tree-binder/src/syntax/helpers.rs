//! Syntax node helper functions
//!
//! Utilities for extracting information from syntax nodes.

use std::sync::Arc;

use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use kestrel_syntax_tree::utils::get_node_span;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::declaration_binder::BindingContext;
use crate::diagnostics::{NotAProtocolContext, NotAProtocolError};

/// Find a child node with the specified kind
pub fn find_child(syntax: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    kestrel_syntax_tree::utils::find_child(syntax, kind)
}

/// Resolve conformances/inheritance from syntax and add as ConformancesBehavior
pub fn resolve_conformance_list(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    context_id: SymbolId,
    ctx: &mut BindingContext,
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

        let span = get_node_span(&ty_node, file_id);

        // Use full type resolution (handles type arguments like Add[MyInt])
        let mut type_ctx =
            TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
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
