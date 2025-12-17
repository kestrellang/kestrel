use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::{CallableBehavior, ReceiverKind};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::Parameter;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::resolution::type_resolver::{resolve_type_from_ty_node, TypeSyntaxContext};
use kestrel_syntax_tree::utils::{find_child, get_node_span};

/// Binder for function declarations
pub struct FunctionBinder;

impl DeclarationBinder for FunctionBinder {
    fn bind_declaration(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Only process function symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Function {
            return;
        }

        let symbol_id = symbol.metadata().id();
        let span = symbol.metadata().span().clone();

        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // Extract type parameters and resolve where clause bounds FIRST
        // This must happen before resolving parameter/return types so that
        // T.Item paths can find the protocol bounds for T
        let generics_behavior =
            crate::binders::utils::generics::resolve_generics(syntax, &source, file_id, symbol_id, context);
        symbol.metadata().add_behavior(generics_behavior);

        // Now extract and resolve parameters from syntax (T.Item will work)
        let resolved_params = crate::binders::utils::parameters::resolve_parameters_from_syntax(
            syntax,
            &source,
            file_id,
            symbol_id,
            context,
            false,
        );

        // Extract and resolve return type from syntax (T.Item will work)
        let resolved_return =
            resolve_return_type_from_syntax(syntax, &source, file_id, symbol_id, context);

        // Determine receiver kind for instance methods
        let receiver_kind = determine_receiver_kind(syntax, symbol);

        // Add a new CallableBehavior with resolved types
        let resolved_callable = match receiver_kind {
            Some(kind) => CallableBehavior::with_receiver(
                resolved_params.clone(),
                resolved_return,
                kind,
                span,
            ),
            None => CallableBehavior::new(resolved_params.clone(), resolved_return, span),
        };
        symbol.metadata().add_behavior(resolved_callable);

        // Resolve function body if present
        if let Some(body_node) = find_child(syntax, SyntaxKind::FunctionBody) {
            resolve_function_body(
                symbol,
                &body_node,
                &resolved_params,
                context,
                &source,
                file_id,
            );
        }
    }
}

/// Resolve a function's body and attach ExecutableBehavior to the symbol
fn resolve_function_body(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    body_node: &SyntaxNode,
    params: &[Parameter],
    context: &mut BindingContext,
    source: &str,
    file_id: usize,
) {
    use crate::body_resolver::context::{create_local_scope_for_body, resolve_body_and_attach_executable};
    use crate::body_resolver::BodyResolutionContext;
    use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
    use kestrel_semantic_tree::symbol::function::FunctionSymbol;

    // Downcast to FunctionSymbol to get Arc<FunctionSymbol> for LocalScope
    let Some(func_sym) = symbol.as_ref().downcast_ref::<FunctionSymbol>() else {
        return;
    };

    let mut local_scope = create_local_scope_for_body(symbol.clone(), "__body_temp");

    // Get receiver kind from CallableBehavior to determine if we need to inject `self`
    let receiver_kind = symbol
        .metadata()
        .behaviors()
        .iter()
        .find(|b| matches!(b.kind(), KestrelBehaviorKind::Callable))
        .and_then(|b| b.as_ref().downcast_ref::<CallableBehavior>())
        .and_then(|cb| cb.receiver());

    // If this is an instance method, inject `self` as the first local
    if let Some(receiver) = receiver_kind {
        if let Some(self_type) = get_self_type(symbol) {
            let is_mutable = matches!(receiver, ReceiverKind::Mutating);
            let decl_span = symbol.metadata().span().clone();
            let self_span = Span::new(decl_span.file_id, decl_span.start..decl_span.start);

            // Add self to local scope
            local_scope.bind(
                "self".to_string(),
                self_type.clone(),
                is_mutable,
                self_span.clone(),
            );
            // Add to the actual function symbol
            func_sym.add_local("self".to_string(), self_type, is_mutable, self_span);
        }
    }

    // Add parameters to local scope
    for param in params {
        let param_ty = param.ty.clone();
        let param_name = param.bind_name.value.clone();
        let param_span = param.bind_name.span.clone();
        // Add to local scope and also to the actual function
        local_scope.bind(
            param_name.clone(),
            param_ty.clone(),
            false,
            param_span.clone(),
        );
        // Add to the actual function symbol
        func_sym.add_local(param_name, param_ty, false, param_span);
    }

    // Create body resolution context
    let mut body_ctx = BodyResolutionContext {
        model: context.model,
        diagnostics: context.diagnostics,
        source,
        file_id,
        function_id: symbol.metadata().id(),
        local_scope,
        loop_stack: Vec::new(),
        next_loop_id: 0,
    };

    resolve_body_and_attach_executable(symbol, body_node, &mut body_ctx);
}

/// Resolve return type from a FunctionDeclaration syntax node during bind phase
fn resolve_return_type_from_syntax(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
) -> Ty {
    // Find the return type node: FunctionDeclaration -> ReturnType -> Ty
    if let Some(return_type_node) = find_child(syntax, SyntaxKind::ReturnType) {
        if let Some(ty_node) = find_child(&return_type_node, SyntaxKind::Ty) {
            let mut type_ctx =
                TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
            return resolve_type_from_ty_node(&ty_node, &mut type_ctx);
        }
    }

    // No explicit return type - use inference placeholder
    // The type inference system will determine the actual return type
    let fn_span = get_node_span(syntax, file_id);
    Ty::infer(Span::new(fn_span.file_id, fn_span.end..fn_span.end))
}

/// Get the type of `self` for an instance method
///
/// Returns the type of the containing struct, protocol, or extension target.
/// For extensions, we use Self type which will resolve to the target type.
fn get_self_type(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<Ty> {
    let parent = symbol.metadata().parent()?;
    let parent_span = parent.metadata().span().clone();

    match parent.metadata().kind() {
        KestrelSymbolKind::Struct | KestrelSymbolKind::Protocol => {
            // Use Self type which refers to the containing type
            // This will be resolved to the concrete type during type checking
            Some(Ty::self_type(parent_span))
        }
        KestrelSymbolKind::Extension => {
            // For extension methods, Self refers to the target type
            // Use Self type which will be resolved during type checking
            Some(Ty::self_type(parent_span))
        }
        _ => None,
    }
}

/// Determine the receiver kind for a function declaration
///
/// Returns:
/// - `None` for static functions and free functions (not in a struct/protocol)
/// - `Some(ReceiverKind)` for instance methods
fn determine_receiver_kind(
    syntax: &SyntaxNode,
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Option<ReceiverKind> {
    // Check if this function is static
    let is_static = syntax
        .children()
        .any(|child| child.kind() == SyntaxKind::StaticModifier);

    if is_static {
        return None; // Static functions have no receiver
    }

    // Check if the function is in a struct, protocol, or extension (instance method)
    let parent_kind = symbol.metadata().parent().map(|p| p.metadata().kind());
    let is_instance_method = matches!(
        parent_kind,
        Some(KestrelSymbolKind::Struct)
            | Some(KestrelSymbolKind::Protocol)
            | Some(KestrelSymbolKind::Extension)
    );

    if !is_instance_method {
        return None; // Free functions have no receiver
    }

    // Check for receiver modifier (mutating/consuming)
    let has_mutating = syntax
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .any(|tok| tok.kind() == SyntaxKind::Mutating);

    let has_consuming = syntax
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .any(|tok| tok.kind() == SyntaxKind::Consuming);

    // Determine receiver kind
    match (has_mutating, has_consuming) {
        (true, _) => Some(ReceiverKind::Mutating),
        (_, true) => Some(ReceiverKind::Consuming),
        _ => Some(ReceiverKind::Borrowing), // Default for instance methods
    }
}
