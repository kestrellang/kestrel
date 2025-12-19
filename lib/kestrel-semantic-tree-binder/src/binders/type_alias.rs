use std::sync::Arc;

use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::associated_type::{
    AssociatedTypeBoundsBehavior
};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasTypedBehavior;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use kestrel_syntax_tree::{SyntaxElement, SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::diagnostics::{
    AssociatedTypeBoundsInWrongContextError, NotAProtocolContext, NotAProtocolError,
    TypeAliasContext as DiagTypeAliasContext, TypeAliasRequiresTypeError,
};
use crate::resolution::type_resolver::{TypeSyntaxContext, resolve_type_from_ty_node};
use kestrel_syntax_tree::utils::{find_child, get_node_span};

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

/// Binder for type alias declarations
pub struct TypeAliasBinder;

impl DeclarationBinder for TypeAliasBinder {
    fn bind_declaration(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        let symbol_kind = symbol.metadata().kind();
        let symbol_id = symbol.metadata().id();

        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        match symbol_kind {
            KestrelSymbolKind::AssociatedType => {
                // Associated type in protocol: resolve bounds and optional default
                bind_associated_type(symbol, syntax, &source, file_id, context);
            }
            KestrelSymbolKind::TypeAlias => {
                // Regular type alias or struct binding: resolve aliased type
                // Enter cycle detector
                if let Err(_) = semantic_tree::cycle::CycleDetector::enter_ref(
                    context.type_alias_cycle_detector,
                    symbol_id,
                ) {
                    return;
                }

                // Determine context to check for validation
                let alias_context = determine_context(symbol.metadata().parent().as_ref());
                let name = symbol.metadata().name().value.clone();
                let span = symbol.metadata().span().clone();

                // Check for bounds in module-level type aliases (not allowed)
                if alias_context == TypeAliasContext::Module {
                    if has_associated_type_bounds(syntax) {
                        context
                            .diagnostics
                            .throw(AssociatedTypeBoundsInWrongContextError {
                                span: span.clone(),
                                name: name.clone(),
                            });
                    }
                }

                // Validate associated type bindings in struct context
                if alias_context == TypeAliasContext::Struct {
                    if let Some(parent) = symbol.metadata().parent() {
                        validate_struct_associated_type_binding(
                            syntax, &source, file_id, &name, &parent, context,
                        );
                    }
                }

                // Extract type parameters and resolve where clause bounds
                let generics_behavior =
                    crate::binders::utils::generics::resolve_generics(syntax, &source, file_id, symbol_id, context);
                symbol.metadata().add_behavior(generics_behavior);

                // Extract and resolve the aliased type from syntax
                if let Some(resolved_type) =
                    resolve_aliased_type_from_syntax(syntax, &source, file_id, symbol_id, context)
                {
                    // Validate constraint satisfaction for struct bindings
                    if alias_context == TypeAliasContext::Struct {
                        if let Some(parent) = symbol.metadata().parent() {
                            validate_struct_binding_constraint_satisfaction(
                                &resolved_type,
                                &name,
                                &parent,
                                span.clone(),
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
                        TypeAliasContext::Struct => {
                            Some(DiagTypeAliasContext::StructWithoutConformance)
                        }
                        TypeAliasContext::Protocol => None, // Abstract associated types are valid
                    };

                    if let Some(ctx) = diag_context {
                        context.diagnostics.throw(TypeAliasRequiresTypeError {
                            span,
                            name,
                            context: ctx,
                        });
                    }
                }

                // Exit cycle detector
                semantic_tree::cycle::CycleDetector::exit_ref(context.type_alias_cycle_detector);
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
    let bounds = resolve_associated_type_bounds(syntax, source, file_id, symbol_id, context);
    if !bounds.is_empty() {
        let bounds_behavior = AssociatedTypeBoundsBehavior::new(bounds);
        symbol.metadata().add_behavior(bounds_behavior);
    }

    // Resolve default type if present (= Int)
    // Note: Validation of defaults against bounds happens in a separate validation pass
    // after all conformances have been resolved (see ConformanceValidator)
    if let Some(default_type) =
        resolve_aliased_type_from_syntax(syntax, source, file_id, symbol_id, context)
    {
        let typed_behavior = TypedBehavior::new(default_type, symbol.metadata().span().clone());
        symbol.metadata().add_behavior(typed_behavior);
    }
}

/// Resolve associated type bounds from syntax
fn resolve_associated_type_bounds(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
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
            SyntaxElement::Node(node)
                if after_colon
                    && (node.kind() == SyntaxKind::Ty || node.kind() == SyntaxKind::TyPath) =>
            {
                // This is a bound type
                let mut type_ctx =
                    TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
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
                            span: get_node_span(node, file_id),
                            name: type_name,
                            context: NotAProtocolContext::Bound,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    bounds
}

/// Resolve the aliased type from a TypeAliasDeclaration syntax node during bind phase.
///
/// Returns None if no aliased type is present (e.g., for abstract associated types in protocols).
/// Uses the unified type resolution from type_syntax module.
fn resolve_aliased_type_from_syntax(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
) -> Option<Ty> {
    // Find the AliasedType node - may not exist for abstract associated types
    let aliased_type_node = find_child(syntax, SyntaxKind::AliasedType)?;

    // Try to find a Ty node first. If it doesn't exist, use the AliasedType node itself.
    let ty_node =
        find_child(&aliased_type_node, SyntaxKind::Ty).unwrap_or_else(|| aliased_type_node.clone());

    // Use unified type resolution
    let mut type_ctx =
        TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
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
        TyKind::Infer => "_".to_string(),
        TyKind::TypeParameter(p) => p.metadata().name().value.clone(),
        TyKind::Protocol { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::Struct { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::TypeAlias { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::AssociatedType { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::UnresolvedFunction { .. } => "closure".to_string(),
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
    _source: &str,
    file_id: usize,
    type_name: &str,
    parent: &Arc<dyn Symbol<KestrelLanguage>>,
    ctx: &mut BindingContext,
) {
    use crate::diagnostics::{
        AmbiguousAssociatedTypeError, QualifiedBindingNotConformingError,
        QualifiedBindingWrongProtocolError,
    };

    let struct_name = parent.metadata().name().value.clone();
    let binding_span = get_node_span(syntax, file_id);

    // Get the struct's conformances
    let conformances = parent
        .metadata()
        .get_behavior::<ConformancesBehavior>()
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
                    ctx.diagnostics.throw(QualifiedBindingNotConformingError {
                        span: binding_span,
                        struct_name,
                        protocol_name: protocol_name.clone(),
                    });
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
                    ctx.diagnostics.throw(QualifiedBindingWrongProtocolError {
                        span: binding_span,
                        protocol_name,
                        type_name: type_name.to_string(),
                    });
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
            ctx.diagnostics.throw(AmbiguousAssociatedTypeError {
                span: binding_span,
                type_name: type_name.to_string(),
                protocols: protocols_with_type,
            });
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
    ctx: &mut BindingContext,
) {
    use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;

    // Get the struct's conformances
    let conformances = parent
        .metadata()
        .get_behavior::<ConformancesBehavior>()
        .map(|cb| cb.conformances().to_vec())
        .unwrap_or_default();

    // Collect all protocols (direct and inherited) to check for associated type definitions
    let mut all_protocols = Vec::new();
    let mut to_check: Vec<_> = conformances.clone();

    // BFS to collect all protocols including inherited ones
    while let Some(conformance) = to_check.pop() {
        if let TyKind::Protocol {
            symbol: protocol_symbol,
            ..
        } = conformance.kind()
        {
            all_protocols.push(conformance.clone());

            // Add inherited protocols (protocol conformances)
            if let Some(inherited_conformances) = protocol_symbol
                .metadata()
                .get_behavior::<ConformancesBehavior>()
            {
                for inherited in inherited_conformances.conformances() {
                    to_check.push(inherited.clone());
                }
            }
        }
    }

    // Find the protocol(s) that define this associated type and get their bounds
    for protocol in &all_protocols {
        if let TyKind::Protocol {
            symbol: protocol_symbol,
            ..
        } = protocol.kind()
        {
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

        if let TyKind::Protocol {
            symbol: required_proto_symbol,
            ..
        } = required_protocol.kind()
        {
            let required_protocol_name = required_proto_symbol.metadata().name().value.clone();

            // Check if the bound type conforms to this protocol
            let conforms = match bound_type.kind() {
                TyKind::Struct { symbol, .. } => {
                    // Get the struct's conformances
                    let conformances = symbol
                        .metadata()
                        .get_behavior::<ConformancesBehavior>()
                        .map(|cb| cb.conformances().to_vec())
                        .unwrap_or_default();

                    // Check if any conformance matches the required protocol
                    // We need to compare by symbol ID, not name, to handle same-named protocols in different scopes
                    conformances.iter().any(|conf| {
                        if let TyKind::Protocol {
                            symbol: proto_sym, ..
                        } = conf.kind()
                        {
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
                ctx.diagnostics
                    .throw(AssociatedTypeConstraintNotSatisfiedError {
                        span,
                        type_name: type_name.to_string(),
                        bound_type: bound_type_name.clone(),
                        required_protocol: required_protocol_name,
                    });
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
    ctx: &mut BindingContext,
) {
    // Collect all protocols (direct and inherited) to check where clauses
    let mut all_protocols = Vec::new();
    let mut to_check: Vec<_> = conformances.to_vec();

    // BFS to collect all protocols including inherited ones
    while let Some(conformance) = to_check.pop() {
        if let TyKind::Protocol {
            symbol: protocol_symbol,
            ..
        } = conformance.kind()
        {
            all_protocols.push(conformance.clone());

            // Add inherited protocols (protocol conformances)
            if let Some(inherited_conformances) = protocol_symbol
                .metadata()
                .get_behavior::<ConformancesBehavior>()
            {
                for inherited in inherited_conformances.conformances() {
                    to_check.push(inherited.clone());
                }
            }
        }
    }

    // For each protocol (including inherited ones)
    for protocol in &all_protocols {
        if let TyKind::Protocol {
            symbol: protocol_symbol,
            ..
        } = protocol.kind()
        {
            // Check if the protocol has a where clause with constraints on associated types
            if let Some(generics) = protocol_symbol
                .metadata()
                .get_behavior::<GenericsBehavior>()
            {
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
                                        ctx,
                                    );
                                }
                            }
                        }
                        // TypeEquality constraints are handled during type checking, not here
                        kestrel_semantic_tree::ty::Constraint::TypeEquality { .. } => {}
                    }
                }
            }
        }
    }
}
