//! Analyzer for duplicate callable detection.
//!
//! Detects duplicate function, initializer, and subscript signatures within scopes.
//! In Kestrel, overloading is label-based - two callables with the same name and labels
//! are duplicates regardless of parameter/return types.
//!
//! **Exception**: Callables that implement different protocol requirements are NOT duplicates.
//! For example, if a struct conforms to `Convertible[Int]` and `Convertible[String]`, it can
//! have both `init(from: Int)` and `init(from: String)` because they implement different
//! protocol requirements.

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::{CallableBehavior, DuplicateKey, SignatureType};
use kestrel_semantic_tree::behavior::implements::ImplementsBehavior;
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
///
/// This analyzer runs duplicate checking in `finalize` rather than `visit_symbol`
/// to ensure that `ImplementsBehavior` (attached by `ConformanceAnalyzer::finalize`)
/// is available when we check for duplicates.
pub struct DuplicateCallableAnalyzer {
    /// Scopes collected during the walk phase to check in finalize
    scopes: Vec<Arc<dyn Symbol<KestrelLanguage>>>,
}

impl DuplicateCallableAnalyzer {
    pub fn new() -> Self {
        Self { scopes: Vec::new() }
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
        _ctx: &mut AnalysisContext,
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
            // Collect scopes to check in finalize, after ImplementsBehavior is attached
            self.scopes.push(symbol.clone());
        }
    }

    fn finalize(&mut self, ctx: &mut AnalysisContext) {
        // Check duplicates in finalize, after ConformanceAnalyzer has attached ImplementsBehavior
        for scope in &self.scopes {
            check_duplicate_callables(scope, ctx);
        }
    }
}

/// Information about a callable for duplicate checking.
struct CallableInfo {
    span: Span,
    kind_name: &'static str,
    /// The conformance signature this callable implements (if any).
    /// Two callables with the same key but different conformance signatures are NOT duplicates.
    /// For example, `Conv[Int8]` and `Conv[Int32]` are different conformances.
    conformance_signature: Option<SignatureType>,
}

/// Check for duplicate callable signatures within a scope.
fn check_duplicate_callables(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    ctx: &mut AnalysisContext,
) {
    // Map from (name, labels) -> list of callables with that key
    let mut seen: HashMap<DuplicateKey, Vec<CallableInfo>> = HashMap::new();

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

        // Get the conformance signature this callable implements (if any)
        let conformance_signature = child
            .metadata()
            .get_behavior::<ImplementsBehavior>()
            .map(|b| b.conformance_signature().clone());

        let info = CallableInfo {
            span,
            kind_name,
            conformance_signature,
        };

        seen.entry(key).or_default().push(info);
    }

    // Check each group for duplicates
    for (key, callables) in seen {
        if callables.len() < 2 {
            continue;
        }

        // Find callables that are true duplicates (same conformance signature or both None)
        // We need to find any pair that conflicts
        for i in 0..callables.len() {
            for j in (i + 1)..callables.len() {
                let a = &callables[i];
                let b = &callables[j];

                // Two callables are duplicates if:
                // 1. Neither implements a protocol requirement (both None), OR
                // 2. They implement the SAME conformance (same signature, e.g., both implement Conv[Int8])
                let is_duplicate = match (&a.conformance_signature, &b.conformance_signature) {
                    (None, None) => true,
                    (Some(sig_a), Some(sig_b)) => sig_a == sig_b,
                    _ => false, // One implements protocol, other doesn't - not duplicates
                };

                if is_duplicate {
                    ctx.report(DuplicateCallableError {
                        signature: key.display(),
                        kind: a.kind_name,
                        first_span: a.span.clone(),
                        duplicate_span: b.span.clone(),
                    });
                }
            }
        }
    }
}
