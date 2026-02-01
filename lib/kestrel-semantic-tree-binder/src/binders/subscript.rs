use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::{
    CallableBehavior, CallableParameter, ParameterAccessMode, ReceiverKind,
};
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::behavior::subscript::SubscriptBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::getter::GetterSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::setter::SetterSymbol;
use kestrel_semantic_tree::symbol::subscript::SubscriptSymbol;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::{Name, Span};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::resolution::type_resolver::{TypeSyntaxContext, resolve_type_from_ty_node};
use kestrel_syntax_tree::utils::find_child;

/// Binder for subscript declarations.
///
/// Subscripts are like computed properties but with parameters. They provide
/// indexed or keyed access to collection elements.
///
/// This binder:
/// 1. Resolves parameter types and return type
/// 2. Adds CallableBehavior to the getter child (params -> return type)
/// 3. Adds CallableBehavior to the setter child if present (params + newValue -> Unit)
/// 4. Binds getter and setter bodies with appropriate locals in scope
pub struct SubscriptBinder;

impl DeclarationBinder for SubscriptBinder {
    fn bind_signature(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Only process subscript symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Subscript {
            return;
        }

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

        // Resolve generics (type parameters and where clause)
        let generics_behavior = crate::binders::utils::generics::resolve_generics(
            syntax, &source, file_id, symbol_id, context,
        );
        symbol.metadata().add_behavior(generics_behavior);

        // Resolve parameters from syntax
        // Subscripts use implicit labels (parameter name becomes the external label)
        let resolved_params = crate::binders::utils::parameters::resolve_parameters_from_syntax(
            syntax, &source, file_id, symbol_id, context, false,
        );

        // Resolve return type from syntax
        let return_type =
            resolve_return_type_from_syntax(syntax, &source, file_id, symbol_id, context);

        // Add TypedBehavior with the return type (for member access resolution)
        let typed_behavior = TypedBehavior::new(return_type.clone(), span.clone());
        symbol.metadata().add_behavior(typed_behavior);

        // Downcast to SubscriptSymbol to access is_static and getter/setter
        let Some(subscript) = symbol.as_ref().downcast_ref::<SubscriptSymbol>() else {
            return;
        };

        let is_static = subscript.is_static();

        // Bind getter signature
        if let Some(getter) = subscript.getter() {
            bind_getter_signature(&getter, &resolved_params, &return_type, is_static, &span);
        }

        // Bind setter signature (if present)
        if let Some(setter) = subscript.setter() {
            bind_setter_signature(&setter, &resolved_params, &return_type, is_static, &span);
        }

        // Add SubscriptBehavior to the subscript symbol for overload resolution
        let callable_params: Vec<CallableParameter> = resolved_params
            .iter()
            .map(|p| CallableParameter {
                access_mode: p.access_mode,
                label: p.label.clone(),
                bind_name: p.bind_name.clone(),
                ty: p.ty.clone(),
                has_default: p.has_default,
            })
            .collect();

        let subscript_behavior = if is_static {
            SubscriptBehavior::new(callable_params, return_type)
        } else {
            SubscriptBehavior::with_receiver(callable_params, return_type, ReceiverKind::Borrowing)
        };
        symbol.metadata().add_behavior(subscript_behavior);
    }

    fn bind_body(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Only process subscript symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Subscript {
            return;
        }

        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // Downcast to SubscriptSymbol
        let Some(subscript) = symbol.as_ref().downcast_ref::<SubscriptSymbol>() else {
            return;
        };

        // Find the subscript body
        let Some(body_node) = find_child(syntax, SyntaxKind::SubscriptBody) else {
            // No body - this is a protocol requirement, nothing to bind
            return;
        };

        // Get the parameters from the getter's CallableBehavior
        let params = subscript
            .getter()
            .and_then(|g| g.metadata().get_behavior::<CallableBehavior>())
            .map(|cb| cb.parameters().to_vec())
            .unwrap_or_default();

        // Get the return type from TypedBehavior
        let return_type = symbol
            .metadata()
            .get_behavior::<TypedBehavior>()
            .map(|tb| tb.ty().clone())
            .unwrap_or_else(|| Ty::error(symbol.metadata().span().clone()));

        // Get where clause for body resolution
        let where_clause = symbol
            .metadata()
            .get_behavior::<GenericsBehavior>()
            .map(|g| g.where_clause().clone());

        // Bind getter body
        if let Some(getter) = subscript.getter() {
            let getter_body_node = find_getter_body(&body_node);
            if let Some(getter_body) = getter_body_node {
                resolve_getter_body(
                    symbol,
                    &getter,
                    &getter_body,
                    &params,
                    context,
                    &source,
                    file_id,
                    where_clause.clone(),
                );
            }
        }

        // Bind setter body (if present)
        if let Some(setter) = subscript.setter()
            && let Some(setter_body) = find_setter_body(&body_node)
        {
            resolve_setter_body(
                symbol,
                &setter,
                &setter_body,
                &params,
                &return_type,
                context,
                &source,
                file_id,
                where_clause,
            );
        }
    }
}

/// Bind the getter's signature (CallableBehavior)
///
/// The getter takes the subscript's parameters and returns the subscript's return type.
/// For instance subscripts, the receiver is Borrowing.
fn bind_getter_signature(
    getter: &Arc<GetterSymbol>,
    params: &[kestrel_semantic_tree::symbol::function::Parameter],
    return_type: &Ty,
    is_static: bool,
    span: &Span,
) {
    // Convert Parameters to CallableParameters
    let callable_params: Vec<CallableParameter> = params
        .iter()
        .map(|p| CallableParameter {
            access_mode: p.access_mode,
            label: p.label.clone(),
            bind_name: p.bind_name.clone(),
            ty: p.ty.clone(),
            has_default: p.has_default,
        })
        .collect();

    // Determine receiver kind
    let callable = if is_static {
        CallableBehavior::new(callable_params, return_type.clone(), span.clone())
    } else {
        CallableBehavior::with_receiver(
            callable_params,
            return_type.clone(),
            ReceiverKind::Borrowing,
            span.clone(),
        )
    };

    getter.metadata().add_behavior(callable);
}

/// Bind the setter's signature (CallableBehavior)
///
/// The setter takes the subscript's parameters plus a newValue parameter,
/// and returns Unit. For instance subscripts, the receiver is Mutating.
fn bind_setter_signature(
    setter: &Arc<SetterSymbol>,
    params: &[kestrel_semantic_tree::symbol::function::Parameter],
    return_type: &Ty,
    is_static: bool,
    span: &Span,
) {
    // Convert Parameters to CallableParameters, then add newValue
    let mut callable_params: Vec<CallableParameter> = params
        .iter()
        .map(|p| CallableParameter {
            access_mode: p.access_mode,
            label: p.label.clone(),
            bind_name: p.bind_name.clone(),
            ty: p.ty.clone(),
            has_default: p.has_default,
        })
        .collect();

    // Add the newValue parameter
    let new_value_name = Name::new("newValue".to_string(), span.clone());
    let new_value_param = CallableParameter::with_access_mode(
        ParameterAccessMode::Consuming,
        new_value_name,
        return_type.clone(),
    );
    callable_params.push(new_value_param);

    // Return type is Unit for setters
    let unit_return = Ty::unit(span.clone());

    // Determine receiver kind - setters mutate self
    let callable = if is_static {
        CallableBehavior::new(callable_params, unit_return, span.clone())
    } else {
        CallableBehavior::with_receiver(
            callable_params,
            unit_return,
            ReceiverKind::Mutating,
            span.clone(),
        )
    };

    setter.metadata().add_behavior(callable);
}

/// Find the getter body within a SubscriptBody node.
///
/// Handles both shorthand syntax (direct CodeBlock) and explicit syntax (GetterClause).
fn find_getter_body(body_node: &SyntaxNode) -> Option<SyntaxNode> {
    // Check for explicit getter: SubscriptBody -> PropertyAccessors -> GetterClause -> CodeBlock
    if let Some(accessors) = find_child(body_node, SyntaxKind::PropertyAccessors)
        && let Some(getter_clause) = find_child(&accessors, SyntaxKind::GetterClause)
    {
        return find_child(&getter_clause, SyntaxKind::CodeBlock);
    }

    // Shorthand syntax: SubscriptBody -> CodeBlock
    find_child(body_node, SyntaxKind::CodeBlock)
}

/// Find the setter body within a SubscriptBody node.
///
/// Only present in explicit syntax with SetterClause.
fn find_setter_body(body_node: &SyntaxNode) -> Option<SyntaxNode> {
    // Explicit setter: SubscriptBody -> PropertyAccessors -> SetterClause -> CodeBlock
    if let Some(accessors) = find_child(body_node, SyntaxKind::PropertyAccessors)
        && let Some(setter_clause) = find_child(&accessors, SyntaxKind::SetterClause)
    {
        return find_child(&setter_clause, SyntaxKind::CodeBlock);
    }

    None
}

/// Resolve a getter's body and attach ExecutableBehavior to the symbol
fn resolve_getter_body(
    subscript_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    getter: &Arc<GetterSymbol>,
    body_node: &SyntaxNode,
    params: &[kestrel_semantic_tree::symbol::function::Parameter],
    context: &mut BindingContext,
    source: &str,
    file_id: usize,
    where_clause: Option<kestrel_semantic_tree::ty::WhereClause>,
) {
    use crate::body_resolver::BodyResolutionContext;
    use crate::body_resolver::context::{create_local_scope_for_body, resolve_code_block};
    use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;

    // Create a local scope for the getter body
    let getter_dyn: Arc<dyn Symbol<KestrelLanguage>> = getter.clone();
    let mut local_scope =
        create_local_scope_for_body(getter_dyn.clone(), "__subscript_getter_temp");

    // Get receiver kind from CallableBehavior
    let callable = getter.metadata().get_behavior::<CallableBehavior>();
    let has_receiver = callable
        .as_ref()
        .map(|c| c.receiver().is_some())
        .unwrap_or(false);

    // If this is an instance getter, inject `self` as the first local (immutable)
    if has_receiver && let Some(self_type) = get_self_type(subscript_symbol) {
        let decl_span = getter.metadata().span().clone();
        let self_span = Span::new(decl_span.file_id, decl_span.start..decl_span.start);
        local_scope.bind("self".to_string(), self_type, false, self_span);
    }

    // Add parameters to local scope
    for param in params {
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

    // Create body resolution context
    let mut body_ctx = BodyResolutionContext::new_with_scope(
        context.model,
        context.diagnostics,
        source,
        file_id,
        getter.metadata().id(),
        local_scope,
        where_clause,
    );

    // Resolve the code block and attach ExecutableBehavior
    let code_block = resolve_code_block(body_node, &mut body_ctx);
    let executable = ExecutableBehavior::new(code_block);
    getter.metadata().add_behavior(executable);
}

/// Resolve a setter's body and attach ExecutableBehavior to the symbol
fn resolve_setter_body(
    subscript_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    setter: &Arc<SetterSymbol>,
    body_node: &SyntaxNode,
    params: &[kestrel_semantic_tree::symbol::function::Parameter],
    return_type: &Ty,
    context: &mut BindingContext,
    source: &str,
    file_id: usize,
    where_clause: Option<kestrel_semantic_tree::ty::WhereClause>,
) {
    use crate::body_resolver::BodyResolutionContext;
    use crate::body_resolver::context::{create_local_scope_for_body, resolve_code_block};
    use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;

    // Create a local scope for the setter body
    let setter_dyn: Arc<dyn Symbol<KestrelLanguage>> = setter.clone();
    let mut local_scope =
        create_local_scope_for_body(setter_dyn.clone(), "__subscript_setter_temp");

    // Get receiver kind from CallableBehavior
    let callable = setter.metadata().get_behavior::<CallableBehavior>();
    let has_receiver = callable
        .as_ref()
        .map(|c| c.receiver().is_some())
        .unwrap_or(false);

    // If this is an instance setter, inject `self` as the first local (mutable)
    if has_receiver && let Some(self_type) = get_self_type(subscript_symbol) {
        let decl_span = setter.metadata().span().clone();
        let self_span = Span::new(decl_span.file_id, decl_span.start..decl_span.start);
        local_scope.bind("self".to_string(), self_type, true, self_span);
    }

    // Add subscript parameters to local scope
    for param in params {
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

    // Add newValue parameter to local scope
    let setter_span = setter.metadata().span().clone();
    local_scope.bind(
        "newValue".to_string(),
        return_type.clone(),
        true, // newValue is mutable (consuming)
        setter_span,
    );

    // Create body resolution context
    let mut body_ctx = BodyResolutionContext::new_with_scope(
        context.model,
        context.diagnostics,
        source,
        file_id,
        setter.metadata().id(),
        local_scope,
        where_clause,
    );

    // Resolve the code block and attach ExecutableBehavior
    let code_block = resolve_code_block(body_node, &mut body_ctx);
    let executable = ExecutableBehavior::new(code_block);
    setter.metadata().add_behavior(executable);
}

/// Get the type of `self` for a subscript.
///
/// Returns the concrete type of the containing struct, enum, or extension target.
/// The hierarchy is: Struct/Extension/Enum -> Subscript
fn get_self_type(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<Ty> {
    use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
    use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
    use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
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
        KestrelSymbolKind::Enum => {
            // Create concrete enum type with type parameters mapping to themselves
            let enum_arc = Arc::clone(&parent).downcast_arc::<EnumSymbol>().ok()?;
            let mut substitutions = Substitutions::new();
            if let Some(generics) = parent.metadata().get_behavior::<GenericsBehavior>() {
                for param in generics.type_parameters() {
                    let param_id = param.metadata().id();
                    let param_ty = Ty::type_parameter(param.clone(), parent_span.clone());
                    substitutions.insert(param_id, param_ty);
                }
            }
            Some(Ty::generic_enum(enum_arc, substitutions, parent_span))
        },
        KestrelSymbolKind::Protocol => {
            // For protocol subscripts, Self remains abstract
            Some(Ty::self_type(parent_span))
        },
        KestrelSymbolKind::Extension => {
            // For extension subscripts, use the target type
            parent
                .metadata()
                .get_behavior::<ExtensionTargetBehavior>()
                .map(|b| b.target_type().clone())
        },
        _ => None,
    }
}

/// Resolve return type from a SubscriptDeclaration syntax node
fn resolve_return_type_from_syntax(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
) -> Ty {
    // Find the return type node: SubscriptDeclaration -> ReturnType -> Ty
    if let Some(return_type_node) = find_child(syntax, SyntaxKind::ReturnType)
        && let Some(ty_node) = find_child(&return_type_node, SyntaxKind::Ty)
    {
        let mut type_ctx =
            TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
        return resolve_type_from_ty_node(&ty_node, &mut type_ctx);
    }

    // No explicit return type - this is an error for subscripts
    let span = kestrel_syntax_tree::utils::get_node_span(syntax, file_id);
    Ty::error(span)
}
