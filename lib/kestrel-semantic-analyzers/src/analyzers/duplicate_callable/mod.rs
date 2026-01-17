//! Analyzer for duplicate callable detection.
//!
//! Detects duplicate function, initializer, and subscript signatures within scopes.
//! In Kestrel, overloading is label-based - two callables with the same name and labels
//! are duplicates regardless of parameter/return types.

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::{CallableBehavior, DuplicateKey};
use kestrel_semantic_tree::behavior::subscript::SubscriptBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_span::Span;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

pub mod diagnostics;
use diagnostics::DuplicateCallableError;

/// Analyzer that detects duplicate callable signatures.
pub struct DuplicateCallableAnalyzer;

impl DuplicateCallableAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DuplicateCallableAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for DuplicateCallableAnalyzer {
    fn name(&self) -> &'static str {
        "duplicate_callable"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        let kind = symbol.metadata().kind();

        // Scopes that can contain callables
        let is_scope = matches!(
            kind,
            KestrelSymbolKind::Module
                | KestrelSymbolKind::Struct
                | KestrelSymbolKind::SourceFile
                | KestrelSymbolKind::Protocol
                | KestrelSymbolKind::Enum
                | KestrelSymbolKind::Extension
        );

        if is_scope {
            check_duplicate_callables(symbol, ctx);
        }
    }
}

/// Check for duplicate callable signatures within a scope.
fn check_duplicate_callables(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    ctx: &mut AnalysisContext,
) {
    // Map from (name, labels) -> (first_span, kind)
    let mut seen: HashMap<DuplicateKey, (Span, &'static str)> = HashMap::new();

    for child in symbol.metadata().children() {
        let child_kind = child.metadata().kind();

        let (key, kind_name): (Option<DuplicateKey>, &'static str) = match child_kind {
            KestrelSymbolKind::Function => {
                let name = child.metadata().name().value.clone();
                let key = child
                    .metadata()
                    .get_behavior::<CallableBehavior>()
                    .map(|b| b.duplicate_key(&name));
                (key, "function")
            }
            KestrelSymbolKind::Initializer => {
                let key = child
                    .metadata()
                    .get_behavior::<CallableBehavior>()
                    .map(|b| b.duplicate_key("init"));
                (key, "initializer")
            }
            KestrelSymbolKind::Subscript => {
                let key = child
                    .metadata()
                    .get_behavior::<SubscriptBehavior>()
                    .map(|b| b.duplicate_key());
                (key, "subscript")
            }
            _ => continue,
        };

        let Some(key) = key else {
            // Behavior not yet attached (shouldn't happen after binding)
            continue;
        };

        let span = child.metadata().declaration_span().clone();

        if let Some((first_span, first_kind)) = seen.get(&key) {
            // Found a duplicate
            ctx.report(DuplicateCallableError {
                signature: key.display(),
                kind: if kind_name == *first_kind {
                    kind_name
                } else {
                    // Different kinds with same signature - shouldn't happen but report
                    kind_name
                },
                first_span: first_span.clone(),
                duplicate_span: span,
            });
        } else {
            seen.insert(key, (span, kind_name));
        }
    }
}
