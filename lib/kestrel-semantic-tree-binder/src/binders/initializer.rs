use std::sync::Arc;

use kestrel_semantic_model::SymbolFor;
use kestrel_semantic_tree::behavior::callable::{CallableBehavior, ReceiverKind};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::Parameter;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::{Span, Spanned};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::resolution::type_resolver::{TypeSyntaxContext, resolve_type_from_ty_node};
use kestrel_syntax_tree::utils::{extract_identifier_from_name, find_child, get_node_span};

/// Binder for initializer declarations
pub struct InitializerBinder;

impl DeclarationBinder for InitializerBinder {
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

        let source = context.source_for_symbol(symbol);

        // Extract and resolve parameters from syntax
        let resolved_params = resolve_parameters_from_syntax(syntax, &source, symbol_id, context);

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
            resolve_initializer_body(symbol, &body_node, &resolved_params, context, &source);
        }
    }
}

/// Resolve an initializer's body and attach ExecutableBehavior to the symbol
fn resolve_initializer_body(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    body_node: &SyntaxNode,
    params: &[Parameter],
    context: &mut BindingContext,
    source: &str,
) {
    use crate::body_resolver::{BodyResolutionContext, resolve_function_body as resolve_body};
    use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
    use kestrel_semantic_tree::behavior::visibility::{Visibility, VisibilityBehavior};
    use kestrel_semantic_tree::symbol::function::FunctionSymbol;
    use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;

    // Downcast to InitializerSymbol
    let Some(init_sym) = symbol.as_ref().downcast_ref::<InitializerSymbol>() else {
        return;
    };

    // Get the symbol from db
    let Some(init_arc) = context.model.query(SymbolFor {
        id: symbol.metadata().id(),
    }) else {
        return;
    };

    // Verify it's an InitializerSymbol
    if init_arc
        .as_ref()
        .downcast_ref::<InitializerSymbol>()
        .is_none()
    {
        return;
    }

    // Create a temporary FunctionSymbol for LocalScope (reuse existing infrastructure)
    let temp_name = Spanned::new("__init_body_temp".to_string(), Span::from(0..0));
    let temp_vis = VisibilityBehavior::new(
        Some(Visibility::Private),
        Span::from(0..0),
        init_arc.clone(),
    );
    let temp_func = Arc::new(FunctionSymbol::new(
        temp_name,
        Span::from(0..0),
        temp_vis,
        false,
        true,
        None,
    ));

    let mut local_scope = crate::resolution::LocalScope::new(temp_func);

    // Inject `self` as the first local (with initializing semantics)
    // In initializers, self is mutable so we can assign to fields
    if let Some(self_type) = get_self_type(symbol) {
        let self_span = Span::from(symbol.metadata().span().start..symbol.metadata().span().start);

        // Add self to local scope (mutable because we're initializing it)
        local_scope.bind(
            "self".to_string(),
            self_type.clone(),
            true,
            self_span.clone(),
        );
        // Add to the actual initializer symbol
        init_sym.add_local("self".to_string(), self_type, true, self_span);
    }

    // Add parameters to local scope
    for param in params {
        let param_ty = param.ty.clone();
        let param_name = param.bind_name.value.clone();
        let param_span = param.bind_name.span.clone();
        // Add to local scope
        local_scope.bind(
            param_name.clone(),
            param_ty.clone(),
            false,
            param_span.clone(),
        );
        // Add to the actual initializer symbol
        init_sym.add_local(param_name, param_ty, false, param_span);
    }

    // Create body resolution context
    let mut body_ctx = BodyResolutionContext {
        model: context.model,
        diagnostics: context.diagnostics,
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
        .filter_map(|param_node| resolve_single_parameter(&param_node, source, context_id, ctx))
        .collect()
}

/// Resolve a single parameter from syntax
fn resolve_single_parameter(
    param_node: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
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
    fn get_name_span(name_node: &SyntaxNode, source: &str) -> kestrel_span::Span {
        get_node_span(name_node, source)
    }

    // Determine label and bind_name based on number of Name nodes
    let (label, bind_name) = if name_nodes.len() >= 2 {
        // Two names: first is label, second is bind_name
        let label_name = extract_identifier_from_name(&name_nodes[0]);
        let bind_name = Spanned::new(
            extract_identifier_from_name(&name_nodes[1])?,
            get_name_span(&name_nodes[1], source),
        );
        (
            label_name.map(|n| Spanned::new(n, get_name_span(&name_nodes[0], source))),
            bind_name,
        )
    } else {
        // One name: it's both the label AND the bind_name
        // In Kestrel, `init(value: Int)` means value is the external label too
        let name = extract_identifier_from_name(&name_nodes[0])?;
        let span = get_name_span(&name_nodes[0], source);
        let label = Some(Spanned::new(name.clone(), span.clone()));
        let bind_name = Spanned::new(name, span);
        (label, bind_name)
    };

    // Find and resolve the type from Ty node
    let ty = if let Some(ty_node) = param_node.children().find(|c| c.kind() == SyntaxKind::Ty) {
        let mut type_ctx = TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, context_id);
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

    Some(Parameter {
        label,
        bind_name,
        ty,
    })
}
