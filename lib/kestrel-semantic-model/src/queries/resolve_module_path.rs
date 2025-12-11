//! ResolveModulePath query - resolve a module path from a context

use kestrel_semantic_tree::error::ModuleNotFoundError;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_span::Span;
use semantic_tree::symbol::SymbolId;

use crate::query::Query;
use crate::SemanticModel;

/// Resolve a module path from a context.
///
/// Takes a path like `["A", "B", "C"]` and resolves it to a module symbol.
/// The first segment is looked up by kind+name index, subsequent segments
/// are looked up as visible children.
pub struct ResolveModulePath {
    pub path: Vec<String>,
    pub context: SymbolId,
}

impl Query for ResolveModulePath {
    type Output = Result<SymbolId, ModuleNotFoundError>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        if self.path.is_empty() {
            return Err(ModuleNotFoundError {
                path: vec![],
                failed_segment_index: 0,
                path_span: Span::from(0..0),
                failed_segment_span: Span::from(0..0),
            });
        }

        // Find first segment using O(1) index lookup
        let first_segment = &self.path[0];
        let modules = model
            .registry()
            .find_by_kind_and_name(KestrelSymbolKind::Module, first_segment);

        let mut current = match modules.into_iter().next() {
            Some(s) => s,
            None => {
                return Err(ModuleNotFoundError {
                    path: self.path.clone(),
                    failed_segment_index: 0,
                    path_span: Span::from(0..0),
                    failed_segment_span: Span::from(0..0),
                });
            }
        };

        // Resolve remaining segments
        for (index, segment) in self.path.iter().enumerate().skip(1) {
            let found = current
                .metadata()
                .visible_children()
                .into_iter()
                .find(|child| child.metadata().name().value == *segment);

            match found {
                Some(child) => current = child,
                None => {
                    return Err(ModuleNotFoundError {
                        path: self.path.clone(),
                        failed_segment_index: index,
                        path_span: Span::from(0..0),
                        failed_segment_span: Span::from(0..0),
                    });
                }
            }
        }

        Ok(current.metadata().id())
    }
}
