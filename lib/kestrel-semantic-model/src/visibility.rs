//! Visibility checking for symbols
//!
//! This module provides visibility checking utilities for determining whether
//! symbols are visible from a given context based on visibility modifiers.

use std::sync::Arc;

use kestrel_semantic_tree::behavior::visibility::Visibility;
use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;

/// Find the ancestor of the given kind for a symbol.
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

/// Check if a target symbol is visible from a context symbol.
pub fn is_visible_from(
    target: &Arc<dyn Symbol<KestrelLanguage>>,
    context: &Arc<dyn Symbol<KestrelLanguage>>,
) -> bool {
    let Some(visibility_behavior) = target.visibility_behavior() else {
        // No visibility behavior means default (internal), which is always visible
        return true;
    };

    match visibility_behavior.visibility() {
        Some(Visibility::Public) => true,
        Some(Visibility::Private) => {
            let visibility_scope = visibility_behavior.visibility_scope();
            Arc::ptr_eq(context, visibility_scope) || is_ancestor(visibility_scope, context)
        }
        Some(Visibility::Internal) => {
            let target_module = find_ancestor_of_kind(target, KestrelSymbolKind::Module);
            let context_module = find_ancestor_of_kind(context, KestrelSymbolKind::Module);

            match (target_module, context_module) {
                (Some(t), Some(c)) => Arc::ptr_eq(&t, &c),
                _ => true, // If we can't determine modules, default to visible
            }
        }
        Some(Visibility::Fileprivate) => {
            let visibility_scope = visibility_behavior.visibility_scope();
            Arc::ptr_eq(context, visibility_scope) || is_ancestor(visibility_scope, context)
        }
        None => true, // Default visibility (internal) - visible everywhere
    }
}

/// Check if potential_ancestor is an ancestor of the given symbol.
fn is_ancestor(
    potential_ancestor: &Arc<dyn Symbol<KestrelLanguage>>,
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> bool {
    let mut current = Some(symbol.clone());

    while let Some(s) = current {
        if Arc::ptr_eq(&s, potential_ancestor) {
            return true;
        }
        current = s.metadata().parent();
    }

    false
}

/// Find children of parent that are visible from context and match name.
pub fn find_visible_children_by_name(
    parent: &Arc<dyn Symbol<KestrelLanguage>>,
    name: &str,
    context: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Vec<Arc<dyn Symbol<KestrelLanguage>>> {
    parent
        .metadata()
        .visible_children()
        .into_iter()
        .filter(|c| c.metadata().name().value == name)
        .filter(|c| is_visible_from(c, context))
        .collect()
}
