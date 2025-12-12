use std::sync::Arc;

use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use kestrel_syntax_tree::utils::{extract_visibility, get_node_span, get_visibility_span};

use crate::builder::Builder;
use kestrel_semantic_tree::behavior::visibility::{Visibility, find_visibility_scope};

/// Builder for initializer declarations.
pub struct InitializerBuilder;

impl Builder for InitializerBuilder {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let parent = parent?;
        let parent_kind = parent.metadata().kind();
        if parent_kind != KestrelSymbolKind::Struct && parent_kind != KestrelSymbolKind::Protocol {
            return None;
        }

        let full_span = get_node_span(syntax, source);

        let init_token_span = syntax
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .find(|tok| tok.kind() == SyntaxKind::Init)
            .map(|tok| {
                let start = tok.text_range().start().into();
                let end = tok.text_range().end().into();
                Span::from(start..end)
            })
            .unwrap_or_else(|| full_span.clone());

        let visibility_str = extract_visibility(syntax);
        let visibility_enum = visibility_str.as_deref().and_then(Visibility::from_keyword);

        let visibility_span =
            get_visibility_span(syntax, source).unwrap_or(init_token_span.clone());
        let visibility_scope = find_visibility_scope(visibility_enum.as_ref(), Some(parent), root);
        let visibility_behavior =
            VisibilityBehavior::new(visibility_enum, visibility_span, visibility_scope);

        let init_symbol = InitializerSymbol::new(
            full_span,
            init_token_span,
            visibility_behavior,
            Some(parent.clone()),
        );
        let init_arc = Arc::new(init_symbol);
        let init_arc_dyn = init_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        parent.metadata().add_child(&init_arc_dyn);

        Some(init_arc)
    }
}
