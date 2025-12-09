use std::sync::Arc;

use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::associated_type::{AssociatedTypeSymbol, AssociatedTypeBoundsBehavior};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_alias::{TypeAliasSymbol, TypeAliasTypedBehavior};
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Constraint, Ty, TyKind, WhereClause};
use kestrel_span::Spanned;
use kestrel_syntax_tree::{SyntaxElement, SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::database::TypePathResolution;
use crate::diagnostics::{
    AssociatedTypeBoundsInWrongContextError, NotAProtocolContext, NotAProtocolError,
    TypeAliasContext as DiagTypeAliasContext, TypeAliasRequiresTypeError, UnresolvedTypeError,
};
use crate::resolver::{BindingContext, Resolver};
use crate::resolvers::type_parameter::{add_type_params_as_children, extract_type_parameters};
use crate::resolution::type_resolver::{resolve_type_from_ty_node, TypeSyntaxContext};
use crate::syntax::{
    extract_name, extract_path_segments, extract_visibility, find_child, find_visibility_scope,
    get_file_id_for_symbol, get_node_span, get_visibility_span, parse_visibility,
};

/// Determines the context in which a type alias declaration appears
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeAliasContext {
    /// In a protocol body - creates AssociatedTypeSymbol
    Protocol,
    /// In a struct body - creates TypeAliasSymbol (associated type binding)
    Struct,
    /// At module/file level - creates regular TypeAliasSymbol
    Module,
}

/// Resolver for type alias declarations
pub struct TypeAliasResolver;

impl Resolver for TypeAliasResolver {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        // Determine context based on parent
        let context = determine_context(parent);

        // Extract the name - may be from Name node or AssociatedTypeTarget node
        let (name_str, name_span) = extract_type_alias_name(syntax, source)?;

        // Get full span
        let full_span = get_node_span(syntax, source);

        // Extract visibility
        let visibility_str = extract_visibility(syntax);
        let visibility_enum = visibility_str.as_deref().and_then(parse_visibility);
        let visibility_span = get_visibility_span(syntax, source).unwrap_or(name_span.clone());
        let visibility_scope = find_visibility_scope(visibility_enum.as_ref(), parent, root);
        let visibility_behavior =
            VisibilityBehavior::new(visibility_enum, visibility_span, visibility_scope);

        // Create the name object
        let name = Spanned::new(name_str, name_span.clone());

        match context {
            TypeAliasContext::Protocol => {
                // In protocol: create AssociatedTypeSymbol
                let symbol = AssociatedTypeSymbol::new(
                    name.clone(),
                    full_span.clone(),
                    visibility_behavior,
                    parent.cloned(),
                );
                let symbol_arc = Arc::new(symbol);
                let symbol_arc_dyn = symbol_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

                // Add to parent
                if let Some(parent) = parent {
                    parent.metadata().add_child(&symbol_arc_dyn);
                }

                Some(symbol_arc_dyn)
            }
            TypeAliasContext::Struct | TypeAliasContext::Module => {
                // In struct or module: create TypeAliasSymbol
                // Use error type as placeholder - actual type will be resolved during bind
                let placeholder_type = Ty::error(full_span.clone());
                let syntactic_typed_behavior = TypedBehavior::new(placeholder_type, full_span.clone());

                // Create the type alias symbol
                let type_alias_symbol = TypeAliasSymbol::new(
                    name.clone(),
                    full_span.clone(),
                    visibility_behavior,
                    syntactic_typed_behavior,
                    parent.cloned(),
                );
                let type_alias_arc = Arc::new(type_alias_symbol);

                // Add TypeAlias type as semantic identity
                let type_alias_type =
                    kestrel_semantic_tree::ty::Ty::type_alias(type_alias_arc.clone(), full_span.clone());
                let semantic_typed_behavior = TypedBehavior::new(type_alias_type, full_span.clone());
                type_alias_arc
                    .metadata()
                    .add_behavior(semantic_typed_behavior);

                let type_alias_arc_dyn = type_alias_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

                // Extract type parameters with correct parent (the type alias, not the module)
                let type_parameters = extract_type_parameters(syntax, source, Some(type_alias_arc_dyn.clone()));

                // Add type parameters as children
                add_type_params_as_children(&type_parameters, &type_alias_arc_dyn);

                // Add to parent
                if let Some(parent) = parent {
                    parent.metadata().add_child(&type_alias_arc_dyn);
                }

                Some(type_alias_arc_dyn)
            }
        }
    }

    fn bind_declaration(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        let symbol_kind = symbol.metadata().kind();
        let symbol_id = symbol.metadata().id();

        // Get file_id and source for this symbol
        let (file_id, source) = context.get_file_context(symbol);

        match symbol_kind {
            KestrelSymbolKind::AssociatedType => {
                // Associated type in protocol: resolve bounds and optional default
                bind_associated_type(symbol, syntax, &source, file_id, context);
            }
            KestrelSymbolKind::TypeAlias => {
                // Regular type alias or struct binding: resolve aliased type
                // Enter cycle detector - guard automatically exits on drop
                let _guard = match semantic_tree::cycle::CycleDetector::enter_ref(context.type_alias_cycle_detector, symbol_id) {
                    Ok(guard) => guard,
                    Err(_) => return,
                };

                // Determine context to check for validation
                let alias_context = determine_context(symbol.metadata().parent().as_ref());
                let name = symbol.metadata().name().value.clone();
                let span = symbol.metadata().span().clone();

                // Check for bounds in module-level type aliases (not allowed)
                if alias_context == TypeAliasContext::Module {
                    if has_associated_type_bounds(syntax) {
                        context.diagnostics.throw(
                            AssociatedTypeBoundsInWrongContextError {
                                span: span.clone(),
                                name: name.clone(),
                            },
                            file_id,
                        );
                    }
                }

                // Validate associated type bindings in struct context
                if alias_context == TypeAliasContext::Struct {
                    if let Some(parent) = symbol.metadata().parent() {
                        validate_struct_associated_type_binding(
                            syntax, &source, &name, &parent, file_id, context,
                        );
                    }
                }

                // Extract type parameters and resolve where clause bounds
                let generics_behavior = resolve_generics(syntax, &source, symbol_id, context);
                symbol.metadata().add_behavior(generics_behavior);

                // Extract and resolve the aliased type from syntax
                if let Some(resolved_type) = resolve_aliased_type_from_syntax(syntax, &source, symbol_id, context, file_id) {
                    // Validate constraint satisfaction for struct bindings
                    if alias_context == TypeAliasContext::Struct {
                        if let Some(parent) = symbol.metadata().parent() {
                            validate_struct_binding_constraint_satisfaction(
                                &resolved_type,
                                &name,
                                &parent,
                                span.clone(),
                                file_id,
                                context,
                            );
                        }
                    }

                    let type_alias_typed_behavior = TypeAliasTypedBehavior::new(resolved_type);
                    symbol.metadata().add_behavior(type_alias_typed_behavior);
                } else {
                    // Type aliases require a type in both module and struct contexts
                    // In protocols, type aliases without `= Type` are valid (abstract associated types)
                    let diag_context = match alias_context {
                        TypeAliasContext::Module => Some(DiagTypeAliasContext::ModuleLevel),
                        TypeAliasContext::Struct => Some(DiagTypeAliasContext::StructWithoutConformance),
                        TypeAliasContext::Protocol => None, // Abstract associated types are valid
                    };

                    if let Some(ctx) = diag_context {
                        context.diagnostics.throw(
                            TypeAliasRequiresTypeError {
                                span,
                                name,
                                context: ctx,
                            },
                            file_id,
                        );
                    }
                }

                // Guard automatically calls exit() when dropped here
            }
            _ => {}
        }
    }
}

/// Check if a type alias syntax node has associated type bounds (e.g., `: Equatable`)
/// Bounds are indicated by a Colon token after the name/target but before Equals
fn has_associated_type_bounds(syntax: &SyntaxNode) -> bool {
    // Look for pattern: Name/AssociatedTypeTarget, then Colon, then Ty (bound)
    // We check for Colon token in the children of the TypeAliasDeclaration
    for element in syntax.children_with_tokens() {
        if let Some(token) = element.into_token() {
            // If we see a colon before equals, it's a bound
            if token.kind() == SyntaxKind::Colon {
                return true;
            }
            // If we see equals first, there are no bounds
            if token.kind() == SyntaxKind::Equals {
                return false;
            }
        }
    }
    false
}

/// Determine context based on parent symbol kind
fn determine_context(parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>) -> TypeAliasContext {
    match parent {
        Some(p) => match p.metadata().kind() {
            KestrelSymbolKind::Protocol => TypeAliasContext::Protocol,
            KestrelSymbolKind::Struct => TypeAliasContext::Struct,
            _ => TypeAliasContext::Module,
        },
        None => TypeAliasContext::Module,
    }
}

/// Extract the name from a type alias declaration
/// Handles both simple names (Name node) and qualified paths (AssociatedTypeTarget node)
fn extract_type_alias_name(syntax: &SyntaxNode, source: &str) -> Option<(String, kestrel_span::Span)> {
    // First try AssociatedTypeTarget (qualified path)
    if let Some(target_node) = find_child(syntax, SyntaxKind::AssociatedTypeTarget) {
        // In qualified path, the name is the last Name node
        if let Some(name_node) = find_child(&target_node, SyntaxKind::Name) {
            let name_str = extract_name_from_node(&name_node)?;
            let name_span = get_node_span(&name_node, source);
            return Some((name_str, name_span));
        }
    }

    // Fall back to simple Name node
    if let Some(name_node) = find_child(syntax, SyntaxKind::Name) {
        let name_str = extract_name_from_node(&name_node)?;
        let name_span = get_node_span(&name_node, source);
        return Some((name_str, name_span));
    }

    None
}

/// Extract name string from a Name node
fn extract_name_from_node(name_node: &SyntaxNode) -> Option<String> {
    name_node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .find(|tok| tok.kind() == SyntaxKind::Identifier)
        .map(|tok| tok.text().to_string())
}

/// Bind an associated type symbol (resolve bounds and default)
fn bind_associated_type(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context: &mut BindingContext,
) {
    let symbol_id = symbol.metadata().id();

    // Resolve bounds if present (: Equatable, Hashable)
    let bounds = resolve_associated_type_bounds(syntax, source, symbol_id, context, file_id);
    if !bounds.is_empty() {
        let bounds_behavior = AssociatedTypeBoundsBehavior::new(bounds);
        symbol.metadata().add_behavior(bounds_behavior);
    }

    // Resolve default type if present (= Int)
    // Note: Validation of defaults against bounds happens in a separate validation pass
    // after all conformances have been resolved (see ConformanceValidator)
    if let Some(default_type) = resolve_aliased_type_from_syntax(syntax, source, symbol_id, context, file_id) {
        let typed_behavior = TypedBehavior::new(default_type, symbol.metadata().span().clone());
        symbol.metadata().add_behavior(typed_behavior);
    }
}

/// Resolve associated type bounds from syntax
fn resolve_associated_type_bounds(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    file_id: usize,
) -> Vec<Ty> {
    // Look for colon followed by type paths (bounds)
    // The syntax is: type Name: Bound1, Bound2 = Default;
    // We need to find Path nodes that come after the colon but before equals
    let mut bounds = Vec::new();

    // Iterate through children looking for Path nodes (bounds)
    // These appear directly in the TypeAliasDeclaration after the Colon token
    let mut after_colon = false;
    for child in syntax.children_with_tokens() {
        match &child {
            SyntaxElement::Token(tok) if tok.kind() == SyntaxKind::Colon => {
                after_colon = true;
            }
            SyntaxElement::Token(tok) if tok.kind() == SyntaxKind::Equals => {
                // Stop collecting bounds once we hit equals
                break;
            }
            SyntaxElement::Node(node) if after_colon && (node.kind() == SyntaxKind::Ty || node.kind() == SyntaxKind::TyPath) => {
                // This is a bound type
                let mut type_ctx = TypeSyntaxContext::new(ctx.db, ctx.diagnostics, file_id, source, context_id);
                let bound_ty = resolve_type_from_ty_node(node, &mut type_ctx);

                // Validate it's a protocol
                match bound_ty.kind() {
                    TyKind::Protocol { .. } => bounds.push(bound_ty),
                    TyKind::Error { .. } => bounds.push(bound_ty), // Keep errors for diagnostics
                    _ => {
                        // Not a protocol - emit error
                        // Extract type name without using Debug format (which can cause cyclic reference issues)
                        let type_name = get_type_display_name(&bound_ty);
                        ctx.diagnostics.throw(NotAProtocolError {
                            span: get_node_span(node, source),
                            name: type_name,
                            context: NotAProtocolContext::Bound,
                        }, file_id);
                    }
                }
            }
            _ => {}
        }
    }

    bounds
}

/// Extract type parameters and resolve where clause bounds, creating a GenericsBehavior.
fn resolve_generics(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
) -> GenericsBehavior {
    // Get type parameters from the symbol's children (they were added during BUILD)
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

    let file_id = ctx.db.symbol_by_id(context_id)
        .map(|s| get_file_id_for_symbol(&s, ctx.diagnostics))
        .unwrap_or(0);

    let mut constraints = Vec::new();

    for child in where_clause_node.children() {
        if child.kind() == SyntaxKind::TypeBound {
            if let Some(constraint) = resolve_type_bound(&child, source, context_id, ctx, type_params, file_id) {
                constraints.push(constraint);
            }
        }
    }

    WhereClause::with_constraints(constraints)
}

/// Resolve a single TypeBound, resolving protocol paths to actual types.
fn resolve_type_bound(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    type_params: &[Arc<TypeParameterSymbol>],
    file_id: usize,
) -> Option<Constraint> {
    // Find the Name node and extract the type parameter name and span
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

            let bound_name = segments.join(".");

            // Resolve the path to a type
            match ctx.db.resolve_type_path(segments, context_id) {
                TypePathResolution::Resolved(resolved_ty) => {
                    match resolved_ty.kind() {
                        TyKind::Protocol { .. } => resolved_ty,
                        TyKind::Struct { symbol, .. } => {
                            ctx.diagnostics.throw(NotAProtocolError {
                                span: span.clone(),
                                name: symbol.metadata().name().value.clone(),
                                context: NotAProtocolContext::Bound,
                            }, file_id);
                            Ty::error(span)
                        }
                        TyKind::TypeAlias { symbol, .. } => {
                            ctx.diagnostics.throw(NotAProtocolError {
                                span: span.clone(),
                                name: symbol.metadata().name().value.clone(),
                                context: NotAProtocolContext::Bound,
                            }, file_id);
                            Ty::error(span)
                        }
                        _ => {
                            ctx.diagnostics.throw(NotAProtocolError {
                                span: span.clone(),
                                name: bound_name.clone(),
                                context: NotAProtocolContext::Bound,
                            }, file_id);
                            Ty::error(span)
                        }
                    }
                }
                TypePathResolution::NotFound { .. } => {
                    ctx.diagnostics.throw(UnresolvedTypeError {
                        span: span.clone(),
                        type_name: bound_name.clone(),
                    }, file_id);
                    Ty::error(span)
                }
                TypePathResolution::Ambiguous { .. } | TypePathResolution::NotAType { .. } => {
                    ctx.diagnostics.throw(NotAProtocolError {
                        span: span.clone(),
                        name: bound_name.clone(),
                        context: NotAProtocolContext::Bound,
                    }, file_id);
                    Ty::error(span)
                }
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

/// Resolve the aliased type from a TypeAliasDeclaration syntax node during bind phase.
///
/// Returns None if no aliased type is present (e.g., for abstract associated types in protocols).
/// Uses the unified type resolution from type_syntax module.
fn resolve_aliased_type_from_syntax(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    file_id: usize,
) -> Option<Ty> {
    // Find the AliasedType node - may not exist for abstract associated types
    let aliased_type_node = find_child(syntax, SyntaxKind::AliasedType)?;

    // Try to find a Ty node first. If it doesn't exist, use the AliasedType node itself.
    let ty_node = find_child(&aliased_type_node, SyntaxKind::Ty)
        .unwrap_or_else(|| aliased_type_node.clone());

    // Use unified type resolution
    let mut type_ctx = TypeSyntaxContext::new(ctx.db, ctx.diagnostics, file_id, source, context_id);
    Some(resolve_type_from_ty_node(&ty_node, &mut type_ctx))
}

/// Get a display name for a type without using Debug formatting
/// This avoids cyclic reference issues that can occur with derived Debug implementations
fn get_type_display_name(ty: &Ty) -> String {
    match ty.kind() {
        TyKind::Unit => "()".to_string(),
        TyKind::Never => "Never".to_string(),
        TyKind::Int(_) => "Int".to_string(),
        TyKind::Float(_) => "Float".to_string(),
        TyKind::Bool => "Bool".to_string(),
        TyKind::String => "String".to_string(),
        TyKind::Tuple(_) => "tuple".to_string(),
        TyKind::Array(_) => "array".to_string(),
        TyKind::Function { .. } => "function".to_string(),
        TyKind::Error => "error".to_string(),
        TyKind::SelfType => "Self".to_string(),
        TyKind::Inferred => "_".to_string(),
        TyKind::TypeParameter(p) => p.metadata().name().value.clone(),
        TyKind::Protocol { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::Struct { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::TypeAlias { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::AssociatedType { symbol, .. } => symbol.metadata().name().value.clone(),
    }
}

/// Validate associated type bindings in a struct.
///
/// This validates:
/// 1. Unqualified bindings are not ambiguous (not defined in multiple conformed protocols)
/// 2. Qualified bindings reference a protocol the struct conforms to
/// 3. Qualified bindings reference an associated type that exists in the protocol
fn validate_struct_associated_type_binding(
    syntax: &SyntaxNode,
    source: &str,
    type_name: &str,
    parent: &Arc<dyn Symbol<KestrelLanguage>>,
    file_id: usize,
    ctx: &mut BindingContext,
) {
    use crate::diagnostics::{
        AmbiguousAssociatedTypeError, QualifiedBindingNotConformingError,
        QualifiedBindingWrongProtocolError,
    };

    let struct_name = parent.metadata().name().value.clone();
    let binding_span = get_node_span(syntax, source);

    // Get the struct's conformances
    let conformances = parent
        .conformances_behavior()
        .map(|cb| cb.conformances().to_vec())
        .unwrap_or_default();

    // Check if this is a qualified binding (has AssociatedTypeTarget with protocol path)
    if let Some(target_node) = find_child(syntax, SyntaxKind::AssociatedTypeTarget) {
        // Qualified binding: type Protocol.Item = Type
        // Extract the protocol from the Ty node inside the target
        if let Some(ty_node) = find_child(&target_node, SyntaxKind::Ty) {
            let protocol_name = extract_path_name_from_ty_node(&ty_node);

            if let Some(protocol_name) = protocol_name {
                // Check 1: Does the struct conform to this protocol?
                let conforms_to_protocol = conformances.iter().any(|conf| {
                    if let TyKind::Protocol { symbol, .. } = conf.kind() {
                        symbol.metadata().name().value == protocol_name
                    } else {
                        false
                    }
                });

                if !conforms_to_protocol {
                    ctx.diagnostics.throw(
                        QualifiedBindingNotConformingError {
                            span: binding_span,
                            struct_name,
                            protocol_name: protocol_name.clone(),
                        },
                        file_id,
                    );
                    return;
                }

                // Check 2: Does the protocol have this associated type?
                let protocol_has_type = conformances.iter().any(|conf| {
                    if let TyKind::Protocol { symbol, .. } = conf.kind() {
                        if symbol.metadata().name().value == protocol_name {
                            // Check if protocol has the associated type
                            let protocol_dyn = symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
                            return protocol_dyn.metadata().children().iter().any(|child| {
                                child.metadata().kind() == KestrelSymbolKind::AssociatedType
                                    && child.metadata().name().value == type_name
                            });
                        }
                    }
                    false
                });

                if !protocol_has_type {
                    ctx.diagnostics.throw(
                        QualifiedBindingWrongProtocolError {
                            span: binding_span,
                            protocol_name,
                            type_name: type_name.to_string(),
                        },
                        file_id,
                    );
                }
            }
        }
    } else {
        // Unqualified binding: type Item = Type
        // Check for ambiguity - does more than one conformed protocol have this associated type?
        let protocols_with_type: Vec<String> = conformances
            .iter()
            .filter_map(|conf| {
                if let TyKind::Protocol { symbol, .. } = conf.kind() {
                    let protocol_dyn = symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
                    let has_type = protocol_dyn.metadata().children().iter().any(|child| {
                        child.metadata().kind() == KestrelSymbolKind::AssociatedType
                            && child.metadata().name().value == type_name
                    });
                    if has_type {
                        return Some(symbol.metadata().name().value.clone());
                    }
                }
                None
            })
            .collect();

        if protocols_with_type.len() > 1 {
            ctx.diagnostics.throw(
                AmbiguousAssociatedTypeError {
                    span: binding_span,
                    type_name: type_name.to_string(),
                    protocols: protocols_with_type,
                },
                file_id,
            );
        }
    }

    // Validate that the bound type satisfies any constraints on the associated type
    // This handles both qualified and unqualified bindings
    // Note: We defer this validation to after the type is resolved in bind_declaration
}

/// Extract the first path segment name from a Ty node
fn extract_path_name_from_ty_node(ty_node: &SyntaxNode) -> Option<String> {
    // Look for Path -> PathElement -> Identifier
    if let Some(path_node) = find_child(ty_node, SyntaxKind::Path) {
        for child in path_node.children() {
            if child.kind() == SyntaxKind::PathElement {
                for elem in child.children_with_tokens() {
                    if let Some(token) = elem.into_token() {
                        if token.kind() == SyntaxKind::Identifier {
                            return Some(token.text().to_string());
                        }
                    }
                }
            }
        }
    }

    // Try TyPath which contains Path
    if let Some(ty_path) = find_child(ty_node, SyntaxKind::TyPath) {
        if let Some(path_node) = find_child(&ty_path, SyntaxKind::Path) {
            for child in path_node.children() {
                if child.kind() == SyntaxKind::PathElement {
                    for elem in child.children_with_tokens() {
                        if let Some(token) = elem.into_token() {
                            if token.kind() == SyntaxKind::Identifier {
                                return Some(token.text().to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Also try TyPath directly at ty_node level
    if ty_node.kind() == SyntaxKind::TyPath {
        if let Some(path_node) = find_child(ty_node, SyntaxKind::Path) {
            for child in path_node.children() {
                if child.kind() == SyntaxKind::PathElement {
                    for elem in child.children_with_tokens() {
                        if let Some(token) = elem.into_token() {
                            if token.kind() == SyntaxKind::Identifier {
                                return Some(token.text().to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Validate that a struct's associated type binding satisfies the protocol's constraints.
///
/// This looks up the associated type in the protocol, gets its bounds, and checks if the
/// bound type conforms to those protocols.
fn validate_struct_binding_constraint_satisfaction(
    bound_type: &Ty,
    type_name: &str,
    parent: &Arc<dyn Symbol<KestrelLanguage>>,
    binding_span: kestrel_span::Span,
    file_id: usize,
    ctx: &mut BindingContext,
) {
    use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;

    // Get the struct's conformances
    let conformances = parent
        .conformances_behavior()
        .map(|cb| cb.conformances().to_vec())
        .unwrap_or_default();

    // Collect all protocols (direct and inherited) to check for associated type definitions
    let mut all_protocols = Vec::new();
    let mut to_check: Vec<_> = conformances.clone();

    // BFS to collect all protocols including inherited ones
    while let Some(conformance) = to_check.pop() {
        if let TyKind::Protocol { symbol: protocol_symbol, .. } = conformance.kind() {
            all_protocols.push(conformance.clone());

            // Add inherited protocols (protocol conformances)
            if let Some(inherited_conformances) = protocol_symbol.conformances_behavior() {
                for inherited in inherited_conformances.conformances() {
                    to_check.push(inherited.clone());
                }
            }
        }
    }

    // Find the protocol(s) that define this associated type and get their bounds
    for protocol in &all_protocols {
        if let TyKind::Protocol { symbol: protocol_symbol, .. } = protocol.kind() {
            let protocol_dyn = protocol_symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>;

            // Find the associated type in this protocol
            for child in protocol_dyn.metadata().children() {
                if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                    && child.metadata().name().value == type_name
                {
                    // Found the associated type - check if it has bounds
                    if let Ok(assoc_type_symbol) = child.downcast_arc::<AssociatedTypeSymbol>() {
                        if let Some(bounds) = assoc_type_symbol.bounds() {
                            // Validate that bound_type satisfies these bounds
                            validate_type_satisfies_bounds(
                                bound_type,
                                &bounds,
                                type_name,
                                binding_span.clone(),
                                file_id,
                                ctx,
                            );
                        }
                    }
                }
            }
        }
    }

    // Also check inherited constraints from where clauses in protocols
    // For test case 3: struct conforming to refined protocol must satisfy constraint
    validate_inherited_where_clause_constraints(
        type_name,
        bound_type,
        &conformances,
        &binding_span,
        file_id,
        ctx,
    );
}

/// Check if a type satisfies a list of protocol bounds.
///
/// For each bound protocol, checks if the type has a ConformancesBehavior that includes
/// that protocol.
fn validate_type_satisfies_bounds(
    bound_type: &Ty,
    required_bounds: &[Ty],
    type_name: &str,
    span: kestrel_span::Span,
    file_id: usize,
    ctx: &mut BindingContext,
) {
    use crate::diagnostics::AssociatedTypeConstraintNotSatisfiedError;

    // Get the type name for error messages
    let bound_type_name = get_type_display_name(bound_type);

    // For each required protocol bound, check if the type conforms to it
    for required_protocol in required_bounds {
        // Skip error bounds - they've already been reported
        if matches!(required_protocol.kind(), TyKind::Error { .. }) {
            continue;
        }

        if let TyKind::Protocol { symbol: required_proto_symbol, .. } = required_protocol.kind() {
            let required_protocol_name = required_proto_symbol.metadata().name().value.clone();

            // Check if the bound type conforms to this protocol
            let conforms = match bound_type.kind() {
                TyKind::Struct { symbol, .. } => {
                    // Get the struct's conformances
                    let conformances = symbol
                        .conformances_behavior()
                        .map(|cb| cb.conformances().to_vec())
                        .unwrap_or_default();

                    // Check if any conformance matches the required protocol
                    // We need to compare by symbol ID, not name, to handle same-named protocols in different scopes
                    conformances.iter().any(|conf| {
                        if let TyKind::Protocol { symbol: proto_sym, .. } = conf.kind() {
                            proto_sym.metadata().id() == required_proto_symbol.metadata().id()
                        } else {
                            false
                        }
                    })
                }
                TyKind::TypeParameter(_) => {
                    // Type parameters might conform through bounds - for now we allow them
                    // A more sophisticated implementation would check the type parameter's bounds
                    true
                }
                TyKind::Error { .. } => {
                    // Don't report additional errors for error types
                    true
                }
                _ => {
                    // Other types (primitives, protocols, etc.) don't have conformances
                    false
                }
            };

            if !conforms {
                ctx.diagnostics.throw(
                    AssociatedTypeConstraintNotSatisfiedError {
                        span,
                        type_name: type_name.to_string(),
                        bound_type: bound_type_name.clone(),
                        required_protocol: required_protocol_name,
                    },
                    file_id,
                );
                return; // Only report the first violation
            }
        }
    }
}

/// Validate inherited where clause constraints on associated types.
///
/// For example, if a protocol has `where Iterator.Item: Comparable`, a struct conforming
/// to that protocol must bind Item to a type that conforms to Comparable.
fn validate_inherited_where_clause_constraints(
    type_name: &str,
    bound_type: &Ty,
    conformances: &[Ty],
    binding_span: &kestrel_span::Span,
    file_id: usize,
    ctx: &mut BindingContext,
) {
    // Collect all protocols (direct and inherited) to check where clauses
    let mut all_protocols = Vec::new();
    let mut to_check: Vec<_> = conformances.to_vec();

    // BFS to collect all protocols including inherited ones
    while let Some(conformance) = to_check.pop() {
        if let TyKind::Protocol { symbol: protocol_symbol, .. } = conformance.kind() {
            all_protocols.push(conformance.clone());

            // Add inherited protocols (protocol conformances)
            if let Some(inherited_conformances) = protocol_symbol.conformances_behavior() {
                for inherited in inherited_conformances.conformances() {
                    to_check.push(inherited.clone());
                }
            }
        }
    }

    // For each protocol (including inherited ones)
    for protocol in &all_protocols {
        if let TyKind::Protocol { symbol: protocol_symbol, .. } = protocol.kind() {
            // Check if the protocol has a where clause with constraints on associated types
            if let Some(generics) = protocol_symbol.generics_behavior() {
                let where_clause = generics.where_clause();

                // Look for constraints in the where clause
                // These are stored as TypeBound or InheritedAssociatedTypeBound constraints
                for constraint in &where_clause.constraints {
                    match constraint {
                        kestrel_semantic_tree::ty::Constraint::TypeBound {
                            param_name,
                            bounds,
                            ..
                        } => {
                            // Check if this constraint is for an associated type
                            // Format is like "Iterator.Item" or just "Item"
                            if let Some(assoc_name) = param_name.split('.').last() {
                                if assoc_name == type_name {
                                    // Validate that the bound type satisfies these bounds
                                    validate_type_satisfies_bounds(
                                        bound_type,
                                        bounds,
                                        type_name,
                                        binding_span.clone(),
                                        file_id,
                                        ctx,
                                    );
                                }
                            }
                        }
                        kestrel_semantic_tree::ty::Constraint::InheritedAssociatedTypeBound {
                            path,
                            bounds,
                            ..
                        } => {
                            // For inherited associated type bounds like "Iterator.Item: Comparable"
                            // Extract the associated type name from the path
                            if let Some(assoc_name) = path.split('.').last() {
                                if assoc_name == type_name {
                                    validate_type_satisfies_bounds(
                                        bound_type,
                                        bounds,
                                        type_name,
                                        binding_span.clone(),
                                        file_id,
                                        ctx,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
