use std::sync::Arc;

use kestrel_semantic_model::{ResolveTypePath, SymbolFor, TypePathResolution};
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Constraint, Ty, TyKind, WhereClause};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::diagnostics::{NotAProtocolContext, NotAProtocolError, UnresolvedTypeError};
use crate::syntax::helpers::resolve_conformance_list;
use kestrel_syntax_tree::utils::{extract_path_segments, find_child, get_node_span};

/// Binder for struct declarations
pub struct StructBinder;

impl DeclarationBinder for StructBinder {
    fn bind_declaration(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Only process struct symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Struct {
            return;
        }

        let symbol_id = symbol.metadata().id();

        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // Extract type parameters and resolve where clause bounds
        let generics_behavior = resolve_generics(syntax, &source, file_id, symbol_id, context);

        // Add GenericsBehavior
        symbol.metadata().add_behavior(generics_behavior);

        // Resolve conformances from syntax and store them
        resolve_conformance_list(
            syntax,
            &source,
            file_id,
            symbol,
            symbol_id,
            context,
            NotAProtocolContext::Conformance,
        );

        // Note: Protocol method linking happens in the ConformanceValidator
        // during the VALIDATE phase, after all children are bound
    }
}

/// Extract type parameters and resolve where clause bounds, creating a GenericsBehavior.
fn resolve_generics(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
) -> GenericsBehavior {
    // Get type parameters from the symbol's children (they were added during BUILD)
    let symbol = match ctx.model.query(SymbolFor { id: context_id }) {
        Some(s) => s,
        None => return GenericsBehavior::empty(),
    };

    let type_parameters: Vec<Arc<TypeParameterSymbol>> = symbol
        .metadata()
        .children()
        .into_iter()
        .filter_map(|child| {
            if child.metadata().kind() == KestrelSymbolKind::TypeParameter {
                child.downcast_arc::<TypeParameterSymbol>().ok()
            } else {
                None
            }
        })
        .collect();

    // Now resolve the where clause with fully resolved protocol types
    let where_clause =
        resolve_where_clause(syntax, source, file_id, context_id, ctx, &type_parameters);

    GenericsBehavior::new(type_parameters, where_clause)
}

/// Resolve where clause from syntax, returning a WhereClause with resolved protocol types.
fn resolve_where_clause(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    type_params: &[Arc<TypeParameterSymbol>],
) -> WhereClause {
    let where_clause_node = match find_child(syntax, SyntaxKind::WhereClause) {
        Some(node) => node,
        None => return WhereClause::new(),
    };

    let mut constraints = Vec::new();

    for child in where_clause_node.children() {
        if child.kind() == SyntaxKind::TypeBound {
            if let Some(constraint) =
                resolve_type_bound(&child, source, file_id, context_id, ctx, type_params)
            {
                constraints.push(constraint);
            }
        }
    }

    WhereClause::with_constraints(constraints)
}

/// Resolve a single TypeBound, resolving protocol paths to actual types.
fn resolve_type_bound(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    type_params: &[Arc<TypeParameterSymbol>],
) -> Option<Constraint> {
    // Find the Name node and extract the type parameter name and span
    let name_node = find_child(syntax, SyntaxKind::Name)?;
    let name_token = name_node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)?;

    let param_name = name_token.text().to_string();
    let text_range = name_token.text_range();
    let param_span: kestrel_span::Span =
        Span::new(file_id, (text_range.start().into())..(text_range.end().into()));

    // Look up the type parameter (may be None if undeclared)
    let param_id = type_params
        .iter()
        .find(|p| p.metadata().name().value == param_name)
        .map(|p| p.metadata().id());

    // Resolve each Path to a protocol type
    let bounds: Vec<Ty> = syntax
        .children()
        .filter(|c| c.kind() == SyntaxKind::Path)
        .map(|path_node| {
            let span = get_node_span(&path_node, file_id);
            let segments = extract_path_segments(&path_node);

            if segments.is_empty() {
                return Ty::error(span);
            }

            let bound_name = segments.join(".");

            // Resolve the path to a type
            match ctx.model.query(ResolveTypePath {
                path: segments,
                context: context_id,
            }) {
                TypePathResolution::Resolved(resolved_ty) => match resolved_ty.kind() {
                    TyKind::Protocol { .. } => resolved_ty,
                    TyKind::Struct { symbol, .. } => {
                        ctx.diagnostics.throw(NotAProtocolError {
                            span: span.clone(),
                            name: symbol.metadata().name().value.clone(),
                            context: NotAProtocolContext::Bound,
                        });
                        Ty::error(span)
                    }
                    TyKind::TypeAlias { symbol, .. } => {
                        ctx.diagnostics.throw(NotAProtocolError {
                            span: span.clone(),
                            name: symbol.metadata().name().value.clone(),
                            context: NotAProtocolContext::Bound,
                        });
                        Ty::error(span)
                    }
                    _ => {
                        ctx.diagnostics.throw(NotAProtocolError {
                            span: span.clone(),
                            name: bound_name.clone(),
                            context: NotAProtocolContext::Bound,
                        });
                        Ty::error(span)
                    }
                },
                TypePathResolution::NotFound { .. } => {
                    ctx.diagnostics.throw(UnresolvedTypeError {
                        span: span.clone(),
                        type_name: bound_name.clone(),
                    });
                    Ty::error(span)
                }
                TypePathResolution::Ambiguous { .. } | TypePathResolution::NotAType { .. } => {
                    ctx.diagnostics.throw(NotAProtocolError {
                        span: span.clone(),
                        name: bound_name.clone(),
                        context: NotAProtocolContext::Bound,
                    });
                    Ty::error(span)
                }
            }
        })
        .collect();

    if bounds.is_empty() {
        None
    } else {
        match param_id {
            Some(id) => Some(Constraint::type_bound(id, param_name, param_span, bounds)),
            None => Some(Constraint::unresolved_type_bound(
                param_name, param_span, bounds,
            )),
        }
    }
}
