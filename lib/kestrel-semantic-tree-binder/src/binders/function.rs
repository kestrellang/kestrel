use std::sync::Arc;

use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;
use kestrel_semantic_tree::behavior::callable::{
    CallableBehavior, ParameterAccessMode, ReceiverKind,
};
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::builtins::{BuiltinKind, LanguageFeature};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::Parameter;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::binders::utils::attributes::{
    BuiltinParseResult, ExternParseResult, parse_builtin_attribute, parse_extern_attribute,
};
use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::diagnostics::{
    BuiltinMethodNotInProtocolError, BuiltinMethodWrongSignatureError, BuiltinWrongKindError,
    DuplicateBuiltinError, ExternFunctionCannotBeGenericError, ExternFunctionCannotHaveBodyError,
    ExternParameterNotConsumingError, TypeNotFFISafeError,
};
use crate::resolution::LocalScope;
use crate::resolution::type_resolver::{TypeSyntaxContext, resolve_type_from_ty_node};
use kestrel_semantic_tree::attributes::AttributeKind;
use kestrel_semantic_tree::behavior::extern_fn::ExternBehavior;
use kestrel_semantic_type_inference::TypeOracle;
use kestrel_syntax_tree::utils::{find_child, get_node_span};

/// Binder for function declarations
pub struct FunctionBinder;

impl FunctionBinder {
    /// Process @builtin attribute on a function.
    fn process_builtin_attribute(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        attributes: &AttributesBehavior,
        source: &str,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        let feature = match parse_builtin_attribute(attributes, source, context.diagnostics) {
            BuiltinParseResult::Success(f) => f,
            BuiltinParseResult::NotBuiltin | BuiltinParseResult::Error => return,
        };

        let definition = feature.definition();
        let attr_span = attributes
            .get_kind(kestrel_semantic_tree::attributes::AttributeKind::Builtin)
            .map(|a| a.span.clone())
            .unwrap_or_else(|| symbol.metadata().span().clone());

        let symbol_id = symbol.metadata().id();

        // Check if this is a protocol method builtin
        if let BuiltinKind::ProtocolMethod { protocol_feature } = &definition.kind {
            Self::process_protocol_method_builtin(
                symbol,
                feature,
                *protocol_feature,
                attr_span,
                syntax,
                source,
                context,
            );
            return;
        }

        // Validate: feature must expect a function
        if !definition.kind.is_function() {
            context.diagnostics.throw(BuiltinWrongKindError {
                span: attr_span,
                feature_name: feature.name().to_string(),
                expected_kind: definition.kind.kind_name().to_string(),
                actual_kind: "function".to_string(),
            });
            return;
        }

        // Register the builtin
        if !context
            .model
            .builtin_registry()
            .register_function(feature, symbol_id)
        {
            context.diagnostics.throw(DuplicateBuiltinError {
                span: attr_span,
                feature_name: feature.name().to_string(),
            });
        }
    }

    /// Process @builtin attribute for a protocol method (e.g., @builtin(.Clone)).
    fn process_protocol_method_builtin(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        feature: LanguageFeature,
        protocol_feature: LanguageFeature,
        attr_span: Span,
        syntax: &SyntaxNode,
        source: &str,
        context: &mut BindingContext,
    ) {
        let symbol_id = symbol.metadata().id();

        // Validate: parent must be a protocol with @builtin matching the required protocol feature
        let parent = symbol.metadata().parent();
        let parent_is_correct_builtin_protocol = parent
            .as_ref()
            .filter(|p| p.metadata().kind() == KestrelSymbolKind::Protocol)
            .and_then(|p| {
                context
                    .model
                    .builtin_registry()
                    .protocol_feature(p.metadata().id())
            })
            .map(|pf| pf == protocol_feature)
            .unwrap_or(false);

        if !parent_is_correct_builtin_protocol {
            context.diagnostics.throw(BuiltinMethodNotInProtocolError {
                span: attr_span,
                method_feature: feature.name().to_string(),
                required_protocol_feature: protocol_feature.name().to_string(),
            });
            return;
        }

        // Validate signature based on the specific feature
        if let Err(expected_signature) =
            Self::validate_protocol_method_signature(feature, symbol, syntax, source)
        {
            context.diagnostics.throw(BuiltinMethodWrongSignatureError {
                span: attr_span,
                method_feature: feature.name().to_string(),
                expected_signature,
            });
            return;
        }

        // Register the builtin method
        if !context
            .model
            .builtin_registry()
            .register_method(feature, symbol_id)
        {
            context.diagnostics.throw(DuplicateBuiltinError {
                span: attr_span,
                feature_name: feature.name().to_string(),
            });
        }
    }

    /// Validate the signature for a builtin protocol method.
    /// Returns Ok(()) if valid, Err(expected_signature) if invalid.
    fn validate_protocol_method_signature(
        feature: LanguageFeature,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        source: &str,
    ) -> Result<(), String> {
        match feature {
            LanguageFeature::Clone => Self::validate_clone_signature(symbol, syntax, source),
            // Add more protocol method validations here as needed
            _ => Ok(()), // Unknown method features pass validation by default
        }
    }

    /// Validate the signature for @builtin(.Clone): `func clone() -> Self`
    ///
    /// Requirements:
    /// - Must be an instance method (has a receiver)
    /// - Must take no parameters (self is implicit)
    /// - Must return Self
    fn validate_clone_signature(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        source: &str,
    ) -> Result<(), String> {
        let expected = "func clone() -> Self".to_string();

        // Check for receiver (must be an instance method)
        let is_static = syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::StaticModifier);

        if is_static {
            return Err(expected);
        }

        // Check parent is a protocol (instance method context)
        let parent_kind = symbol.metadata().parent().map(|p| p.metadata().kind());
        if !matches!(parent_kind, Some(KestrelSymbolKind::Protocol)) {
            return Err(expected);
        }

        // Check parameters: should have no explicit parameters (only implicit self)
        // In protocol methods, `self` is implicit for instance methods
        if let Some(params_node) = find_child(syntax, SyntaxKind::ParameterList) {
            let param_count = params_node
                .children()
                .filter(|c| c.kind() == SyntaxKind::Parameter)
                .count();

            if param_count > 0 {
                return Err(expected);
            }
        }

        // Check return type: must be Self
        if let Some(return_type_node) = find_child(syntax, SyntaxKind::ReturnType) {
            if let Some(ty_node) = find_child(&return_type_node, SyntaxKind::Ty) {
                // Check if it's a TyPath containing just "Self"
                if let Some(ty_path) = ty_node.children().find(|c| c.kind() == SyntaxKind::TyPath) {
                    // Get the text of the type path
                    let start: usize = ty_path.text_range().start().into();
                    let end: usize = ty_path.text_range().end().into();
                    let type_text = source[start..end].trim();

                    if type_text != "Self" {
                        return Err(expected);
                    }
                } else {
                    // Not a TyPath (could be tuple, function, etc.)
                    return Err(expected);
                }
            } else {
                // No Ty node in return type
                return Err(expected);
            }
        } else {
            // No return type specified (returns Unit, not Self)
            return Err(expected);
        }

        Ok(())
    }

    /// Process @extern attribute on a function.
    ///
    /// Validates that:
    /// - The function is not generic
    /// - The function has no body (implementation is external)
    ///
    /// If valid, attaches an ExternBehavior to the symbol.
    fn process_extern_attribute(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        attributes: &AttributesBehavior,
        source: &str,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        let result = parse_extern_attribute(attributes, source, context.diagnostics);

        let (calling_convention, mangle_name) = match result {
            ExternParseResult::Success {
                calling_convention,
                mangle_name,
            } => (calling_convention, mangle_name),
            ExternParseResult::NotExtern | ExternParseResult::Error => return,
        };

        let attr_span = attributes
            .get_kind(AttributeKind::Extern)
            .map(|a| a.span.clone())
            .unwrap_or_else(|| symbol.metadata().span().clone());

        // Validation 1: Cannot be generic
        if let Some(generics) = symbol.metadata().get_behavior::<GenericsBehavior>() {
            if generics.is_generic() {
                context
                    .diagnostics
                    .throw(ExternFunctionCannotBeGenericError {
                        span: attr_span.clone(),
                    });
                return;
            }
        }

        // Validation 2: Cannot have a body
        // Check for FunctionBody in syntax (non-empty braces or expression body)
        if let Some(body_node) = find_child(syntax, SyntaxKind::FunctionBody) {
            // Check if body has actual content
            // A FunctionBody contains either a CodeBlock or a single Expression
            // An empty body `{}` has an empty CodeBlock
            let has_content = body_node.children().any(|child| {
                match child.kind() {
                    SyntaxKind::CodeBlock => {
                        // Check if the code block has any statements or trailing expression
                        child.children().any(|grandchild| {
                            matches!(
                                grandchild.kind(),
                                SyntaxKind::Statement | SyntaxKind::Expression
                            )
                        })
                    }
                    SyntaxKind::Expression => true, // Expression body like `func foo() -> Int = 42`
                    _ => false,
                }
            });

            if has_content {
                context
                    .diagnostics
                    .throw(ExternFunctionCannotHaveBodyError { span: attr_span });
                return;
            }
        }

        // Validation 3 (mutating param check) is now done in bind_members before
        // CallableBehavior is created, so we can check the original access modes.

        // Validation 4: All parameter types and return type must conform to FFISafe
        if let Some(ffi_safe_id) = context
            .model
            .builtin_registry()
            .protocol(LanguageFeature::FFISafe)
        {
            if let Some(callable) = symbol.metadata().get_behavior::<CallableBehavior>() {
                // Check each parameter type
                for param in callable.parameters() {
                    if !context.model.conforms_to(&param.ty, ffi_safe_id) {
                        context.diagnostics.throw(TypeNotFFISafeError {
                            span: param.ty.span().clone(),
                            ty: param.ty.to_string(),
                            context: "parameter".to_string(),
                        });
                    }
                }

                // Check return type (skip if Unit - void is always valid for extern)
                let return_ty = callable.return_type();
                if !return_ty.is_unit() && !context.model.conforms_to(return_ty, ffi_safe_id) {
                    context.diagnostics.throw(TypeNotFFISafeError {
                        span: return_ty.span().clone(),
                        ty: return_ty.to_string(),
                        context: "return type".to_string(),
                    });
                }
            }
        }

        // Attach ExternBehavior to the symbol
        let extern_behavior = ExternBehavior::new(calling_convention, mangle_name);
        symbol.metadata().add_behavior(extern_behavior);
    }
}

impl DeclarationBinder for FunctionBinder {
    fn bind_signature(
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

        // Resolve attributes
        let attributes_behavior = crate::binders::utils::attributes::resolve_attributes(
            syntax,
            &source,
            file_id,
            context.diagnostics,
        );
        symbol.metadata().add_behavior(attributes_behavior.clone());

        // Process @builtin attribute if present
        Self::process_builtin_attribute(symbol, &attributes_behavior, &source, syntax, context);

        // Extract type parameters and resolve where clause bounds FIRST
        // This must happen before resolving parameter/return types so that
        // T.Item paths can find the protocol bounds for T
        let generics_behavior = crate::binders::utils::generics::resolve_generics(
            syntax, &source, file_id, symbol_id, context,
        );
        symbol.metadata().add_behavior(generics_behavior);

        // Now extract and resolve parameters from syntax (T.Item will work)
        let resolved_params = crate::binders::utils::parameters::resolve_parameters_from_syntax(
            syntax, &source, file_id, symbol_id, context, false,
        );

        // Extract and resolve return type from syntax (T.Item will work)
        let resolved_return =
            resolve_return_type_from_syntax(syntax, &source, file_id, symbol_id, context);

        // Determine receiver kind for instance methods
        let receiver_kind = determine_receiver_kind(syntax, symbol);

        // Check if this is an extern function - extern functions always use consuming params
        // because FFI can't handle Kestrel's borrowing semantics
        let is_extern = attributes_behavior
            .get_kind(AttributeKind::Extern)
            .is_some();

        // For extern functions, validate that no parameter uses 'mutating' access mode,
        // then force all parameters to Consuming access mode
        let resolved_params = if is_extern {
            // Validate: error if user explicitly wrote 'mutating'
            for param in &resolved_params {
                if param.access_mode == ParameterAccessMode::Mutating {
                    context.diagnostics.throw(ExternParameterNotConsumingError {
                        span: param.bind_name.span.clone(),
                        param_name: param.bind_name.value.clone(),
                    });
                }
            }

            // Transform: force all params to Consuming for FFI compatibility
            resolved_params
                .into_iter()
                .map(|mut p| {
                    p.access_mode = ParameterAccessMode::Consuming;
                    p
                })
                .collect()
        } else {
            resolved_params
        };

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

        // Process @extern attribute if present (must be after CallableBehavior is added
        // so we can check parameter types and access modes)
        Self::process_extern_attribute(symbol, &attributes_behavior, &source, syntax, context);

        // NOTE: Body resolution is deferred to bind_body() to handle forward references
    }

    fn bind_body(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Only process function symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Function {
            return;
        }

        // Get the CallableBehavior to extract resolved parameters
        let Some(callable) = symbol.metadata().get_behavior::<CallableBehavior>() else {
            return;
        };
        let resolved_params = callable.parameters().to_vec();

        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

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
    use crate::body_resolver::BodyResolutionContext;
    use crate::body_resolver::context::{
        create_local_scope_for_body, resolve_body_and_attach_executable,
    };
    use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
    use kestrel_semantic_tree::symbol::function::FunctionSymbol;

    // Downcast to FunctionSymbol to get Arc<FunctionSymbol> for LocalScope
    let Some(func_sym) = symbol.as_ref().downcast_ref::<FunctionSymbol>() else {
        return;
    };

    let mut local_scope = if let Ok(func) = symbol.clone().downcast_arc::<FunctionSymbol>() {
        LocalScope::new(func)
    } else {
        create_local_scope_for_body(symbol.clone(), "__body_temp")
    };

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
        }
    }

    // Add parameters to local scope
    // Mutability depends on access mode:
    // - Borrow: immutable (read-only)
    // - Mutating: mutable (read-write, but caller keeps ownership)
    // - Consuming: mutable (takes ownership, can modify)
    for param in params {
        use kestrel_semantic_tree::behavior::callable::ParameterAccessMode;
        let param_ty = param.ty.clone();
        let param_name = param.bind_name.value.clone();
        let param_span = param.bind_name.span.clone();
        let is_mutable = match param.access_mode {
            ParameterAccessMode::Borrow => false,
            ParameterAccessMode::Mutating => true,
            ParameterAccessMode::Consuming => true,
        };
        // Add to local scope (this also adds it to the FunctionSymbol's locals)
        local_scope.bind(param_name, param_ty, is_mutable, param_span);
    }

    // Get the where clause from the function's generics behavior
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

    // No explicit return type - default to Unit
    let fn_span = get_node_span(syntax, file_id);
    Ty::unit(Span::new(fn_span.file_id, fn_span.end..fn_span.end))
}

/// Get the type of `self` for an instance method
///
/// Returns the type of the containing struct, protocol, or extension target.
/// For extensions, we use Self type which will resolve to the target type.
fn get_self_type(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<Ty> {
    let parent = symbol.metadata().parent()?;
    let parent_span = parent.metadata().span().clone();

    match parent.metadata().kind() {
        KestrelSymbolKind::Struct | KestrelSymbolKind::Enum | KestrelSymbolKind::Protocol => {
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
            | Some(KestrelSymbolKind::Enum)
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
