use std::sync::Arc;

use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_span::Spanned;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use kestrel_syntax_tree::utils::{
    extract_name, extract_visibility, find_child, get_node_span, get_visibility_span,
};

use crate::builder::Builder;
use crate::builders::type_parameter::{add_type_params_as_children, extract_type_parameters};
use kestrel_semantic_tree::behavior::visibility::{Visibility, find_visibility_scope};

/// Builder for function declarations.
pub struct FunctionBuilder;

impl Builder for FunctionBuilder {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        file_id: usize,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let name_str = extract_name(syntax)?;
        let name_node = find_child(syntax, SyntaxKind::Name)?;
        let name_span = get_node_span(&name_node, file_id);

        let full_span = get_node_span(syntax, file_id);

        let visibility_str = extract_visibility(syntax);
        let visibility_enum = visibility_str.as_deref().and_then(Visibility::from_keyword);
        let visibility_span = get_visibility_span(syntax, file_id).unwrap_or(name_span.clone());
        let visibility_scope = find_visibility_scope(visibility_enum.as_ref(), parent, root);
        let visibility_behavior =
            VisibilityBehavior::new(visibility_enum, visibility_span, visibility_scope);

        let is_static = syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::StaticModifier);

        let has_body = syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::FunctionBody);

        let name = Spanned::new(name_str, name_span);

        let function_symbol = FunctionSymbol::new(
            name,
            full_span,
            visibility_behavior,
            is_static,
            has_body,
            parent.cloned(),
        );
        let function_arc = Arc::new(function_symbol);
        let function_arc_dyn = function_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        let type_parameters =
            extract_type_parameters(syntax, source, file_id, Some(function_arc_dyn.clone()));
        add_type_params_as_children(&type_parameters, &function_arc_dyn);

        if let Some(parent) = parent {
            parent.metadata().add_child(&function_arc_dyn);
        }

        Some(function_arc)
    }
}
