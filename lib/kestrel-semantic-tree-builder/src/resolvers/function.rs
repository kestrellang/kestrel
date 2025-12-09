use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::{CallableBehavior, ReceiverKind};
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::{FunctionSymbol, Parameter};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Constraint, Ty, TyKind, WhereClause};
use kestrel_span::Spanned;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::database::TypePathResolution;
use crate::resolver::{BindingContext, Resolver};
use crate::resolvers::type_parameter::{add_type_params_as_children, extract_type_parameters};
use crate::resolution::type_resolver::{extract_type_from_ty_node, extract_type_from_node, resolve_type_from_ty_node, TypeSyntaxContext};
use crate::syntax::{
    extract_identifier_from_name, extract_name, extract_path_segments, extract_visibility, find_child,
    find_visibility_scope, get_file_id_for_symbol, get_node_span, get_visibility_span, parse_visibility,
};

/// Resolver for function declarations
pub struct FunctionResolver;

impl Resolver for FunctionResolver {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        // Extract name
        let name_str = extract_name(syntax)?;
        let name_node = find_child(syntax, SyntaxKind::Name)?;
        let name_span = get_node_span(&name_node, source);

        // Get full span
        let full_span = get_node_span(syntax, source);

        // Extract visibility
        let visibility_str = extract_visibility(syntax);
        let visibility_enum = visibility_str.as_deref().and_then(parse_visibility);

        let visibility_span = get_visibility_span(syntax, source).unwrap_or(name_span.clone());

        // Determine visibility scope
        let visibility_scope = find_visibility_scope(visibility_enum.as_ref(), parent, root);

        // Create visibility behavior
        let visibility_behavior =
            VisibilityBehavior::new(visibility_enum, visibility_span, visibility_scope);

        // Check if this function is static
        let is_static = syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::StaticModifier);

        // Check if this function has a body
        let has_body = syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::FunctionBody);

        // Extract parameters
        let parameters = extract_parameters(syntax, source);

        // Extract return type
        let return_type = extract_return_type(syntax, source);

        // Create the name object
        let name = Spanned::new(name_str, name_span);

        // Extract type parameters (they'll have function as parent later)
        let type_parameters = extract_type_parameters(syntax, source, parent.cloned());

        // Create the function symbol (GenericsBehavior is added during BIND)
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

        // Add type parameters as children of the function (not the module)
        // This ensures type parameters are in scope during type resolution
        add_type_params_as_children(&type_parameters, &function_arc_dyn);

        // Add to parent if exists
        if let Some(parent) = parent {
            parent.metadata().add_child(&function_arc_dyn);
        }

        Some(function_arc)
    }

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

        // Get file_id and source for this symbol
        let (file_id, source) = context.get_file_context(symbol);

        // Extract type parameters and resolve where clause bounds FIRST
        // This must happen before resolving parameter/return types so that
        // T.Item paths can find the protocol bounds for T
        let generics_behavior = resolve_generics(syntax, &source, symbol_id, context);
        symbol.metadata().add_behavior(generics_behavior);

        // Now extract and resolve parameters from syntax (T.Item will work)
        let resolved_params = resolve_parameters_from_syntax(syntax, &source, symbol_id, context, file_id);

        // Extract and resolve return type from syntax (T.Item will work)
        let resolved_return = resolve_return_type_from_syntax(syntax, &source, symbol_id, context, file_id);

        // Determine receiver kind for instance methods
        let receiver_kind = determine_receiver_kind(syntax, symbol);

        // Add a new CallableBehavior with resolved types
        let resolved_callable = match receiver_kind {
            Some(kind) => CallableBehavior::with_receiver(resolved_params.clone(), resolved_return, kind, span),
            None => CallableBehavior::new(resolved_params.clone(), resolved_return, span),
        };
        symbol.metadata().add_behavior(resolved_callable);

        // Resolve function body if present
        if let Some(body_node) = find_child(syntax, SyntaxKind::FunctionBody) {
            resolve_function_body(symbol, &body_node, &resolved_params, context, file_id, &source);
        }
    }
}

/// Extract type parameters and resolve where clause bounds, creating a GenericsBehavior.
fn resolve_generics(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
) -> GenericsBehavior {
    // Re-extract type parameters (they were already extracted during BUILD and added as children)
    // We need to get them from the symbol's children
    let symbol = match ctx.db.symbol_by_id(context_id) {
        Some(s) => s,
        None => return GenericsBehavior::empty(),
    };

    let type_parameters: Vec<Arc<TypeParameterSymbol>> = symbol
        .metadata()
        .children()
        .into_iter()
        .filter_map(|child| {
            if child.metadata().kind() == KestrelSymbolKind::TypeParameter {
                child.downcast_arc::<TypeParameterSymbol>().ok()
            } else {
                None
            }
        })
        .collect();

    // Now resolve the where clause with fully resolved protocol types
    let where_clause = resolve_where_clause(syntax, source, context_id, ctx, &type_parameters);

    GenericsBehavior::new(type_parameters, where_clause)
}

/// Resolve where clause from syntax, returning a WhereClause with resolved protocol types.
fn resolve_where_clause(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    type_params: &[Arc<TypeParameterSymbol>],
) -> WhereClause {
    let where_clause_node = match find_child(syntax, SyntaxKind::WhereClause) {
        Some(node) => node,
        None => return WhereClause::new(),
    };

    let mut constraints = Vec::new();

    // Get file_id for diagnostics
    let file_id = ctx.db.symbol_by_id(context_id)
        .map(|s| get_file_id_for_symbol(&s, ctx.diagnostics))
        .unwrap_or(0);

    for child in where_clause_node.children() {
        if child.kind() == SyntaxKind::TypeBound {
            if let Some(constraint) = resolve_type_bound(&child, source, context_id, ctx, type_params, file_id, &constraints) {
                constraints.push(constraint);
            }
        }
    }

    WhereClause::with_constraints(constraints)
}

/// Resolve a single TypeBound, resolving protocol paths to actual types.
///
/// Handles both simple bounds (T: Protocol) and associated type bounds (T.Item: Protocol).
fn resolve_type_bound(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    type_params: &[Arc<TypeParameterSymbol>],
    file_id: usize,
    already_collected: &[Constraint],
) -> Option<Constraint> {
    use crate::diagnostics::WhereClauseAssociatedTypeNotFoundError;

    // Check for AssociatedTypeTarget first (T.Item: Protocol syntax)
    if let Some(target_node) = find_child(syntax, SyntaxKind::AssociatedTypeTarget) {
        // Extract path segments from the target (e.g., ["T", "Item"])
        let path_segments = extract_path_from_node(&target_node);

        if path_segments.len() >= 2 {
            let type_param_name = &path_segments[0];
            let assoc_type_name = &path_segments[1];

            // Find the type parameter
            let type_param = type_params
                .iter()
                .find(|p| &p.metadata().name().value == type_param_name);

            if let Some(type_param) = type_param {
                let param_id = type_param.metadata().id();

                // Get protocol bounds from already-collected constraints
                let bounds: Vec<&Ty> = already_collected
                    .iter()
                    .filter_map(|c| {
                        if c.param_id() == Some(param_id) {
                            match c {
                                Constraint::TypeBound { bounds, .. } => Some(bounds.iter().collect::<Vec<_>>()),
                                // InheritedAssociatedTypeBound has param_id() = None, so won't match
                                Constraint::InheritedAssociatedTypeBound { .. } => None,
                            }
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect();

                // Check if any bound protocol has this associated type
                let mut found_assoc_type = false;
                let mut protocol_name = String::new();

                for bound in &bounds {
                    if let TyKind::Protocol { symbol, .. } = bound.kind() {
                        protocol_name = symbol.metadata().name().value.clone();
                        let protocol_dyn = symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>;

                        // Check direct children for the associated type
                        let has_type = protocol_dyn.metadata().children().iter().any(|child| {
                            child.metadata().kind() == KestrelSymbolKind::AssociatedType
                                && &child.metadata().name().value == assoc_type_name
                        });

                        if has_type {
                            found_assoc_type = true;
                            break;
                        }
                    }
                }

                if !found_assoc_type && !bounds.is_empty() {
                    let target_span = get_node_span(&target_node, source);
                    ctx.diagnostics.throw(
                        WhereClauseAssociatedTypeNotFoundError {
                            span: target_span,
                            type_param: type_param_name.clone(),
                            assoc_type_name: assoc_type_name.clone(),
                            protocol_name,
                        },
                        file_id,
                    );
                }
            }
        }

        // Associated type bounds don't create constraints in the same way
        // They're validated above but don't need to be added to the constraint list
        return None;
    }

    // Simple bound: T: Protocol
    let name_node = find_child(syntax, SyntaxKind::Name)?;
    let name_token = name_node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)?;

    let param_name = name_token.text().to_string();
    let text_range = name_token.text_range();
    let param_span: kestrel_span::Span = (text_range.start().into())..(text_range.end().into());

    // Look up the type parameter (may be None if undeclared)
    let param_id = type_params
        .iter()
        .find(|p| p.metadata().name().value == param_name)
        .map(|p| p.metadata().id());

    // Resolve each Path to a protocol type
    let bounds: Vec<Ty> = syntax
        .children()
        .filter(|c| c.kind() == SyntaxKind::Path)
        .map(|path_node| {
            let span = get_node_span(&path_node, source);
            let segments = extract_path_segments(&path_node);

            if segments.is_empty() {
                return Ty::error(span);
            }

            // Resolve the path to a type
            match ctx.db.resolve_type_path(segments, context_id) {
                TypePathResolution::Resolved(resolved_ty) => {
                    if let TyKind::Protocol { .. } = resolved_ty.kind() {
                        resolved_ty
                    } else {
                        // Not a protocol - error already reported by validation
                        Ty::error(span)
                    }
                }
                TypePathResolution::NotFound { .. } => {
                    // Error already reported during general type resolution
                    Ty::error(span)
                }
                _ => Ty::error(span),
            }
        })
        .collect();

    if bounds.is_empty() {
        None
    } else {
        match param_id {
            Some(id) => Some(Constraint::type_bound(id, param_name, param_span, bounds)),
            None => Some(Constraint::unresolved_type_bound(param_name, param_span, bounds)),
        }
    }
}

/// Extract path segments from an AssociatedTypeTarget or Path node
fn extract_path_from_node(node: &SyntaxNode) -> Vec<String> {
    let mut segments = Vec::new();

    // Try to find a Path child
    if let Some(path_node) = find_child(node, SyntaxKind::Path) {
        for child in path_node.children() {
            if child.kind() == SyntaxKind::PathElement {
                for elem in child.children_with_tokens() {
                    if let Some(token) = elem.into_token() {
                        if token.kind() == SyntaxKind::Identifier {
                            segments.push(token.text().to_string());
                        }
                    }
                }
            }
        }
    }

    segments
}

/// Resolve a function's body and attach ExecutableBehavior to the symbol
fn resolve_function_body(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    body_node: &SyntaxNode,
    params: &[Parameter],
    context: &mut BindingContext,
    file_id: usize,
    source: &str,
) {
    use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
    use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
    use kestrel_semantic_tree::symbol::function::FunctionSymbol;
    use crate::body_resolver::{BodyResolutionContext, resolve_function_body as resolve_body};

    // Downcast to FunctionSymbol to get Arc<FunctionSymbol> for LocalScope
    let Some(func_sym) = symbol.as_ref().downcast_ref::<FunctionSymbol>() else {
        return;
    };

    // Create LocalScope with the function symbol
    // We need to create an Arc<FunctionSymbol>, but we only have &FunctionSymbol
    // The workaround is to get the symbol from the db
    let Some(func_arc) = context.db.symbol_by_id(symbol.metadata().id()) else {
        return;
    };

    // Verify it's a FunctionSymbol (already confirmed above)
    if func_arc.as_ref().downcast_ref::<FunctionSymbol>().is_none() {
        return;
    }

    // Create a temporary FunctionSymbol that we can use with LocalScope
    // This is needed because LocalScope::new takes Arc<FunctionSymbol>
    // The locals will be added to the actual function through the Arc<dyn Symbol>
    use kestrel_semantic_tree::behavior::visibility::{Visibility, VisibilityBehavior};
    use kestrel_span::Spanned;

    let temp_name = Spanned::new("__body_temp".to_string(), 0..0);
    let temp_vis = VisibilityBehavior::new(Some(Visibility::Private), 0..0, func_arc.clone());
    let temp_func = Arc::new(FunctionSymbol::new(
        temp_name,
        0..0,
        temp_vis,
        true,
        true,
        vec![],
        kestrel_semantic_tree::ty::Ty::unit(0..0),
        None,
    ));

    let mut local_scope = crate::resolution::LocalScope::new(temp_func);

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
            let self_span = symbol.metadata().span().start..symbol.metadata().span().start;

            // Add self to local scope
            local_scope.bind("self".to_string(), self_type.clone(), is_mutable, self_span.clone());
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
        local_scope.bind(param_name.clone(), param_ty.clone(), false, param_span.clone());
        // Add to the actual function symbol
        func_sym.add_local(param_name, param_ty, false, param_span);
    }

    // Create body resolution context
    let mut body_ctx = BodyResolutionContext {
        db: context.db,
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

    // Transfer locals from temp function to real function
    // (locals created during body resolution need to be added to the real function)
    // The temp function's locals are tracked separately, so we need to sync them

    // Create and attach ExecutableBehavior
    let executable = ExecutableBehavior::new(code_block);
    symbol.metadata().add_behavior(executable);
}

/// Extract parameters from a FunctionDeclaration syntax node
fn extract_parameters(syntax: &SyntaxNode, source: &str) -> Vec<Parameter> {
    // Find the ParameterList node
    let param_list = match find_child(syntax, SyntaxKind::ParameterList) {
        Some(node) => node,
        None => return Vec::new(),
    };

    // Extract each Parameter node
    param_list
        .children()
        .filter(|child| child.kind() == SyntaxKind::Parameter)
        .filter_map(|param_node| extract_single_parameter(&param_node, source))
        .collect()
}

/// Extract a single parameter from a Parameter syntax node
///
/// Parameter structure:
/// - Parameter
///   - Name (label, optional - if there are 2 Name nodes, first is label)
///   - Name (bind_name)
///   - Colon
///   - Ty
fn extract_single_parameter(param_node: &SyntaxNode, source: &str) -> Option<Parameter> {
    // Collect all Name nodes
    let name_nodes: Vec<SyntaxNode> = param_node
        .children()
        .filter(|child| child.kind() == SyntaxKind::Name)
        .collect();

    if name_nodes.is_empty() {
        return None;
    }

    // Extract name strings and spans
    let (label, bind_name) = if name_nodes.len() >= 2 {
        // Two names: first is label, second is bind_name
        let label_str = extract_identifier_from_name(&name_nodes[0])?;
        let label_span = get_node_span(&name_nodes[0], source);
        let label = Spanned::new(label_str, label_span);

        let bind_str = extract_identifier_from_name(&name_nodes[1])?;
        let bind_span = get_node_span(&name_nodes[1], source);
        let bind_name = Spanned::new(bind_str, bind_span);

        (Some(label), bind_name)
    } else {
        // One name: it's the bind_name
        let bind_str = extract_identifier_from_name(&name_nodes[0])?;
        let bind_span = get_node_span(&name_nodes[0], source);
        let bind_name = Spanned::new(bind_str, bind_span);

        (None, bind_name)
    };

    // Extract the type
    let ty = extract_type_from_node(param_node, source);

    Some(match label {
        Some(l) => Parameter::with_label(l, bind_name, ty),
        None => Parameter::new(bind_name, ty),
    })
}

/// Extract return type from a FunctionDeclaration syntax node
fn extract_return_type(syntax: &SyntaxNode, source: &str) -> Ty {
    // Find the ReturnType node
    if let Some(ret_node) = find_child(syntax, SyntaxKind::ReturnType) {
        // Get the Ty child
        if let Some(ty_node) = find_child(&ret_node, SyntaxKind::Ty) {
            return extract_type_from_ty_node(&ty_node, source);
        }
    }

    // Default to unit type if no return type specified
    let fn_span = get_node_span(syntax, source);
    Ty::unit(fn_span.end..fn_span.end)
}

/// Resolve parameters from a FunctionDeclaration syntax node during bind phase
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

    // Determine label and bind_name based on number of Name nodes
    let (label, bind_name) = if name_nodes.len() >= 2 {
        // Two names: first is label, second is bind_name
        let label_name = extract_identifier_from_name(&name_nodes[0]);
        let bind_name = Spanned::new(
            extract_identifier_from_name(&name_nodes[1])?,
            get_node_span(&name_nodes[1], source),
        );
        (label_name.map(|n| Spanned::new(n, get_node_span(&name_nodes[0], source))), bind_name)
    } else {
        // One name: no label, it's the bind_name
        let bind_name = Spanned::new(
            extract_identifier_from_name(&name_nodes[0])?,
            get_node_span(&name_nodes[0], source),
        );
        (None, bind_name)
    };

    // Find and resolve the type from Ty node using shared utility
    let ty = if let Some(ty_node) = param_node.children().find(|c| c.kind() == SyntaxKind::Ty) {
        let mut type_ctx = TypeSyntaxContext::new(ctx.db, ctx.diagnostics, file_id, source, context_id);
        resolve_type_from_ty_node(&ty_node, &mut type_ctx)
    } else {
        // No type annotation - inferred
        Ty::inferred(get_node_span(param_node, source))
    };

    Some(Parameter { label, bind_name, ty })
}

/// Resolve return type from a FunctionDeclaration syntax node during bind phase
fn resolve_return_type_from_syntax(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    file_id: usize,
) -> Ty {
    // Find the return type node: FunctionDeclaration -> ReturnType -> Ty
    if let Some(return_type_node) = find_child(syntax, SyntaxKind::ReturnType) {
        if let Some(ty_node) = find_child(&return_type_node, SyntaxKind::Ty) {
            let mut type_ctx = TypeSyntaxContext::new(ctx.db, ctx.diagnostics, file_id, source, context_id);
            return resolve_type_from_ty_node(&ty_node, &mut type_ctx);
        }
    }

    // No explicit return type - defaults to unit
    let fn_span = get_node_span(syntax, source);
    Ty::unit(fn_span.end..fn_span.end)
}

/// Get the type of `self` for an instance method
///
/// Returns the type of the containing struct or protocol.
/// For now, we use `Self` type which will be resolved later.
fn get_self_type(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<Ty> {
    let parent = symbol.metadata().parent()?;
    let parent_span = parent.metadata().span().clone();

    match parent.metadata().kind() {
        KestrelSymbolKind::Struct | KestrelSymbolKind::Protocol => {
            // Use Self type which refers to the containing type
            // This will be resolved to the concrete type during type checking
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

    // Check if the function is in a struct or protocol (instance method)
    let parent_kind = symbol.metadata().parent().map(|p| p.metadata().kind());
    let is_instance_method = matches!(
        parent_kind,
        Some(KestrelSymbolKind::Struct) | Some(KestrelSymbolKind::Protocol)
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
