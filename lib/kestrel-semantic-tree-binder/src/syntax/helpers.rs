//! Syntax node helper functions
//!
//! Utilities for extracting information from syntax nodes.

use std::sync::Arc;

use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use kestrel_syntax_tree::utils::get_node_span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::declaration_binder::BindingContext;
use crate::diagnostics::{
    MissingParentProtocolConformanceError, NotAProtocolContext, NotAProtocolError,
};

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
    use crate::diagnostics::NegativeConformanceNotAllowedError;
    use crate::resolution::type_resolver::{resolve_type_from_ty_node, TypeSyntaxContext};
    use kestrel_semantic_tree::builtins::BuiltinKind;

    let conformance_list = match find_child(syntax, SyntaxKind::ConformanceList) {
        Some(node) => node,
        None => return,
    };

    let mut resolved = Vec::new();
    let mut negative_resolved = Vec::new();

    for item in conformance_list.children() {
        if item.kind() != SyntaxKind::ConformanceItem {
            continue;
        }

        // Check if this is a negative conformance (has NegativeConformance child)
        let is_negative = find_child(&item, SyntaxKind::NegativeConformance).is_some();

        // Find the Ty node - it might be directly under ConformanceItem or under NegativeConformance
        let ty_node = if is_negative {
            let neg_node = find_child(&item, SyntaxKind::NegativeConformance).unwrap();
            find_child(&neg_node, SyntaxKind::Ty)
        } else {
            find_child(&item, SyntaxKind::Ty)
        };

        let ty_node = match ty_node {
            Some(node) => node,
            None => continue,
        };

        let span = get_node_span(&ty_node, file_id);
        let item_span = get_node_span(&item, file_id);

        // Use full type resolution (handles type arguments like Add[MyInt])
        let mut type_ctx =
            TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
        let resolved_ty = resolve_type_from_ty_node(&ty_node, &mut type_ctx);

        // Validate that it's a protocol
        match resolved_ty.kind() {
            TyKind::Protocol {
                symbol: protocol_sym,
                ..
            } => {
                if is_negative {
                    // Validate that this protocol allows negation
                    let protocol_id = protocol_sym.metadata().id();
                    let allows_negation = ctx
                        .model
                        .builtin_registry()
                        .protocol_feature(protocol_id)
                        .map(|feature| {
                            let def = feature.definition();
                            matches!(
                                def.kind,
                                BuiltinKind::Protocol {
                                    implicit_conformance: true,
                                    ..
                                }
                            )
                        })
                        .unwrap_or(false);

                    if !allows_negation {
                        ctx.diagnostics.throw(NegativeConformanceNotAllowedError {
                            span: item_span,
                            protocol_name: protocol_sym.metadata().name().value.clone(),
                        });
                    } else {
                        negative_resolved.push(resolved_ty);
                    }
                } else {
                    resolved.push(resolved_ty);
                }
            }
            TyKind::Struct {
                symbol: struct_sym, ..
            } => {
                ctx.diagnostics.throw(NotAProtocolError {
                    span: span.clone(),
                    name: struct_sym.metadata().name().value.clone(),
                    context: error_context,
                });
                if !is_negative {
                    resolved.push(Ty::error(span));
                }
            }
            TyKind::Error => {
                // Error already reported by type resolver
                if !is_negative {
                    resolved.push(resolved_ty);
                }
            }
            _ => {
                let type_name = format!("{:?}", resolved_ty.kind());
                ctx.diagnostics.throw(NotAProtocolError {
                    span: span.clone(),
                    name: type_name,
                    context: error_context,
                });
                if !is_negative {
                    resolved.push(Ty::error(span));
                }
            }
        }
    }

    // Validate that all parent protocols are also declared
    validate_parent_protocol_conformances(&resolved, symbol, ctx);

    // Validate no conflicting conformances (e.g., Cloneable + not Copyable)
    validate_no_conflicting_conformances(&resolved, &negative_resolved, symbol, ctx);

    let conformances_behavior = ConformancesBehavior::with_negatives(resolved, negative_resolved);
    symbol.metadata().add_behavior(conformances_behavior);
}

/// Validate that a type doesn't have conflicting conformances.
///
/// Specifically, this checks if a type conforms to a protocol that refines Copyable
/// (like Cloneable) while also opting out with `not Copyable`.
fn validate_no_conflicting_conformances(
    conformances: &[Ty],
    negative_conformances: &[Ty],
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    ctx: &mut BindingContext,
) {
    use crate::diagnostics::ConflictingCopyableConformanceError;
    use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;

    // Only validate structs and enums, not protocols
    let kind = symbol.metadata().kind();
    if kind != KestrelSymbolKind::Struct && kind != KestrelSymbolKind::Enum {
        return;
    }

    // Check if there's a `not Copyable` in the negative conformances
    let copyable_id = ctx.model.builtin_registry().copyable_protocol();
    let has_not_copyable = copyable_id.map_or(false, |copyable_id| {
        negative_conformances.iter().any(|ty| {
            if let TyKind::Protocol { symbol, .. } = ty.kind() {
                symbol.metadata().id() == copyable_id
            } else {
                false
            }
        })
    });

    if !has_not_copyable {
        return;
    }

    // Check if any positive conformance is to a protocol that refines Copyable
    // For now, we directly check for Cloneable (which is defined to refine Copyable)
    let cloneable_id = ctx.model.builtin_registry().cloneable_protocol();

    for conformance in conformances {
        if let TyKind::Protocol {
            symbol: protocol_symbol,
            ..
        } = conformance.kind()
        {
            let protocol_id = protocol_symbol.metadata().id();

            // Check if this is Cloneable
            if cloneable_id == Some(protocol_id) {
                ctx.diagnostics.throw(ConflictingCopyableConformanceError {
                    span: symbol.metadata().span().clone(),
                    refining_protocol: protocol_symbol.metadata().name().value.clone(),
                });
                return; // Only report once
            }

            // Also check if this protocol inherits from Copyable (transitively)
            // by checking its parent protocols
            if let Some(parent_conformances) = protocol_symbol
                .metadata()
                .get_behavior::<ConformancesBehavior>()
            {
                if let Some(copyable_id) = copyable_id {
                    let inherits_copyable =
                        parent_conformances.conformances().iter().any(|parent| {
                            if let TyKind::Protocol {
                                symbol: parent_sym, ..
                            } = parent.kind()
                            {
                                parent_sym.metadata().id() == copyable_id
                            } else {
                                false
                            }
                        });

                    if inherits_copyable {
                        ctx.diagnostics.throw(ConflictingCopyableConformanceError {
                            span: symbol.metadata().span().clone(),
                            refining_protocol: protocol_symbol.metadata().name().value.clone(),
                        });
                        return; // Only report once
                    }
                }
            }
        }
    }
}

/// Validate that if a struct conforms to protocol B which inherits from A,
/// it must also explicitly declare conformance to A.
///
/// This only applies to structs, not to protocols (protocol inheritance is different).
///
/// Exception: If the parent protocol has implicit conformance (like Copyable),
/// we don't require explicit conformance since all types implicitly conform.
fn validate_parent_protocol_conformances(
    conformances: &[Ty],
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    ctx: &mut BindingContext,
) {
    use kestrel_semantic_tree::builtins::LanguageFeature;
    use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;

    // Only validate structs, not protocols
    // Protocol inheritance (protocol B: A) is different from struct conformance (struct S: A, B)
    if symbol.metadata().kind() != KestrelSymbolKind::Struct {
        return;
    }

    // Collect all directly declared protocol IDs for quick lookup
    let declared_protocol_ids: std::collections::HashSet<_> = conformances
        .iter()
        .filter_map(|ty| {
            if let TyKind::Protocol { symbol, .. } = ty.kind() {
                Some(symbol.metadata().id())
            } else {
                None
            }
        })
        .collect();

    // For each declared conformance, check its parent protocols
    for conformance in conformances {
        if let TyKind::Protocol {
            symbol: protocol_symbol,
            ..
        } = conformance.kind()
        {
            // Get the protocol's own conformances (parent protocols)
            if let Some(parent_conformances) = protocol_symbol
                .metadata()
                .get_behavior::<ConformancesBehavior>()
            {
                for parent in parent_conformances.conformances() {
                    if let TyKind::Protocol {
                        symbol: parent_protocol,
                        ..
                    } = parent.kind()
                    {
                        let parent_id = parent_protocol.metadata().id();

                        // Skip if parent protocol has implicit conformance (like Copyable)
                        // All types implicitly conform to these unless opted out
                        if let Some(feature) =
                            ctx.model.builtin_registry().protocol_feature(parent_id)
                        {
                            if let kestrel_semantic_tree::builtins::BuiltinKind::Protocol {
                                implicit_conformance: true,
                                ..
                            } = feature.definition().kind
                            {
                                continue;
                            }
                        }

                        // Check if the parent protocol is in our declared conformances
                        if !declared_protocol_ids.contains(&parent_id) {
                            let child_name = protocol_symbol.metadata().name().value.clone();
                            let parent_name = parent_protocol.metadata().name().value.clone();
                            let struct_name = symbol.metadata().name().value.clone();

                            ctx.diagnostics
                                .throw(MissingParentProtocolConformanceError {
                                    span: symbol.metadata().span().clone(),
                                    struct_name,
                                    child_protocol: child_name,
                                    parent_protocol: parent_name,
                                });
                        }
                    }
                }
            }
        }
    }
}
