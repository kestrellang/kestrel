use kestrel_semantic_tree::behavior::callable::ParameterAccessMode;
use kestrel_semantic_tree::symbol::function::Parameter;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Spanned;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::declaration_binder::BindingContext;
use crate::resolution::type_resolver::{TypeSyntaxContext, resolve_type_from_ty_node};
use kestrel_syntax_tree::utils::{extract_identifier_from_name, find_child, get_node_span};

pub(crate) fn resolve_parameters_from_syntax(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    implicit_labels: bool,
) -> Vec<Parameter> {
    let param_list = match find_child(syntax, SyntaxKind::ParameterList) {
        Some(node) => node,
        None => return Vec::new(),
    };

    param_list
        .children()
        .filter(|child| child.kind() == SyntaxKind::Parameter)
        .filter_map(|param_node| {
            resolve_single_parameter(
                &param_node,
                source,
                file_id,
                context_id,
                ctx,
                implicit_labels,
            )
        })
        .collect()
}

/// Extract access mode from a parameter syntax node.
///
/// Looks for `Mutating` or `Consuming` tokens as direct children of the parameter node.
fn extract_access_mode(param_node: &SyntaxNode) -> ParameterAccessMode {
    for child in param_node.children_with_tokens() {
        if let Some(token) = child.as_token() {
            match token.kind() {
                SyntaxKind::Mutating => return ParameterAccessMode::Mutating,
                SyntaxKind::Consuming => return ParameterAccessMode::Consuming,
                _ => {},
            }
        }
    }
    ParameterAccessMode::Borrow
}

fn resolve_single_parameter(
    param_node: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    implicit_labels: bool,
) -> Option<Parameter> {
    // Extract access mode first (mutating/consuming keyword)
    let access_mode = extract_access_mode(param_node);

    let name_nodes: Vec<SyntaxNode> = param_node
        .children()
        .filter(|child| child.kind() == SyntaxKind::Name)
        .collect();

    if name_nodes.is_empty() {
        return None;
    }

    let (label, bind_name) = if name_nodes.len() >= 2 {
        let label_name = extract_identifier_from_name(&name_nodes[0]);
        let bind_name = Spanned::new(
            extract_identifier_from_name(&name_nodes[1])?,
            get_node_span(&name_nodes[1], file_id),
        );
        (
            label_name.map(|n| Spanned::new(n, get_node_span(&name_nodes[0], file_id))),
            bind_name,
        )
    } else {
        // Single name - use it as both the label and internal binding name.
        let name = extract_identifier_from_name(&name_nodes[0])?;
        let span = get_node_span(&name_nodes[0], file_id);
        let label = implicit_labels.then(|| Spanned::new(name.clone(), span.clone()));
        let bind_name = Spanned::new(name, span);
        (label, bind_name)
    };

    let ty = if let Some(ty_node) = param_node.children().find(|c| c.kind() == SyntaxKind::Ty) {
        let mut type_ctx =
            TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
        resolve_type_from_ty_node(&ty_node, &mut type_ctx)
    } else {
        Ty::infer(get_node_span(param_node, file_id))
    };

    Some(Parameter {
        access_mode,
        label,
        bind_name,
        ty,
    })
}
