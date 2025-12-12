use std::sync::Arc;

use kestrel_semantic_model::SymbolFor;
use kestrel_semantic_tree::behavior::callable::{CallableBehavior, ReceiverKind};
use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::Parameter;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::{Span, Spanned};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::resolver::{BindingContext, Resolver};
use crate::resolution::type_resolver::{resolve_type_from_ty_node, TypeSyntaxContext};
use crate::syntax::{
    extract_identifier_from_name, extract_visibility, find_child, find_visibility_scope,
    get_node_span, get_visibility_span, parse_visibility,
};

/// Resolver for initializer declarations
pub struct InitializerResolver;

impl Resolver for InitializerResolver {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        // Initializers must be inside a struct or protocol
        let parent = parent?;
        let parent_kind = parent.metadata().kind();
        if parent_kind != KestrelSymbolKind::Struct && parent_kind != KestrelSymbolKind::Protocol {
            return None;
        }

        // Get full span
        let full_span = get_node_span(syntax, source);

        // Find the `init` token to get declaration span
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

        // Extract visibility
        let visibility_str = extract_visibility(syntax);
        let visibility_enum = visibility_str.as_deref().and_then(parse_visibility);

        let visibility_span = get_visibility_span(syntax, source).unwrap_or(init_token_span.clone());

        // Determine visibility scope
        let visibility_scope = find_visibility_scope(visibility_enum.as_ref(), Some(parent), root);

        // Create visibility behavior
        let visibility_behavior =
            VisibilityBehavior::new(visibility_enum, visibility_span, visibility_scope);

        // Create the initializer symbol
        let init_symbol = InitializerSymbol::new(
            full_span,
            init_token_span,
            visibility_behavior,
            Some(parent.clone()),
        );
        let init_arc = Arc::new(init_symbol);
        let init_arc_dyn = init_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        // Add to parent struct
        parent.metadata().add_child(&init_arc_dyn);

        Some(init_arc)
    }

    fn bind_declaration(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Only process initializer symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Initializer {
            return;
        }

        let symbol_id = symbol.metadata().id();
        let span = symbol.metadata().span().clone();

        // Get file_id and source for this symbol
        let (file_id, source) = context.get_file_context(symbol);

        // Extract and resolve parameters from syntax
        let resolved_params = resolve_parameters_from_syntax(syntax, &source, symbol_id, context, file_id);

        // Initializers always return Self (the struct type)
        // Get the parent struct to determine Self type
        let return_type = symbol
            .metadata()
            .parent()
            .map(|p| Ty::self_type(p.metadata().span().clone()))
            .unwrap_or_else(|| Ty::error(span.clone()));

        // Initializers always have ReceiverKind::Initializing
        let receiver_kind = ReceiverKind::Initializing;

        // Create CallableBehavior with initializing receiver
        let resolved_callable = CallableBehavior::with_receiver(
            resolved_params.clone(),
            return_type,
            receiver_kind,
            span,
        );
        symbol.metadata().add_behavior(resolved_callable);

        // Resolve initializer body
        if let Some(body_node) = find_child(syntax, SyntaxKind::FunctionBody) {
            resolve_initializer_body(symbol, &body_node, &resolved_params, context, file_id, &source);
        }
    }
}

/// Resolve an initializer's body and attach ExecutableBehavior to the symbol
fn resolve_initializer_body(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    body_node: &SyntaxNode,
    params: &[Parameter],
    context: &mut BindingContext,
    file_id: usize,
    source: &str,
) {
    use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
    use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
    use crate::body_resolver::{BodyResolutionContext, resolve_function_body as resolve_body};
    use kestrel_semantic_tree::symbol::function::FunctionSymbol;
    use kestrel_semantic_tree::behavior::visibility::{Visibility, VisibilityBehavior};

    // Downcast to InitializerSymbol
    let Some(init_sym) = symbol.as_ref().downcast_ref::<InitializerSymbol>() else {
        return;
    };

    // Get the symbol from db
    let Some(init_arc) = context.model.query(SymbolFor { id: symbol.metadata().id() }) else {
        return;
    };

    // Verify it's an InitializerSymbol
    if init_arc.as_ref().downcast_ref::<InitializerSymbol>().is_none() {
        return;
    }

    // Create a temporary FunctionSymbol for LocalScope (reuse existing infrastructure)
    let temp_name = Spanned::new("__init_body_temp".to_string(), Span::from(0..0));
    let temp_vis = VisibilityBehavior::new(Some(Visibility::Private), Span::from(0..0), init_arc.clone());
    let temp_func = Arc::new(FunctionSymbol::new(
        temp_name,
        Span::from(0..0),
        temp_vis,
        false,
        true,
        vec![],
        Ty::unit(Span::from(0..0)),
        None,
    ));

    let mut local_scope = crate::resolution::LocalScope::new(temp_func);

    // Inject `self` as the first local (with initializing semantics)
    // In initializers, self is mutable so we can assign to fields
    if let Some(self_type) = get_self_type(symbol) {
        let self_span = Span::from(symbol.metadata().span().start..symbol.metadata().span().start);

        // Add self to local scope (mutable because we're initializing it)
        local_scope.bind("self".to_string(), self_type.clone(), true, self_span.clone());
        // Add to the actual initializer symbol
        init_sym.add_local("self".to_string(), self_type, true, self_span);
    }

    // Add parameters to local scope
    for param in params {
        let param_ty = param.ty.clone();
        let param_name = param.bind_name.value.clone();
        let param_span = param.bind_name.span.clone();
        // Add to local scope
        local_scope.bind(param_name.clone(), param_ty.clone(), false, param_span.clone());
        // Add to the actual initializer symbol
        init_sym.add_local(param_name, param_ty, false, param_span);
    }

    // Create body resolution context
    let mut body_ctx = BodyResolutionContext {
        model: context.model,
        diagnostics: context.diagnostics,
        file_id,
        source,
        function_id: symbol.metadata().id(),
        local_scope,
        loop_stack: Vec::new(),
        next_loop_id: 0,
    };

    // Resolve the body
    let code_block = resolve_body(body_node, &mut body_ctx);

    // Create and attach ExecutableBehavior
    let executable = ExecutableBehavior::new(code_block);
    symbol.metadata().add_behavior(executable);
}

/// Get the type of `self` for an initializer
///
/// Returns the type of the containing struct.
fn get_self_type(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<Ty> {
    let parent = symbol.metadata().parent()?;
    let parent_span = parent.metadata().span().clone();

    match parent.metadata().kind() {
        KestrelSymbolKind::Struct => {
            // Use Self type which refers to the containing struct
            Some(Ty::self_type(parent_span))
        }
        _ => None,
    }
}

/// Resolve parameters from an InitializerDeclaration syntax node during bind phase
fn resolve_parameters_from_syntax(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    file_id: usize,
) -> Vec<Parameter> {
    // Find the ParameterList node
    let param_list = match find_child(syntax, SyntaxKind::ParameterList) {
        Some(node) => node,
        None => return Vec::new(),
    };

    // Extract and resolve each parameter
    param_list
        .children()
        .filter(|child| child.kind() == SyntaxKind::Parameter)
        .filter_map(|param_node| resolve_single_parameter(&param_node, source, context_id, ctx, file_id))
        .collect()
}

/// Resolve a single parameter from syntax
fn resolve_single_parameter(
    param_node: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    file_id: usize,
) -> Option<Parameter> {
    // Collect all Name nodes
    let name_nodes: Vec<SyntaxNode> = param_node
        .children()
        .filter(|child| child.kind() == SyntaxKind::Name)
        .collect();

    if name_nodes.is_empty() {
        return None;
    }

    // Get span helper
    fn get_name_span(name_node: &SyntaxNode) -> kestrel_span::Span {
        let start = name_node.text_range().start().into();
        let end = name_node.text_range().end().into();
        Span::from(start..end)
    }

    // Determine label and bind_name based on number of Name nodes
    let (label, bind_name) = if name_nodes.len() >= 2 {
        // Two names: first is label, second is bind_name
        let label_name = extract_identifier_from_name(&name_nodes[0]);
        let bind_name = Spanned::new(
            extract_identifier_from_name(&name_nodes[1])?,
            get_name_span(&name_nodes[1]),
        );
        (label_name.map(|n| Spanned::new(n, get_name_span(&name_nodes[0]))), bind_name)
    } else {
        // One name: it's both the label AND the bind_name
        // In Kestrel, `init(value: Int)` means value is the external label too
        let name = extract_identifier_from_name(&name_nodes[0])?;
        let span = get_name_span(&name_nodes[0]);
        let label = Some(Spanned::new(name.clone(), span.clone()));
        let bind_name = Spanned::new(name, span);
        (label, bind_name)
    };

    // Find and resolve the type from Ty node
    let ty = if let Some(ty_node) = param_node.children().find(|c| c.kind() == SyntaxKind::Ty) {
        let mut type_ctx = TypeSyntaxContext::new(ctx.model, ctx.diagnostics, file_id, source, context_id);
        resolve_type_from_ty_node(&ty_node, &mut type_ctx)
    } else {
        // No type annotation - type variable
        let param_span: kestrel_span::Span = {
            let start = param_node.text_range().start().into();
            let end = param_node.text_range().end().into();
            Span::from(start..end)
        };
        Ty::type_var(param_span)
    };

    Some(Parameter { label, bind_name, ty })
}
