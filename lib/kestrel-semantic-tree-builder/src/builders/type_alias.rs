use std::sync::Arc;

use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasSymbol;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Spanned;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use kestrel_syntax_tree::utils::{
    extract_visibility, find_child, get_node_span, get_visibility_span,
};

use crate::builder::Builder;
use crate::builders::type_parameter::{add_type_params_as_children, extract_type_parameters};
use kestrel_semantic_tree::behavior::visibility::{Visibility, find_visibility_scope};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeAliasContext {
    Protocol,
    ConcreteType,
    Module,
}

/// Builder for type alias declarations.
pub struct TypeAliasBuilder;

impl Builder for TypeAliasBuilder {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        file_id: usize,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let context = determine_context(parent);

        let (name_str, name_span) = extract_type_alias_name(syntax, source, file_id)?;
        let full_span = get_node_span(syntax, file_id);

        let visibility_str = extract_visibility(syntax);
        let visibility_enum = visibility_str.as_deref().and_then(Visibility::from_keyword);
        let visibility_span = get_visibility_span(syntax, file_id).unwrap_or(name_span.clone());
        let visibility_scope = find_visibility_scope(visibility_enum.as_ref(), parent, root);
        let visibility_behavior =
            VisibilityBehavior::new(visibility_enum, visibility_span, visibility_scope);

        let name = Spanned::new(name_str, name_span.clone());

        match context {
            TypeAliasContext::Protocol => {
                let symbol = AssociatedTypeSymbol::new(
                    name.clone(),
                    full_span.clone(),
                    visibility_behavior,
                    parent.cloned(),
                );
                let symbol_arc = Arc::new(symbol);
                let symbol_arc_dyn = symbol_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

                if let Some(parent) = parent {
                    parent.metadata().add_child(&symbol_arc_dyn);
                }

                Some(symbol_arc_dyn)
            },
            TypeAliasContext::ConcreteType | TypeAliasContext::Module => {
                let placeholder_type = Ty::error(full_span.clone());
                let syntactic_typed_behavior =
                    TypedBehavior::new(placeholder_type, full_span.clone());

                let type_alias_symbol = TypeAliasSymbol::new(
                    name.clone(),
                    full_span.clone(),
                    visibility_behavior,
                    syntactic_typed_behavior,
                    parent.cloned(),
                );
                let type_alias_arc = Arc::new(type_alias_symbol);

                let type_alias_type = Ty::type_alias(type_alias_arc.clone(), full_span.clone());
                let semantic_typed_behavior =
                    TypedBehavior::new(type_alias_type, full_span.clone());
                type_alias_arc
                    .metadata()
                    .add_behavior(semantic_typed_behavior);

                let type_alias_arc_dyn = type_alias_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

                let type_parameters = extract_type_parameters(
                    syntax,
                    source,
                    file_id,
                    Some(type_alias_arc_dyn.clone()),
                );
                add_type_params_as_children(&type_parameters, &type_alias_arc_dyn);

                if let Some(parent) = parent {
                    parent.metadata().add_child(&type_alias_arc_dyn);
                }

                Some(type_alias_arc_dyn)
            },
        }
    }
}

fn determine_context(parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>) -> TypeAliasContext {
    match parent {
        Some(p) => match p.metadata().kind() {
            KestrelSymbolKind::Protocol => TypeAliasContext::Protocol,
            KestrelSymbolKind::Struct | KestrelSymbolKind::Enum | KestrelSymbolKind::Extension => TypeAliasContext::ConcreteType,
            _ => TypeAliasContext::Module,
        },
        None => TypeAliasContext::Module,
    }
}

fn extract_type_alias_name(
    syntax: &SyntaxNode,
    _source: &str,
    file_id: usize,
) -> Option<(String, kestrel_span::Span)> {
    if let Some(target_node) = find_child(syntax, SyntaxKind::AssociatedTypeTarget)
        && let Some(name_node) = find_child(&target_node, SyntaxKind::Name)
    {
        let name_str = extract_name_from_node(&name_node)?;
        let name_span = get_node_span(&name_node, file_id);
        return Some((name_str, name_span));
    }

    if let Some(name_node) = find_child(syntax, SyntaxKind::Name) {
        let name_str = extract_name_from_node(&name_node)?;
        let name_span = get_node_span(&name_node, file_id);
        return Some((name_str, name_span));
    }

    None
}

fn extract_name_from_node(name_node: &SyntaxNode) -> Option<String> {
    name_node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .find(|tok| tok.kind() == SyntaxKind::Identifier)
        .map(|tok| tok.text().to_string())
}
