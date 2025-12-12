use std::sync::Arc;

use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Spanned;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use kestrel_syntax_tree::utils::{
    extract_name, extract_visibility, find_child, get_node_span, get_visibility_span,
};

use kestrel_semantic_tree::behavior::visibility::{Visibility, find_visibility_scope};
use crate::builder::Builder;
use crate::builders::type_parameter::{add_type_params_as_children, extract_type_parameters};

/// Builder for protocol declarations.
pub struct ProtocolBuilder;

impl Builder for ProtocolBuilder {
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

        let name = Spanned::new(name_str, name_span);

        let protocol_symbol = ProtocolSymbol::new(
            name,
            full_span.clone(),
            visibility_behavior,
            parent.cloned(),
        );
        let protocol_arc = Arc::new(protocol_symbol);

        let protocol_type = Ty::protocol(protocol_arc.clone(), full_span.clone());
        let typed_behavior = TypedBehavior::new(protocol_type, full_span.clone());
        protocol_arc.metadata().add_behavior(typed_behavior);

        let protocol_arc_dyn = protocol_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        let type_parameters = extract_type_parameters(syntax, source, Some(protocol_arc_dyn.clone()));
        add_type_params_as_children(&type_parameters, &protocol_arc_dyn);

        if let Some(parent) = parent {
            parent.metadata().add_child(&protocol_arc_dyn);
        }

        Some(protocol_arc)
    }
}
