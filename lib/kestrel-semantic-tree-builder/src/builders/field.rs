use std::sync::Arc;

use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Spanned;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use kestrel_syntax_tree::utils::{
    extract_name, extract_visibility, find_child, get_node_span, get_visibility_span,
};

use crate::builder::Builder;
use kestrel_semantic_tree::behavior::visibility::{Visibility, find_visibility_scope};

/// Builder for field declarations.
pub struct FieldBuilder;

impl Builder for FieldBuilder {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let name_str = extract_name(syntax)?;
        let name_node = find_child(syntax, SyntaxKind::Name)?;
        let name_span = get_node_span(&name_node, source);

        let full_span = get_node_span(syntax, source);

        let visibility_str = extract_visibility(syntax);
        let visibility_enum = visibility_str.as_deref().and_then(Visibility::from_keyword);

        let visibility_span = get_visibility_span(syntax, source).unwrap_or(name_span.clone());
        let visibility_scope = find_visibility_scope(visibility_enum.as_ref(), parent, root);
        let visibility_behavior =
            VisibilityBehavior::new(visibility_enum, visibility_span, visibility_scope);

        let is_static = syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::StaticModifier);

        let is_mutable = syntax
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .any(|tok| tok.kind() == SyntaxKind::Var);

        let field_type = Ty::error(full_span.clone());
        let name = Spanned::new(name_str, name_span);

        let field_symbol = FieldSymbol::new(
            name,
            full_span,
            visibility_behavior,
            is_static,
            is_mutable,
            field_type,
            parent.cloned(),
        );
        let field_arc = Arc::new(field_symbol);
        let field_arc_dyn = field_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        if let Some(parent) = parent {
            parent.metadata().add_child(&field_arc_dyn);
        }

        Some(field_arc)
    }
}
