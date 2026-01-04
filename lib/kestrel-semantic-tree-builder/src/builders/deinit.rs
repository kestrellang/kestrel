use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::deinit::DeinitSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use kestrel_syntax_tree::utils::get_node_span;

use crate::builder::Builder;

/// Builder for deinit declarations.
///
/// Deinit blocks are used for RAII-style cleanup when values go out of scope.
/// Unlike initializers, deinit has no visibility (always private) and no parameters.
pub struct DeinitBuilder;

impl Builder for DeinitBuilder {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        _source: &str,
        file_id: usize,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        _root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let parent = parent?;
        let parent_kind = parent.metadata().kind();
        
        // Deinit can only appear in struct bodies (not protocol or enum)
        if parent_kind != KestrelSymbolKind::Struct {
            return None;
        }

        let full_span = get_node_span(syntax, file_id);

        // Find the deinit keyword token for the declaration span
        let deinit_token_span = syntax
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .find(|tok| tok.kind() == SyntaxKind::Deinit)
            .map(|tok| {
                let start = tok.text_range().start().into();
                let end = tok.text_range().end().into();
                Span::new(file_id, start..end)
            })
            .unwrap_or_else(|| full_span.clone());

        let deinit_symbol = DeinitSymbol::new(
            full_span,
            deinit_token_span,
            Some(parent.clone()),
        );
        let deinit_arc = Arc::new(deinit_symbol);
        let deinit_arc_dyn = deinit_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        parent.metadata().add_child(&deinit_arc_dyn);

        Some(deinit_arc)
    }
}
