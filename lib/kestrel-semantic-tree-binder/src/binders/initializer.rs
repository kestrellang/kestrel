use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::{CallableBehavior, ReceiverKind};
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::Parameter;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::resolution::LocalScope;
use kestrel_syntax_tree::utils::find_child;

/// Binder for initializer declarations
pub struct InitializerBinder;

impl DeclarationBinder for InitializerBinder {
    fn bind_signature(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        let symbol_id = symbol.metadata().id();
        let span = symbol.metadata().span().clone();

        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // Resolve attributes
        let attributes_behavior = crate::binders::utils::attributes::resolve_attributes(
            syntax,
            &source,
            file_id,
            context.diagnostics,
        );
        symbol.metadata().add_behavior(attributes_behavior);

        // Extract doc comment
        if let Some(doc) = crate::binders::utils::doc_comment::extract_doc_comment(syntax) {
            symbol.metadata().add_behavior(doc);
        }

        // Extract type parameters and resolve where clause bounds FIRST
        // This must happen before resolving parameter/return types so that
        // T.Item paths can find the protocol bounds for T
        let generics_behavior = crate::binders::utils::generics::resolve_generics(
            syntax, &source, file_id, symbol_id, context,
        );
        symbol.metadata().add_behavior(generics_behavior);

        // Extract and resolve parameters from syntax
        // Initializers use explicit labels only (like functions), not implicit labels
        let resolved_params = crate::binders::utils::parameters::resolve_parameters_from_syntax(
            syntax, &source, file_id, symbol_id, context, false,
        );

        // Initializers return unit type - they don't return a value
        let return_type = Ty::unit(span.clone());

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

        // NOTE: Body resolution is deferred to bind_body() to handle forward references
    }

    fn bind_body(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Get the CallableBehavior to extract resolved parameters
        let Some(callable) = symbol.metadata().get_behavior::<CallableBehavior>() else {
            return;
        };
        let resolved_params = callable.parameters().to_vec();

        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // Resolve initializer body if present
        if let Some(body_node) = find_child(syntax, SyntaxKind::FunctionBody) {
            resolve_initializer_body(
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

/// Resolve an initializer's body and attach ExecutableBehavior to the symbol
fn resolve_initializer_body(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    body_node: &SyntaxNode,
    params: &[Parameter],
    context: &mut BindingContext,
    source: &str,
    file_id: usize,
) {
    use crate::body_resolver::BodyResolutionContext;
    use crate::body_resolver::context::{
        create_local_scope_for_body, resolve_body_and_attach_executable,
    };
    use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;

    // Downcast to InitializerSymbol
    let Some(_) = symbol.as_ref().downcast_ref::<InitializerSymbol>() else {
        return;
    };

    let mut local_scope = if let Ok(init) = symbol.clone().downcast_arc::<InitializerSymbol>() {
        LocalScope::new(init)
    } else {
        create_local_scope_for_body(symbol.clone(), "__init_body_temp")
    };

    // Inject `self` as the first local (with initializing semantics)
    // In initializers, self is mutable so we can assign to fields
    if let Some(self_type) = get_self_type(symbol, context.model) {
        let decl_span = symbol.metadata().span().clone();
        let self_span = Span::new(decl_span.file_id, decl_span.start..decl_span.start);

        // Add self to local scope (mutable because we're initializing it)
        local_scope.bind(
            "self".to_string(),
            self_type.clone(),
            true,
            self_span.clone(),
        );
    }

    // Add parameters to local scope
    // Mutability depends on access mode:
    // - Borrow: immutable (read-only)
    // - Mutating: mutable (read-write, but caller keeps ownership)
    // - Consuming: mutable (takes ownership, can modify)
    for param in params {
        let param_ty = param.ty.clone();
        let param_name = param.bind_name.value.clone();
        let param_span = param.bind_name.span.clone();
        let is_mutable = param.access_mode.is_mutable();
        // Add to local scope
        local_scope.bind(
            param_name.clone(),
            param_ty.clone(),
            is_mutable,
            param_span.clone(),
        );
    }

    // Get where clause from GenericsBehavior if present
    let where_clause = symbol
        .metadata()
        .get_behavior::<GenericsBehavior>()
        .map(|g| g.where_clause().clone());

    // Create body resolution context
    let mut body_ctx = BodyResolutionContext::new_with_scope(
        context.model,
        context.diagnostics,
        source,
        file_id,
        symbol.metadata().id(),
        local_scope,
        where_clause,
    );

    resolve_body_and_attach_executable(symbol, body_node, &mut body_ctx);
}

/// Get the type of `self` for an initializer
///
/// Returns the concrete type of the containing struct with type parameters.
fn get_self_type(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    model: &kestrel_semantic_model::SemanticModel,
) -> Option<Ty> {
    let parent = symbol.metadata().parent()?;
    crate::binders::utils::self_type::self_type_for_parent(&parent, model)
}
