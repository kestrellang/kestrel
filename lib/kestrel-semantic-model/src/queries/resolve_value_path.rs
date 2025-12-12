//! ResolveValuePath query - resolve a value path to a value

use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::valued::ValueBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{ExtensionsFor, IsVisibleFrom, ResolveName, SymbolFor, VisibleChildrenByName};
use crate::query::Query;
use crate::resolution::{SymbolResolution, ValuePathResolution};

/// Resolve a value path to a value.
///
/// Handles:
/// - Variables and constants
/// - Functions (including overloads)
/// - Static methods on types (including extensions)
/// - Type parameters (for static method calls like T.create())
pub struct ResolveValuePath {
    pub path: Vec<String>,
    pub context: SymbolId,
}

impl Query for ResolveValuePath {
    type Output = ValuePathResolution;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        if self.path.is_empty() {
            return ValuePathResolution::NotFound {
                segment: String::new(),
                index: 0,
            };
        }

        // First segment: use scope-aware name resolution
        let first = &self.path[0];
        let first_resolution = model.query(ResolveName {
            name: first.clone(),
            context: self.context,
        });

        let first_symbols: Vec<_> = match first_resolution {
            SymbolResolution::Found(ids) => ids
                .iter()
                .filter_map(|id| model.query(SymbolFor { id: *id }))
                .collect(),
            SymbolResolution::Ambiguous(ids) => {
                let symbols: Vec<_> = ids
                    .iter()
                    .filter_map(|id| model.query(SymbolFor { id: *id }))
                    .collect();

                let all_functions = symbols
                    .iter()
                    .all(|s| s.metadata().kind() == KestrelSymbolKind::Function);

                if !all_functions {
                    return ValuePathResolution::Ambiguous {
                        segment: first.clone(),
                        index: 0,
                        candidates: ids,
                    };
                }
                symbols
            }
            SymbolResolution::NotFound => {
                return ValuePathResolution::NotFound {
                    segment: first.clone(),
                    index: 0,
                };
            }
        };

        if first_symbols.is_empty() {
            return ValuePathResolution::NotFound {
                segment: first.clone(),
                index: 0,
            };
        }

        // Single-segment paths
        if self.path.len() == 1 {
            return extract_value_from_symbols(&first_symbols, first, 0);
        }

        // Multi-segment paths require single resolution
        if first_symbols.len() > 1 {
            return ValuePathResolution::Ambiguous {
                segment: first.clone(),
                index: 0,
                candidates: first_symbols.iter().map(|s| s.metadata().id()).collect(),
            };
        }

        let current_symbol = first_symbols.into_iter().next().unwrap();

        // Special case: if first segment is a type parameter, return it
        // The remaining segments are member accesses that the caller should handle
        if current_symbol.metadata().kind() == KestrelSymbolKind::TypeParameter {
            return ValuePathResolution::TypeParameter {
                symbol_id: current_symbol.metadata().id(),
            };
        }

        let mut current_symbol = current_symbol;

        for (index, segment) in self.path.iter().enumerate().skip(1) {
            let mut matches = model.query(VisibleChildrenByName {
                parent: current_symbol.metadata().id(),
                name: segment.clone(),
                context: self.context,
            });

            // If no direct children match, search extensions for static methods
            // This handles cases like Point.origin() where origin is a static method in an extension
            if matches.is_empty() && current_symbol.metadata().kind() == KestrelSymbolKind::Struct {
                let current_id = current_symbol.metadata().id();
                let extensions = model.query(ExtensionsFor {
                    target_id: current_id,
                });

                // Search all extensions for static methods with the given name
                for extension in extensions {
                    for child in extension.metadata().children() {
                        if child.metadata().name().value == *segment
                            && child.metadata().kind() == KestrelSymbolKind::Function
                            && model.query(IsVisibleFrom {
                                target: child.metadata().id(),
                                context: self.context,
                            })
                        {
                            // Check if it's a static method (no receiver)
                            if let Some(callable) =
                                child.metadata().get_behavior::<CallableBehavior>()
                            {
                                if callable.is_static() {
                                    matches.push(child);
                                }
                            }
                        }
                    }
                }
            }

            // Last segment: handle overloads
            if index == self.path.len() - 1 {
                return extract_value_from_symbols(&matches, segment, index);
            }

            // Intermediate segments must resolve to single symbol
            match matches.len() {
                0 => {
                    return ValuePathResolution::NotFound {
                        segment: segment.clone(),
                        index,
                    };
                }
                1 => {
                    current_symbol = matches.into_iter().next().unwrap();
                }
                _ => {
                    return ValuePathResolution::Ambiguous {
                        segment: segment.clone(),
                        index,
                        candidates: matches.iter().map(|s| s.metadata().id()).collect(),
                    };
                }
            }
        }

        ValuePathResolution::NotFound {
            segment: self.path.last().cloned().unwrap_or_default(),
            index: self.path.len().saturating_sub(1),
        }
    }
}

/// Helper to extract value information from resolved symbols.
fn extract_value_from_symbols(
    symbols: &[Arc<dyn Symbol<KestrelLanguage>>],
    segment: &str,
    index: usize,
) -> ValuePathResolution {
    if symbols.is_empty() {
        return ValuePathResolution::NotFound {
            segment: segment.to_string(),
            index,
        };
    }

    // Check if all symbols are functions (potential overloads)
    let all_functions = symbols
        .iter()
        .all(|s| s.metadata().kind() == KestrelSymbolKind::Function);

    if all_functions && symbols.len() > 1 {
        return ValuePathResolution::Overloaded {
            candidates: symbols.iter().map(|s| s.metadata().id()).collect(),
        };
    }

    // Single symbol - try to extract value
    let symbol = &symbols[0];

    // Check for ValueBehavior
    if let Some(value_beh) = symbol.metadata().get_behavior::<ValueBehavior>() {
        return ValuePathResolution::Symbol {
            symbol_id: symbol.metadata().id(),
            ty: value_beh.ty().clone(),
        };
    }

    // Check for CallableBehavior (functions are values)
    if let Some(callable_beh) = symbol.metadata().get_behavior::<CallableBehavior>() {
        return ValuePathResolution::Symbol {
            symbol_id: symbol.metadata().id(),
            ty: callable_beh.function_type(),
        };
    }

    // Check if this is a type parameter (for static method/init calls like T.create() or T())
    if symbol.metadata().kind() == KestrelSymbolKind::TypeParameter {
        return ValuePathResolution::TypeParameter {
            symbol_id: symbol.metadata().id(),
        };
    }

    ValuePathResolution::NotAValue {
        symbol_id: symbol.metadata().id(),
    }
}
