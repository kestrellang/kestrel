use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::{
    CallableBehavior, CallableParameter, ParameterAccessMode, ReceiverKind,
};
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::setter::SetterSymbol;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::{Name, Span};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::declaration_binder::{BindingContext, DeclarationBinder};
use kestrel_syntax_tree::utils::find_child;

/// Binder for setter declarations (computed property setters)
pub struct SetterBinder;

impl DeclarationBinder for SetterBinder {
    fn bind_signature(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        _syntax: &SyntaxNode,
        _context: &mut BindingContext,
    ) {
        // Only process setter symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Setter {
            return;
        }

        let span = symbol.metadata().span().clone();

        // Get the parent field symbol to determine the field type and static modifier
        let Some(parent) = symbol.metadata().parent() else {
            return;
        };

        if parent.metadata().kind() != KestrelSymbolKind::Field {
            return;
        }

        // Get the field type from the parent field's TypedBehavior
        let field_type = parent
            .metadata()
            .get_behavior::<TypedBehavior>()
            .map(|tb| tb.ty().clone())
            .unwrap_or_else(|| Ty::error(span.clone()));

        // Check if the field is static by downcasting to FieldSymbol
        let is_static = parent
            .as_ref()
            .downcast_ref::<FieldSymbol>()
            .map(|f| f.is_static())
            .unwrap_or(false);

        // Create the `newValue` parameter with consuming access mode
        // The parameter takes the field's type
        let new_value_name = Name::new("newValue".to_string(), span.clone());
        let new_value_param = CallableParameter::with_access_mode(
            ParameterAccessMode::Consuming,
            new_value_name,
            field_type,
        );

        // Return type is Unit for setters
        let return_type = Ty::unit(span.clone());

        // Determine receiver kind:
        // - Static setters have no receiver (None)
        // - Instance setters have Mutating receiver (they modify self)
        let callable = if is_static {
            CallableBehavior::new(vec![new_value_param], return_type, span)
        } else {
            CallableBehavior::with_receiver(
                vec![new_value_param],
                return_type,
                ReceiverKind::Mutating,
                span,
            )
        };

        symbol.metadata().add_behavior(callable);
    }

    fn bind_body(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Only process setter symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Setter {
            return;
        }

        // Check if this setter belongs to a subscript
        // If so, the SubscriptBinder will handle body binding (including parameters)
        if let Some(parent) = symbol.metadata().parent()
            && parent.metadata().kind() == KestrelSymbolKind::Subscript
        {
            // Skip - subscript setter bodies are handled by SubscriptBinder
            return;
        }

        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // Find the setter body - it's the CodeBlock inside the SetterClause
        // Syntax structure: SetterClause -> Set token, CodeBlock
        if let Some(body_node) = find_child(syntax, SyntaxKind::CodeBlock) {
            resolve_setter_body(symbol, &body_node, context, &source, file_id);
        }
    }
}

/// Resolve a setter's body and attach ExecutableBehavior to the symbol
fn resolve_setter_body(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    body_node: &SyntaxNode,
    context: &mut BindingContext,
    source: &str,
    file_id: usize,
) {
    use crate::body_resolver::BodyResolutionContext;
    use crate::body_resolver::context::{create_local_scope_for_body, resolve_code_block};
    use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;

    // Downcast to SetterSymbol
    let Some(_) = symbol.as_ref().downcast_ref::<SetterSymbol>() else {
        return;
    };

    // Create a local scope - setters use a dummy function for local storage
    // since SetterSymbol doesn't implement LocalContainer
    let mut local_scope = create_local_scope_for_body(symbol.clone(), "__setter_body_temp");

    // Get the CallableBehavior to access receiver and parameters
    let callable = symbol.metadata().get_behavior::<CallableBehavior>();
    let has_receiver = callable
        .as_ref()
        .map(|c| c.receiver().is_some())
        .unwrap_or(false);

    // If this is an instance setter, inject `self` as the first local (mutable)
    if has_receiver
        && let Some(self_type) = get_self_type(symbol)
    {
        let decl_span = symbol.metadata().span().clone();
        let self_span = Span::new(decl_span.file_id, decl_span.start..decl_span.start);

        // Add self to local scope (mutable - setters modify self)
        local_scope.bind("self".to_string(), self_type.clone(), true, self_span.clone());
    }

    // Add the `newValue` parameter to local scope
    if let Some(ref callable) = callable {
        for param in callable.parameters() {
            let param_ty = param.ty.clone();
            let param_name = param.bind_name.value.clone();
            let param_span = param.bind_name.span.clone();
            let is_mutable = match param.access_mode {
                ParameterAccessMode::Borrow => false,
                ParameterAccessMode::Mutating => true,
                ParameterAccessMode::Consuming => true,
            };
            local_scope.bind(param_name, param_ty, is_mutable, param_span);
        }
    }

    // Create body resolution context
    let mut body_ctx = BodyResolutionContext::new_with_scope(
        context.model,
        context.diagnostics,
        source,
        file_id,
        symbol.metadata().id(),
        local_scope,
        None, // Setters don't have their own where clause
    );

    // Resolve the code block and attach ExecutableBehavior
    let code_block = resolve_code_block(body_node, &mut body_ctx);
    let executable = ExecutableBehavior::new(code_block);
    symbol.metadata().add_behavior(executable);
}

/// Get the type of `self` for a setter
///
/// Returns the concrete type of the containing struct (grandparent of the setter).
/// The hierarchy is: Struct -> Field -> Setter
fn get_self_type(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<Ty> {
    use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
    use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
    use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
    use kestrel_semantic_tree::ty::Substitutions;

    // Setter's parent is Field, Field's parent is Struct/Extension
    let field = symbol.metadata().parent()?;
    let struct_parent = field.metadata().parent()?;
    let struct_span = struct_parent.metadata().span().clone();

    match struct_parent.metadata().kind() {
        KestrelSymbolKind::Struct => {
            // Create concrete struct type with type parameters mapping to themselves
            let struct_arc = Arc::clone(&struct_parent).downcast_arc::<StructSymbol>().ok()?;
            let mut substitutions = Substitutions::new();
            if let Some(generics) = struct_parent.metadata().get_behavior::<GenericsBehavior>() {
                for param in generics.type_parameters() {
                    let param_id = param.metadata().id();
                    let param_ty = Ty::type_parameter(param.clone(), struct_span.clone());
                    substitutions.insert(param_id, param_ty);
                }
            }
            Some(Ty::generic_struct(struct_arc, substitutions, struct_span))
        }
        KestrelSymbolKind::Extension => {
            // For extension properties, use the target type
            struct_parent
                .metadata()
                .get_behavior::<ExtensionTargetBehavior>()
                .map(|b| b.target_type().clone())
        }
        _ => None,
    }
}
