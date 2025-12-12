use std::sync::Arc;

use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::{FunctionSymbol, Parameter};
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::{Span, Spanned};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use kestrel_syntax_tree::utils::{
    extract_identifier_from_name, extract_name, extract_visibility, find_child, get_node_span,
    get_visibility_span,
};

use kestrel_semantic_tree::behavior::visibility::{Visibility, find_visibility_scope};
use crate::builder::Builder;
use crate::builders::type_parameter::{add_type_params_as_children, extract_type_parameters};

/// Builder for function declarations.
pub struct FunctionBuilder;

impl Builder for FunctionBuilder {
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

        let has_body = syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::FunctionBody);

        let parameters = extract_parameters(syntax, source);
        let return_type = extract_return_type(syntax, source);

        let name = Spanned::new(name_str, name_span);

        let function_symbol = FunctionSymbol::with_generics(
            name,
            full_span,
            visibility_behavior,
            is_static,
            has_body,
            parameters,
            return_type,
            parent.cloned(),
        );
        let function_arc = Arc::new(function_symbol);
        let function_arc_dyn = function_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        let type_parameters = extract_type_parameters(syntax, source, Some(function_arc_dyn.clone()));
        add_type_params_as_children(&type_parameters, &function_arc_dyn);

        if let Some(parent) = parent {
            parent.metadata().add_child(&function_arc_dyn);
        }

        Some(function_arc)
    }
}

fn extract_parameters(syntax: &SyntaxNode, source: &str) -> Vec<Parameter> {
    let param_list = match find_child(syntax, SyntaxKind::ParameterList) {
        Some(node) => node,
        None => return Vec::new(),
    };

    param_list
        .children()
        .filter(|child| child.kind() == SyntaxKind::Parameter)
        .filter_map(|param_node| extract_single_parameter(&param_node, source))
        .collect()
}

fn extract_single_parameter(param_node: &SyntaxNode, source: &str) -> Option<Parameter> {
    let name_nodes: Vec<SyntaxNode> = param_node
        .children()
        .filter(|child| child.kind() == SyntaxKind::Name)
        .collect();

    if name_nodes.is_empty() {
        return None;
    }

    let (label, bind_name) = if name_nodes.len() >= 2 {
        let label_str = extract_identifier_from_name(&name_nodes[0])?;
        let label_span = get_node_span(&name_nodes[0], source);
        let label = Spanned::new(label_str, label_span);

        let bind_str = extract_identifier_from_name(&name_nodes[1])?;
        let bind_span = get_node_span(&name_nodes[1], source);
        let bind_name = Spanned::new(bind_str, bind_span);

        (Some(label), bind_name)
    } else {
        let bind_str = extract_identifier_from_name(&name_nodes[0])?;
        let bind_span = get_node_span(&name_nodes[0], source);
        let bind_name = Spanned::new(bind_str, bind_span);
        (None, bind_name)
    };

    let ty = kestrel_semantic_tree_builder::resolution::type_resolver::extract_type_from_node(
        param_node, source,
    );

    Some(match label {
        Some(l) => Parameter::with_label(l, bind_name, ty),
        None => Parameter::new(bind_name, ty),
    })
}

fn extract_return_type(syntax: &SyntaxNode, source: &str) -> Ty {
    if let Some(ret_node) = find_child(syntax, SyntaxKind::ReturnType) {
        if let Some(ty_node) = find_child(&ret_node, SyntaxKind::Ty) {
            return kestrel_semantic_tree_builder::resolution::type_resolver::extract_type_from_ty_node(
                &ty_node,
                source,
            );
        }
    }

    let fn_span = get_node_span(syntax, source);
    Ty::unit(Span::from(fn_span.end..fn_span.end))
}
