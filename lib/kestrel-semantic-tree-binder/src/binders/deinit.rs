use std::sync::Arc;

use kestrel_semantic_tree::behavior::deinit::DeinitBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::deinit::DeinitSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::diagnostics::DuplicateDeinitError;
use crate::resolution::LocalScope;
use kestrel_syntax_tree::utils::find_child;

/// Binder for deinit declarations
pub struct DeinitBinder;

impl DeclarationBinder for DeinitBinder {
    fn bind_signature(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Extract doc comment
        if let Some(doc) = crate::binders::utils::doc_comment::extract_doc_comment(syntax) {
            symbol.metadata().add_behavior(doc);
        }

        // Attach DeinitBehavior to parent struct
        // Also check for duplicate deinit (struct can only have one)
        attach_deinit_behavior_to_parent(symbol, context);
    }

    fn bind_body(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // Resolve deinit body
        if let Some(body_node) = find_child(syntax, SyntaxKind::FunctionBody) {
            resolve_deinit_body(symbol, &body_node, context, &source, file_id);
        }
    }
}

/// Attach DeinitBehavior to the parent struct.
///
/// This also validates that only one deinit exists per struct.
fn attach_deinit_behavior_to_parent(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    context: &mut BindingContext,
) {
    let Some(parent) = symbol.metadata().parent() else {
        return;
    };

    // Only structs can have deinit
    if parent.metadata().kind() != KestrelSymbolKind::Struct {
        return;
    }

    let deinit_id = symbol.metadata().id();

    // Check if parent already has a DeinitBehavior (duplicate deinit error)
    if let Some(existing_deinit) = parent.metadata().get_behavior::<DeinitBehavior>() {
        // Get the existing deinit symbol for span information
        let existing_span = context
            .model
            .registry()
            .get(existing_deinit.deinit_symbol())
            .map(|s| s.metadata().span().clone())
            .unwrap_or_else(|| symbol.metadata().span().clone());

        context.diagnostics.throw(DuplicateDeinitError {
            first_span: existing_span,
            duplicate_span: symbol.metadata().span().clone(),
            struct_name: parent.metadata().name().value.clone(),
        });
        return;
    }

    // Attach the DeinitBehavior to the parent struct
    parent
        .metadata()
        .add_behavior(DeinitBehavior::new(deinit_id));
}

/// Resolve a deinit's body and attach ExecutableBehavior to the symbol
fn resolve_deinit_body(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    body_node: &SyntaxNode,
    context: &mut BindingContext,
    source: &str,
    file_id: usize,
) {
    use crate::body_resolver::BodyResolutionContext;
    use crate::body_resolver::context::{
        create_local_scope_for_body, resolve_body_and_attach_executable,
    };

    // Downcast to DeinitSymbol
    let Some(_) = symbol.as_ref().downcast_ref::<DeinitSymbol>() else {
        return;
    };

    let mut local_scope = if let Ok(deinit) = symbol.clone().downcast_arc::<DeinitSymbol>() {
        LocalScope::new(deinit)
    } else {
        create_local_scope_for_body(symbol.clone(), "__deinit_body_temp")
    };

    // Inject `self` as the first local
    // In deinit, self is read-only - we can access fields but shouldn't modify them
    // (though the language design could allow mutable access before drop)
    if let Some(self_type) = get_self_type(symbol) {
        let decl_span = symbol.metadata().span().clone();
        let self_span = Span::new(decl_span.file_id, decl_span.start..decl_span.start);

        // Add self to local scope (immutable - we're reading, not modifying)
        local_scope.bind(
            "self".to_string(),
            self_type.clone(),
            false, // immutable - deinit can read but not modify self
            self_span.clone(),
        );
    }

    // Deinit has no parameters - just the implicit self

    // Create body resolution context
    let mut body_ctx = BodyResolutionContext::new_with_scope(
        context.model,
        context.diagnostics,
        source,
        file_id,
        symbol.metadata().id(),
        local_scope,
        None, // Deinit doesn't have its own where clause
    );

    resolve_body_and_attach_executable(symbol, body_node, &mut body_ctx);
}

/// Get the type of `self` for a deinit
///
/// Returns the concrete type of the containing struct with type parameters.
fn get_self_type(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<Ty> {
    use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
    use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
    use kestrel_semantic_tree::ty::Substitutions;

    let parent = symbol.metadata().parent()?;
    let parent_span = parent.metadata().span().clone();

    match parent.metadata().kind() {
        KestrelSymbolKind::Struct => {
            // Create concrete struct type with type parameters mapping to themselves
            let struct_arc = Arc::clone(&parent).downcast_arc::<StructSymbol>().ok()?;
            let mut substitutions = Substitutions::new();
            if let Some(generics) = parent.metadata().get_behavior::<GenericsBehavior>() {
                for param in generics.type_parameters() {
                    let param_id = param.metadata().id();
                    let param_ty = Ty::type_parameter(param.clone(), parent_span.clone());
                    substitutions.insert(param_id, param_ty);
                }
            }
            Some(Ty::generic_struct(struct_arc, substitutions, parent_span))
        },
        _ => None,
    }
}
