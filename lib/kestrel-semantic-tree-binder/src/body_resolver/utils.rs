//! Shared utilities for body resolution.
//!
//! This module contains helper functions used across multiple body resolution
//! modules, including type formatting, signature matching, and behavior lookups.

use std::sync::Arc;

use kestrel_reporting::IntoDiagnostic;
use kestrel_semantic_model::SymbolFor;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::behavior::implements::ImplementsBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::expr::{ExprKind, Expression};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
use kestrel_semantic_tree::ty::{Constraint, Substitutions, Ty, TyKind, WhereClause};
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxKind;
use semantic_tree::symbol::{Symbol, SymbolId};

use super::context::BodyResolutionContext;
use crate::diagnostics::TypeParameterCannotBeUsedAsValueError;

/// Check if a syntax kind is an expression kind
pub fn is_expression_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Expression
            | SyntaxKind::ExprUnit
            | SyntaxKind::ExprInteger
            | SyntaxKind::ExprFloat
            | SyntaxKind::ExprString
            | SyntaxKind::ExprChar
            | SyntaxKind::ExprBool
            | SyntaxKind::ExprArray
            | SyntaxKind::ExprDictionary
            | SyntaxKind::ExprTuple
            | SyntaxKind::ExprGrouping
            | SyntaxKind::ExprPath
            | SyntaxKind::ExprUnary
            | SyntaxKind::ExprPostfix
            | SyntaxKind::ExprBinary
            | SyntaxKind::ExprNull
            | SyntaxKind::ExprCall
            | SyntaxKind::ExprAssignment
            | SyntaxKind::ExprCompoundAssignment
            | SyntaxKind::ExprIf
            | SyntaxKind::ExprWhile
            | SyntaxKind::ExprLoop
            | SyntaxKind::ExprFor
            | SyntaxKind::ExprBreak
            | SyntaxKind::ExprContinue
            | SyntaxKind::ExprReturn
            | SyntaxKind::ExprThrow
            | SyntaxKind::ExprTry
            | SyntaxKind::ExprTupleIndex
            | SyntaxKind::ExprClosure
            | SyntaxKind::ExprImplicitMemberAccess
            | SyntaxKind::ExprMatch
    )
}

/// Check if an expression is a standalone type parameter reference and emit an error if so.
///
/// Type parameters cannot be used as values directly - they must be called as init (`T()`)
/// or used for member access (`T.staticMethod()`). This function should be called anywhere
/// an expression is used as a value (variable initializers, return statements, function arguments).
///
/// Returns the expression unchanged if valid, or an error expression if invalid.
pub fn validate_not_standalone_type_param(
    expr: Expression,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    if let ExprKind::TypeParameterRef(symbol_id) = &expr.kind {
        // Get the type parameter name for the error message
        let type_param_name = ctx
            .model
            .query(SymbolFor { id: *symbol_id })
            .map(|s| s.metadata().name().value.clone())
            .unwrap_or_else(|| "T".to_string());

        let error = TypeParameterCannotBeUsedAsValueError {
            span: expr.span.clone(),
            type_param_name,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());

        return Expression::error(expr.span);
    }
    expr
}

/// Check if a callable signature matches the given arity and labels.
///
/// For parameters with default values, callers may omit trailing arguments.
/// The arity must be between the number of required parameters (those without
/// defaults) and the total number of parameters.
pub fn matches_signature(
    callable: &CallableBehavior,
    arity: usize,
    labels: &[Option<String>],
) -> bool {
    let params = callable.parameters();

    // Count required parameters (those without defaults)
    let required_count = params.iter().filter(|p| !p.has_default()).count();

    // Check arity: must be at least required_count and at most total params
    if arity < required_count || arity > params.len() {
        return false;
    }

    // Check labels for provided arguments only (first `arity` parameters)
    for (i, label) in labels.iter().enumerate() {
        if i >= params.len() {
            return false; // More labels than params - shouldn't happen
        }
        let param_label = params[i].external_label();
        let label_ref = label.as_deref();
        if param_label != label_ref {
            return false;
        }
    }

    true
}

/// Get the CallableBehavior from a symbol if it has one
pub fn get_callable_behavior(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Option<CallableBehavior> {
    for behavior in symbol.metadata().behaviors() {
        if behavior.kind() == KestrelBehaviorKind::Callable
            && let Some(callable) = behavior.as_ref().downcast_ref::<CallableBehavior>()
        {
            return Some(callable.clone());
        }
    }
    None
}

/// Create a struct type from a struct symbol.
///
/// This function downcasts the symbol to StructSymbol and creates a Ty::struct.
/// If the downcast fails (shouldn't happen for struct symbols), returns Ty::error.
pub fn create_struct_type(struct_symbol: &Arc<dyn Symbol<KestrelLanguage>>, span: Span) -> Ty {
    // Clone the Arc and use downcast_arc from downcast-rs to convert
    // Arc<dyn Symbol> to Arc<StructSymbol>
    let sym_clone = Arc::clone(struct_symbol);

    match sym_clone.downcast_arc::<StructSymbol>() {
        Ok(struct_arc) => {
            let type_params = struct_arc.type_parameters();
            if type_params.is_empty() {
                return Ty::r#struct(struct_arc, span);
            }

            // No explicit type arguments provided: treat as Struct[_, _, ...]
            let mut substitutions = Substitutions::new();
            for param in type_params {
                substitutions.insert(param.metadata().id(), Ty::infer(span.clone()));
            }

            Ty::generic_struct(struct_arc, substitutions, span)
        },
        Err(_) => {
            // This shouldn't happen if we're calling this on a struct symbol
            Ty::error(span)
        },
    }
}

/// Create a struct type with explicit type arguments.
///
/// This function takes explicit type arguments and creates a generic struct type
/// with those arguments mapped to the struct's type parameters IN ORDER.
///
/// For example, if we have `struct Pair[T, U]` and call `Pair[Int, String]`,
/// this creates substitutions: {T -> Int, U -> String}.
///
/// If we call `Pair[U, T]` where U and T are type parameters in the current scope,
/// this creates substitutions: {Pair's T -> U, Pair's U -> T} (preserving the type parameters).
///
/// # Arguments
/// * `struct_symbol` - The struct symbol
/// * `type_args` - The explicit type arguments (already resolved in current scope by TypeResolver)
/// * `span` - The span for the created type
/// * `ctx` - The body resolution context (used for resolving default type arguments)
pub fn create_struct_type_with_type_args(
    struct_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    type_args: &[Ty],
    span: Span,
    ctx: &super::context::BodyResolutionContext,
) -> Ty {
    use kestrel_semantic_model::{ResolveTypePath, TypePathResolution};

    let sym_clone = Arc::clone(struct_symbol);

    match sym_clone.downcast_arc::<StructSymbol>() {
        Ok(struct_arc) => {
            let type_params = struct_arc.type_parameters();

            // If not generic, just create a simple struct type
            if type_params.is_empty() {
                return Ty::r#struct(struct_arc, span);
            }

            // Build substitutions by mapping struct's type parameters to provided type arguments IN ORDER
            // This is the key fix: we map by position, not by name
            // So Pair[U, T] maps Pair's first param (T) to U, and Pair's second param (U) to T
            let mut substitutions = Substitutions::new();
            for (param, arg_ty) in type_params.iter().zip(type_args.iter()) {
                substitutions.insert(param.metadata().id(), arg_ty.clone());
            }

            // Fill in any missing type parameters with defaults or inferred type
            for param in type_params {
                let param_id = param.metadata().id();
                if !substitutions.contains(param_id) {
                    // Try to use the parameter's default, resolving UnresolvedPath if needed
                    let default_ty = if let Some(default) = param.default() {
                        if let TyKind::UnresolvedPath { segments } = default.kind() {
                            // Resolve the path using the struct's context
                            match ctx.model.query(ResolveTypePath {
                                path: segments.to_vec(),
                                context: struct_arc.metadata().id(),
                            }) {
                                TypePathResolution::Resolved(resolved_ty) => resolved_ty,
                                _ => Ty::infer(span.clone()), // Fallback to infer if resolution fails
                            }
                        } else {
                            default.clone()
                        }
                    } else {
                        Ty::infer(span.clone())
                    };
                    substitutions.insert(param_id, default_ty);
                }
            }

            Ty::generic_struct(struct_arc, substitutions, span)
        },
        Err(_) => Ty::error(span),
    }
}

/// Create a generic struct type with substitutions inferred from argument types.
///
/// This is used for implicit struct initialization where the type arguments
/// need to be inferred from the argument values' types.
///
/// # Arguments
/// * `struct_symbol` - The struct symbol
/// * `fields` - The struct's fields (in order)
/// * `arguments` - The argument types being passed to the fields
/// * `span` - The span for the created type
pub fn create_generic_struct_type(
    struct_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    fields: &[Arc<dyn Symbol<KestrelLanguage>>],
    arguments: &[Ty],
    span: Span,
) -> Ty {
    let sym_clone = Arc::clone(struct_symbol);

    match sym_clone.downcast_arc::<StructSymbol>() {
        Ok(struct_arc) => {
            let type_params = struct_arc.type_parameters();

            // If not generic, just create a simple struct type
            if type_params.is_empty() {
                return Ty::r#struct(struct_arc, span);
            }

            // Build substitutions by matching field types to argument types
            let mut substitutions = Substitutions::new();

            for (field, arg_ty) in fields.iter().zip(arguments.iter()) {
                // Get the field's declared type
                let field_ty = get_field_type(field);

                // If the field type is a type parameter, map it to the argument type
                if let Some(TyKind::TypeParameter(param)) = field_ty.map(|t| t.kind().clone()) {
                    let param_id = param.metadata().id();
                    // Only insert if this type parameter belongs to this struct
                    if type_params.iter().any(|p| p.metadata().id() == param_id) {
                        substitutions.insert(param_id, arg_ty.clone());
                    }
                }
            }

            // Fill in any missing type parameters with inferred type
            for param in type_params {
                let param_id = param.metadata().id();
                if !substitutions.contains(param_id) {
                    substitutions.insert(param_id, Ty::infer(span.clone()));
                }
            }

            Ty::generic_struct(struct_arc, substitutions, span)
        },
        Err(_) => Ty::error(span),
    }
}

/// Get the type from a field symbol's TypedBehavior
fn get_field_type(field: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<Ty> {
    for behavior in field.metadata().behaviors() {
        if behavior.kind() == KestrelBehaviorKind::Typed
            && let Some(typed) = behavior.as_ref().downcast_ref::<TypedBehavior>()
        {
            return Some(typed.ty().clone());
        }
    }
    None
}

/// Get the container symbol from a type (for member lookup)
pub fn get_type_container(
    ty: &Ty,
    ctx: &BodyResolutionContext,
) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
    match ty.kind() {
        TyKind::Struct { symbol, .. } => Some(symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>),
        TyKind::Enum { symbol, .. } => Some(symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>),
        TyKind::Protocol { symbol, .. } => Some(symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>),
        TyKind::TypeAlias { .. } => {
            // Expand the type alias and recurse to get the container of the underlying type
            let expanded = ty.expand_aliases();
            // Avoid infinite recursion if expand_aliases returns the same type
            if !matches!(expanded.kind(), TyKind::TypeAlias { .. }) {
                get_type_container(&expanded, ctx)
            } else {
                None
            }
        },
        TyKind::SelfType => {
            // Resolve Self to the containing struct/protocol
            // Get the function symbol, then its parent (which should be the struct/protocol)
            let function = ctx.model.query(SymbolFor {
                id: ctx.function_id,
            })?;
            let parent = function.metadata().parent()?;
            match parent.metadata().kind() {
                KestrelSymbolKind::Struct | KestrelSymbolKind::Protocol => Some(parent),
                KestrelSymbolKind::Extension => {
                    // For extension methods, Self refers to the target type
                    // Get the ExtensionTargetBehavior and return the target struct/enum/protocol
                    if let Some(target_beh) =
                        parent.metadata().get_behavior::<ExtensionTargetBehavior>()
                    {
                        match target_beh.target_type().kind() {
                            TyKind::Struct { symbol, .. } => {
                                Some(symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>)
                            },
                            TyKind::Enum { symbol, .. } => {
                                Some(symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>)
                            },
                            TyKind::Protocol { symbol, .. } => {
                                // For protocol extensions, return the protocol symbol
                                // This allows calling protocol methods on self in default implementations
                                Some(symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>)
                            },
                            _ => None,
                        }
                    } else {
                        None
                    }
                },
                KestrelSymbolKind::Field => {
                    // For getters/setters, the hierarchy is Struct -> Field -> Getter/Setter
                    // We need the grandparent (the Struct)
                    let grandparent = parent.metadata().parent()?;
                    match grandparent.metadata().kind() {
                        KestrelSymbolKind::Struct | KestrelSymbolKind::Protocol => {
                            Some(grandparent)
                        },
                        _ => None,
                    }
                },
                _ => None,
            }
        },
        // TypeParameter is handled separately via get_type_parameter_bounds
        // to support multiple protocol bounds
        _ => None,
    }
}

/// Get the where clause from a symbol that can have one (Function, Initializer, Struct, Protocol).
///
/// Returns None if the symbol doesn't have a where clause or can't be downcast.
/// Note: Returns a cloned WhereClause since FunctionSymbol now uses RwLock.
pub fn get_where_clause(symbol: &dyn Symbol<KestrelLanguage>) -> Option<WhereClause> {
    // Try FunctionSymbol
    if let Some(func) = symbol.as_any().downcast_ref::<FunctionSymbol>() {
        return Some(func.where_clause());
    }
    // Try InitializerSymbol
    if let Some(init) = symbol.as_any().downcast_ref::<InitializerSymbol>() {
        return Some(init.where_clause());
    }
    // Try StructSymbol
    if let Some(struc) = symbol.as_any().downcast_ref::<StructSymbol>() {
        return Some(struc.where_clause().clone());
    }
    // Try ProtocolSymbol
    if let Some(proto) = symbol.as_any().downcast_ref::<ProtocolSymbol>() {
        return Some(proto.where_clause().clone());
    }
    None
}

/// Extract bounds for a specific type parameter from a where clause.
///
/// This is a low-level helper that collects all bounds for a given parameter
/// from a single where clause.
fn extract_bounds_for_param(
    where_clause: &WhereClause,
    param_id: semantic_tree::symbol::SymbolId,
) -> Vec<Ty> {
    where_clause
        .bounds_for(param_id)
        .into_iter()
        .cloned()
        .collect()
}

/// Filter and extract only resolved Protocol bounds for a type parameter.
///
/// The where clause stores bounds as Ty::error() placeholders during BUILD phase.
/// This function filters to only include bounds that have been resolved to Protocol types.
/// Unresolved bounds (Ty::error) are skipped - they'll be caught by validation.
fn filter_resolved_bounds(
    where_clause: &WhereClause,
    param_id: semantic_tree::symbol::SymbolId,
) -> Vec<Ty> {
    extract_bounds_for_param(where_clause, param_id)
        .into_iter()
        .filter(|ty| matches!(ty.kind(), TyKind::Protocol { .. }))
        .collect()
}

/// Get the protocol bounds for a type parameter from a specific context.
///
/// This is used during body resolution where we know which function we're in.
///
/// Note: The where clause bounds are stored as Ty::error() placeholders during
/// the BUILD phase. We need to resolve them dynamically here because the
/// current implementation doesn't update the stored bounds during BIND.
pub fn get_type_parameter_bounds_from_context(
    type_param: &Arc<TypeParameterSymbol>,
    ctx: &mut BodyResolutionContext,
) -> Vec<Ty> {
    get_type_parameter_bounds_by_id(type_param.metadata().id(), ctx)
}

/// Get the protocol bounds for a type parameter by its symbol ID.
///
/// This variant takes a SymbolId directly, useful when we don't have an
/// Arc<TypeParameterSymbol> available.
///
/// Also validates that bounds don't use generic protocols (like `Container[E]`),
/// emitting an error for any such bounds found.
pub fn get_type_parameter_bounds_by_id(
    param_id: SymbolId,
    ctx: &mut BodyResolutionContext,
) -> Vec<Ty> {
    let mut bounds = Vec::new();

    // First, check the context's where clause. This is important for subscripts
    // where the where clause is attached to the subscript but the function_id
    // points to the getter/setter (which don't have where clauses themselves).
    bounds.extend(filter_resolved_bounds(ctx.where_clause(), param_id));

    // Also check the function symbol and its parent for additional where clauses
    if let Some(function) = ctx.model.query(SymbolFor {
        id: ctx.function_id,
    }) {
        // Check function's where clause
        if let Some(where_clause) = get_where_clause(function.as_ref()) {
            bounds.extend(filter_resolved_bounds(&where_clause, param_id));
        }

        // Also check parent (struct/protocol/extension) where clause
        if let Some(parent) = function.metadata().parent() {
            // For extensions, get the combined where clause from ExtensionTargetBehavior
            // This includes both inherited struct constraints AND extension's own constraints
            if parent.metadata().kind() == KestrelSymbolKind::Extension {
                if let Some(target_beh) =
                    parent.metadata().get_behavior::<ExtensionTargetBehavior>()
                {
                    let where_clause = target_beh.where_clause();
                    bounds.extend(filter_resolved_bounds(where_clause, param_id));
                }
            } else if let Some(where_clause) = get_where_clause(parent.as_ref()) {
                bounds.extend(filter_resolved_bounds(&where_clause, param_id));
            }
        }
    }

    bounds
}

/// Check if a constraint path matches an associated type with its container.
///
/// For example, for `where I.Item: Protocol`:
/// - `path` would be `["I", "Item"]`
/// - `assoc_name` would be `"Item"`
/// - `container` would be the type for `I`
///
/// This function verifies that:
/// 1. The last segment of the path matches the associated type name
/// 2. The preceding segments match the container type chain
fn path_matches_associated_type(
    path: &[String],
    assoc_name: &str,
    container: Option<&Ty>,
) -> bool {
    if path.is_empty() {
        return false;
    }

    // Last segment must match the associated type name
    if path.last() != Some(&assoc_name.to_string()) {
        return false;
    }

    // If path has only one segment (e.g., ["Item"]), it matches any container
    // This is for simple constraints like `where Item: Protocol` in protocol extensions
    if path.len() == 1 {
        return true;
    }

    // For multi-segment paths, verify the container chain matches
    // E.g., for path ["I", "Item"], container should be type parameter "I"
    let prefix = &path[..path.len() - 1];

    // Check if container matches the path prefix
    match container {
        None => {
            // No container means this is a top-level associated type in a protocol
            // Only single-segment paths should match (handled above)
            false
        }
        Some(container_ty) => {
            // Recursively check the container chain
            container_matches_path(container_ty, prefix)
        }
    }
}

/// Check if a container type matches a path prefix.
///
/// For example, for path prefix ["I"]:
/// - A type parameter named "I" would match
///
/// For path prefix ["I", "Item"]:
/// - An associated type "Item" with container type parameter "I" would match
fn container_matches_path(container: &Ty, path: &[String]) -> bool {
    if path.is_empty() {
        return true;
    }

    match container.kind() {
        TyKind::TypeParameter(type_param) => {
            // Type parameter matches if it's the only segment and names match
            path.len() == 1 && path[0] == type_param.metadata().name().value
        }
        TyKind::AssociatedType { symbol, container: nested_container } => {
            // Associated type matches if:
            // 1. Last segment matches the associated type name
            // 2. Remaining prefix matches the nested container
            if path.is_empty() {
                return false;
            }
            let assoc_name = symbol.metadata().name().value.clone();
            if path.last() != Some(&assoc_name) {
                return false;
            }
            let prefix = &path[..path.len() - 1];
            match nested_container {
                Some(c) => container_matches_path(c, prefix),
                None => prefix.is_empty(),
            }
        }
        TyKind::SelfType => {
            // Self matches if it's the only segment and named "Self"
            path.len() == 1 && path[0] == "Self"
        }
        _ => {
            // Other types don't match path prefixes
            false
        }
    }
}

/// Get the protocol bounds for an associated type from context.
///
/// This combines:
/// 1. Direct bounds from the associated type symbol (e.g., `type Item: Protocol`)
/// 2. Bounds from SelfBound constraints in the context's where clause (e.g., `where Item: Protocol`)
///
/// The `container` parameter is used to verify that nested path constraints like
/// `where I.Item: Protocol` match the actual associated type being queried.
///
/// This is analogous to `get_type_parameter_bounds_from_context` but for associated types.
pub fn get_associated_type_bounds_from_context(
    assoc_type: &Arc<AssociatedTypeSymbol>,
    container: Option<&Ty>,
    ctx: &mut BodyResolutionContext,
) -> Vec<Ty> {
    let mut bounds = Vec::new();
    let assoc_name = assoc_type.metadata().name().value.clone();

    // 1. Get direct bounds from the associated type symbol
    if let Some(direct_bounds) = assoc_type.bounds() {
        for bound in direct_bounds {
            if matches!(bound.kind(), TyKind::Protocol { .. }) {
                bounds.push(bound.clone());
            }
        }
    }

    // 2. Check context's where clause for SelfBound constraints
    // These are constraints like `where Item: Comparable` in protocol extensions
    // or nested constraints like `where I.Item: Iterator`
    for constraint in ctx.where_clause().constraints() {
        if let Constraint::SelfBound {
            associated_type_path,
            bounds: self_bounds,
            ..
        } = constraint
        {
            // Match constraints on this associated type, verifying the full path
            // Single-level: where Item: Protocol → path = ["Item"]
            // Nested: where I.Item: Protocol → path = ["I", "Item"]
            if path_matches_associated_type(associated_type_path, &assoc_name, container) {
                for bound in self_bounds {
                    if matches!(bound.kind(), TyKind::Protocol { .. }) {
                        bounds.push(bound.clone());
                    }
                }
            }
        }
        if let Constraint::InheritedAssociatedTypeBound { path, bounds: assoc_bounds, .. } =
            constraint
        {
            // Convert dot-separated path to vec for matching
            let path_segments: Vec<String> = path.split('.').map(|s| s.to_string()).collect();
            if path_matches_associated_type(&path_segments, &assoc_name, container) {
                for bound in assoc_bounds {
                    if matches!(bound.kind(), TyKind::Protocol { .. }) {
                        bounds.push(bound.clone());
                    }
                }
            }
        }
        if let Constraint::TypeBound {
            param: None,
            param_name,
            bounds: param_bounds,
            ..
        } = constraint
        {
            // TypeBound with param: None is for simple associated type constraints
            // Match when:
            // 1. Name matches AND container is None (top-level associated type)
            // 2. Name matches AND container is Self (protocol extension associated type)
            // This handles `where Item: Protocol` in protocol extensions where Item
            // has container Self (since it's implicitly Self.Item)
            let container_is_self_or_none = match container {
                None => true,
                Some(ty) => matches!(ty.kind(), TyKind::SelfType),
            };

            if param_name == &assoc_name && container_is_self_or_none {
                for bound in param_bounds {
                    if matches!(bound.kind(), TyKind::Protocol { .. }) {
                        bounds.push(bound.clone());
                    }
                }
            }
        }
    }

    // 3. Also check function's parent where clause (for struct, enum, and extension)
    // The context's where_clause contains the function's own constraints,
    // but we also need to check the parent's where clause for inherited constraints
    if let Some(function) = ctx.model.query(SymbolFor {
        id: ctx.function_id,
    }) {
        if let Some(parent) = function.metadata().parent() {
            // Get the parent's where clause depending on its kind
            let parent_where_clause = match parent.metadata().kind() {
                KestrelSymbolKind::Extension => {
                    parent
                        .metadata()
                        .get_behavior::<ExtensionTargetBehavior>()
                        .map(|t| t.where_clause().clone())
                }
                KestrelSymbolKind::Struct | KestrelSymbolKind::Enum => {
                    parent
                        .metadata()
                        .get_behavior::<GenericsBehavior>()
                        .map(|g| g.where_clause().clone())
                }
                _ => None,
            };

            if let Some(where_clause) = parent_where_clause {
                for constraint in where_clause.constraints() {
                    if let Constraint::SelfBound {
                        associated_type_path,
                        bounds: self_bounds,
                        ..
                    } = constraint
                    {
                        if path_matches_associated_type(associated_type_path, &assoc_name, container) {
                            for bound in self_bounds {
                                if let TyKind::Protocol { symbol, .. } = bound.kind() {
                                    // Check if this protocol is already in bounds
                                    let already_present = bounds.iter().any(|b| {
                                        if let TyKind::Protocol {
                                            symbol: existing, ..
                                        } = b.kind()
                                        {
                                            existing.metadata().id() == symbol.metadata().id()
                                        } else {
                                            false
                                        }
                                    });
                                    if !already_present {
                                        bounds.push(bound.clone());
                                    }
                                }
                            }
                        }
                    }
                    if let Constraint::InheritedAssociatedTypeBound {
                        path,
                        bounds: assoc_bounds,
                        ..
                    } = constraint
                    {
                        let path_segments: Vec<String> = path.split('.').map(|s| s.to_string()).collect();
                        if path_matches_associated_type(&path_segments, &assoc_name, container) {
                            for bound in assoc_bounds {
                                if let TyKind::Protocol { symbol, .. } = bound.kind() {
                                    let already_present = bounds.iter().any(|b| {
                                        if let TyKind::Protocol {
                                            symbol: existing, ..
                                        } = b.kind()
                                        {
                                            existing.metadata().id() == symbol.metadata().id()
                                        } else {
                                            false
                                        }
                                    });
                                    if !already_present {
                                        bounds.push(bound.clone());
                                    }
                                }
                            }
                        }
                    }
                    if let Constraint::TypeBound {
                        param: None,
                        param_name,
                        bounds: param_bounds,
                        ..
                    } = constraint
                    {
                        // TypeBound with param: None matches simple associated type constraints
                        // Match when container is None or Self (for protocol extensions)
                        let container_is_self_or_none = match container {
                            None => true,
                            Some(ty) => matches!(ty.kind(), TyKind::SelfType),
                        };

                        if param_name == &assoc_name && container_is_self_or_none {
                            for bound in param_bounds {
                                if let TyKind::Protocol { symbol, .. } = bound.kind() {
                                    let already_present = bounds.iter().any(|b| {
                                        if let TyKind::Protocol {
                                            symbol: existing, ..
                                        } = b.kind()
                                        {
                                            existing.metadata().id() == symbol.metadata().id()
                                        } else {
                                            false
                                        }
                                    });
                                    if !already_present {
                                        bounds.push(bound.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    bounds
}

/// Substitute Self type with a replacement type recursively.
///
/// This is used when looking up methods on constrained type parameters.
/// Protocol methods use `Self` to refer to the conforming type, which
/// needs to be replaced with the actual receiver type (e.g., `T`).
pub fn substitute_self(ty: &Ty, replacement: &Ty) -> Ty {
    ty.substitute_self(replacement)
}

/// Substitute type parameters in a type with their concrete types.
///
/// This recursively traverses a type and replaces any TypeParameter with
/// the corresponding concrete type from the substitutions map.
pub fn substitute_type(ty: &Ty, substitutions: &Substitutions) -> Ty {
    match ty.kind() {
        TyKind::TypeParameter(param) => {
            let param_id = param.metadata().id();
            substitutions
                .get(param_id)
                .cloned()
                .unwrap_or_else(|| ty.clone())
        },
        // Note: Array[T] struct types are handled by the Struct case above
        TyKind::Tuple(elements) => {
            let new_elements: Vec<Ty> = elements
                .iter()
                .map(|e| substitute_type(e, substitutions))
                .collect();
            Ty::tuple(new_elements, ty.span().clone())
        },
        TyKind::Function {
            params,
            return_type,
        } => {
            let new_params: Vec<Ty> = params
                .iter()
                .map(|p| substitute_type(p, substitutions))
                .collect();
            let new_return = substitute_type(return_type, substitutions);
            Ty::function(new_params, new_return, ty.span().clone())
        },
        TyKind::Struct {
            symbol,
            substitutions: inner_subs,
        } => {
            // Apply our substitutions to the inner substitutions
            let mut new_subs = Substitutions::new();
            for (id, inner_ty) in inner_subs.iter() {
                new_subs.insert(*id, substitute_type(inner_ty, substitutions));
            }
            Ty::generic_struct(symbol.clone(), new_subs, ty.span().clone())
        },
        TyKind::Enum {
            symbol,
            substitutions: inner_subs,
        } => {
            // Apply our substitutions to the inner substitutions
            let mut new_subs = Substitutions::new();
            for (id, inner_ty) in inner_subs.iter() {
                new_subs.insert(*id, substitute_type(inner_ty, substitutions));
            }
            Ty::generic_enum(symbol.clone(), new_subs, ty.span().clone())
        },
        TyKind::TypeAlias {
            symbol,
            substitutions: inner_subs,
        } => {
            // Apply our substitutions to the inner substitutions
            // This is important for type aliases like T? (OptionalTypeOperator[T])
            let mut new_subs = Substitutions::new();
            for (id, inner_ty) in inner_subs.iter() {
                new_subs.insert(*id, substitute_type(inner_ty, substitutions));
            }
            Ty::generic_type_alias(symbol.clone(), new_subs, ty.span().clone())
        },
        // For simple types, just return a clone
        _ => ty.clone(),
    }
}

/// Format a symbol kind for error messages
pub fn format_symbol_kind(kind: KestrelSymbolKind) -> String {
    match kind {
        KestrelSymbolKind::AssociatedType => "associated type".to_string(),
        KestrelSymbolKind::Deinit => "deinit".to_string(),
        KestrelSymbolKind::Enum => "enum".to_string(),
        KestrelSymbolKind::EnumCase => "enum case".to_string(),
        KestrelSymbolKind::Extension => "extension".to_string(),
        KestrelSymbolKind::Field => "field".to_string(),
        KestrelSymbolKind::Function => "function".to_string(),
        KestrelSymbolKind::Import => "import".to_string(),
        KestrelSymbolKind::Initializer => "initializer".to_string(),
        KestrelSymbolKind::Module => "module".to_string(),
        KestrelSymbolKind::Protocol => "protocol".to_string(),
        KestrelSymbolKind::SourceFile => "source file".to_string(),
        KestrelSymbolKind::Struct => "struct".to_string(),
        KestrelSymbolKind::TypeAlias => "type alias".to_string(),
        KestrelSymbolKind::TypeParameter => "type parameter".to_string(),
        KestrelSymbolKind::Getter => "getter".to_string(),
        KestrelSymbolKind::Setter => "setter".to_string(),
        KestrelSymbolKind::Subscript => "subscript".to_string(),
    }
}

// =============================================================================
// Type Argument Inference
// =============================================================================

/// Infer type arguments for a generic function call from the argument types.
///
/// This function matches argument types against parameter types and infers
/// substitutions for type parameters. For example:
/// - `func getHash[T](value: T) -> Int` called with `Point` argument
/// - Infers `T = Point`
///
/// # Arguments
/// * `type_params` - The type parameters of the generic function
/// * `callable` - The CallableBehavior containing parameter types
/// * `arg_types` - The types of the arguments being passed
///
/// # Returns
/// Substitutions mapping type parameter IDs to inferred concrete types
pub fn infer_type_arguments(
    type_params: &[Arc<TypeParameterSymbol>],
    callable: &CallableBehavior,
    arg_types: &[Ty],
) -> Substitutions {
    let mut substitutions = Substitutions::new();

    let params = callable.parameters();

    // Match each argument type against the corresponding parameter type
    for (param, arg_ty) in params.iter().zip(arg_types.iter()) {
        // Recursively extract type parameter mappings
        infer_from_type(&param.ty, arg_ty, type_params, &mut substitutions);
    }

    substitutions
}

/// Recursively infer type parameter mappings by matching a parameter type against an argument type.
fn infer_from_type(
    param_ty: &Ty,
    arg_ty: &Ty,
    type_params: &[Arc<TypeParameterSymbol>],
    substitutions: &mut Substitutions,
) {
    match param_ty.kind() {
        // If parameter is a type parameter, map it to the argument type
        TyKind::TypeParameter(param) => {
            let param_id = param.metadata().id();
            // Only infer if this type parameter is in our list
            if type_params.iter().any(|tp| tp.metadata().id() == param_id) {
                // Only insert if not already mapped (first match wins)
                if !substitutions.contains(param_id) {
                    substitutions.insert(param_id, arg_ty.clone());
                }
            }
        },

        // Note: Array[T] types are handled by the Struct case above (via substitutions)

        // For tuple types, recurse into each element
        TyKind::Tuple(elems) => {
            if let TyKind::Tuple(arg_elems) = arg_ty.kind() {
                for (pe, ae) in elems.iter().zip(arg_elems.iter()) {
                    infer_from_type(pe, ae, type_params, substitutions);
                }
            }
        },

        // For function types, recurse into params and return type
        TyKind::Function {
            params,
            return_type,
        } => {
            if let TyKind::Function {
                params: arg_params,
                return_type: arg_ret,
            } = arg_ty.kind()
            {
                for (pp, ap) in params.iter().zip(arg_params.iter()) {
                    infer_from_type(pp, ap, type_params, substitutions);
                }
                infer_from_type(return_type, arg_ret, type_params, substitutions);
            }
        },

        // For struct types with substitutions, match the inner type arguments
        TyKind::Struct {
            symbol: param_struct,
            substitutions: param_subs,
        } => {
            if let TyKind::Struct {
                symbol: arg_struct,
                substitutions: arg_subs,
            } = arg_ty.kind()
            {
                // Only if same struct
                if param_struct.metadata().id() == arg_struct.metadata().id() {
                    // Match substitutions
                    for (id, param_sub_ty) in param_subs.iter() {
                        if let Some(arg_sub_ty) = arg_subs.get(*id) {
                            infer_from_type(param_sub_ty, arg_sub_ty, type_params, substitutions);
                        }
                    }
                }
            }
        },

        // For enum types with substitutions, match the inner type arguments
        TyKind::Enum {
            symbol: param_enum,
            substitutions: param_subs,
        } => {
            if let TyKind::Enum {
                symbol: arg_enum,
                substitutions: arg_subs,
            } = arg_ty.kind()
            {
                // Only if same enum
                if param_enum.metadata().id() == arg_enum.metadata().id() {
                    // Match substitutions
                    for (id, param_sub_ty) in param_subs.iter() {
                        if let Some(arg_sub_ty) = arg_subs.get(*id) {
                            infer_from_type(param_sub_ty, arg_sub_ty, type_params, substitutions);
                        }
                    }
                }
            }
        },

        // Other types don't contribute to inference
        _ => {},
    }
}

// =============================================================================
// Call-Site Constraint Verification
// =============================================================================

use crate::diagnostics::ConstraintNotSatisfiedError;

/// Verify that type arguments satisfy the constraints of a generic callable.
///
/// This function checks that each concrete type argument satisfies the protocol
/// bounds declared in the where clause.
///
/// # Arguments
/// * `type_params` - The type parameters of the callable
/// * `type_args` - The concrete type arguments being passed
/// * `where_clause` - The where clause containing constraints
/// * `call_span` - Span of the call site for error reporting
/// * `model` - Semantic model for symbol lookup
/// * `diagnostics` - Diagnostic context for reporting errors
///
/// # Returns
/// `true` if all constraints are satisfied, `false` otherwise
pub fn verify_type_argument_constraints(
    type_params: &[Arc<TypeParameterSymbol>],
    type_args: &[Ty],
    where_clause: &WhereClause,
    call_span: Span,
    model: &kestrel_semantic_model::SemanticModel,
    diagnostics: &mut kestrel_reporting::DiagnosticContext,
) -> bool {
    let mut all_satisfied = true;

    for (param, arg) in type_params.iter().zip(type_args.iter()) {
        // Skip constraint checking for poison types to avoid cascading errors
        if arg.is_poison() {
            continue;
        }

        let param_id = param.metadata().id();
        let bounds_with_spans = where_clause.bounds_for_with_span(param_id);

        for (bound, constraint_span) in bounds_with_spans {
            if !type_satisfies_bound(arg, bound, model) {
                // Report constraint not satisfied
                let param_name = param.metadata().name().value.clone();
                let type_name = arg.to_string();
                let constraint_name = bound.to_string();

                let error = ConstraintNotSatisfiedError {
                    call_span: call_span.clone(),
                    type_name,
                    constraint_name,
                    type_param_name: param_name,
                    constraint_span: Some(constraint_span.clone()),
                };
                diagnostics.add_diagnostic(error.into_diagnostic());
                all_satisfied = false;
            }
        }
    }

    all_satisfied
}

/// Check if a type satisfies a protocol bound.
///
/// This checks if a concrete type conforms to a protocol, either directly,
/// through extensions, or transitively through protocol extensions.
///
/// Uses the TypeOracle::conforms_to method which implements the full conformance
/// checking logic including transitive conformance through protocol extensions.
pub fn type_satisfies_bound(
    ty: &Ty,
    bound: &Ty,
    model: &kestrel_semantic_model::SemanticModel,
) -> bool {
    use kestrel_semantic_type_inference::TypeOracle;

    // Get the protocol from the bound
    let TyKind::Protocol {
        symbol: required_proto,
        ..
    } = bound.kind()
    else {
        // Bound is not a protocol - shouldn't happen with proper validation
        return false;
    };

    // Inference placeholders - optimistically assume they will satisfy bounds
    // once resolved. Type inference will catch actual violations later.
    if matches!(ty.kind(), TyKind::Infer) {
        return true;
    }

    // Use the TypeOracle::conforms_to method which handles:
    // - Direct conformances
    // - Extension conformances
    // - Transitive conformance through protocol extensions
    // - Protocol inheritance
    model.conforms_to(ty, required_proto.metadata().id())
}

/// Replace any remaining type parameters in a type with inference placeholders.
///
/// This is used when initializer parameter types might contain type parameters
/// that weren't in the substitution map (due to symbol ID mismatches between
/// how type parameters are stored in different phases).
#[allow(dead_code)]
pub fn replace_unsubstituted_type_params(ty: &Ty, span: &Span) -> Ty {
    match ty.kind() {
        // Type parameter - replace with inference placeholder
        TyKind::TypeParameter(_) => Ty::infer(span.clone()),

        // Composite types - recursively replace
        TyKind::Tuple(elements) => {
            let new_elements: Vec<Ty> = elements
                .iter()
                .map(|e| replace_unsubstituted_type_params(e, span))
                .collect();
            Ty::tuple(new_elements, ty.span().clone())
        },

        TyKind::Pointer(element) => {
            let new_element = replace_unsubstituted_type_params(element, span);
            Ty::pointer(new_element, ty.span().clone())
        },

        TyKind::Function {
            params,
            return_type,
        } => {
            let new_params: Vec<Ty> = params
                .iter()
                .map(|p| replace_unsubstituted_type_params(p, span))
                .collect();
            let new_return = replace_unsubstituted_type_params(return_type, span);
            Ty::function(new_params, new_return, ty.span().clone())
        },

        TyKind::Struct {
            symbol,
            substitutions,
        } => {
            let mut new_subs = Substitutions::new();
            for (key, sub_ty) in substitutions.iter() {
                new_subs.insert(*key, replace_unsubstituted_type_params(sub_ty, span));
            }
            Ty::generic_struct(symbol.clone(), new_subs, ty.span().clone())
        },

        TyKind::Enum {
            symbol,
            substitutions,
        } => {
            let mut new_subs = Substitutions::new();
            for (key, sub_ty) in substitutions.iter() {
                new_subs.insert(*key, replace_unsubstituted_type_params(sub_ty, span));
            }
            Ty::generic_enum(symbol.clone(), new_subs, ty.span().clone())
        },

        TyKind::Protocol {
            symbol,
            substitutions,
        } => {
            let mut new_subs = Substitutions::new();
            for (key, sub_ty) in substitutions.iter() {
                new_subs.insert(*key, replace_unsubstituted_type_params(sub_ty, span));
            }
            Ty::generic_protocol(symbol.clone(), new_subs, ty.span().clone())
        },

        // Other types - return as-is
        _ => ty.clone(),
    }
}

/// Replace type parameters with inference placeholders, except for those in the preserved set.
///
/// This is used when initializer parameter types might contain type parameters
/// that weren't in the substitution map, but we want to preserve type parameters
/// that came from explicit type arguments (e.g., Pointer[T] where T is from the caller's scope).
pub fn replace_type_params_except(
    ty: &Ty,
    preserved: &std::collections::HashSet<SymbolId>,
    span: &Span,
) -> Ty {
    match ty.kind() {
        // Type parameter - replace with inference placeholder unless preserved
        TyKind::TypeParameter(tp) => {
            if preserved.contains(&tp.metadata().id()) {
                ty.clone() // Preserve this type parameter
            } else {
                Ty::infer(span.clone()) // Replace with inference
            }
        },

        // Composite types - recursively process
        TyKind::Tuple(elements) => {
            let new_elements: Vec<Ty> = elements
                .iter()
                .map(|e| replace_type_params_except(e, preserved, span))
                .collect();
            Ty::tuple(new_elements, ty.span().clone())
        },

        TyKind::Pointer(element) => {
            let new_element = replace_type_params_except(element, preserved, span);
            Ty::pointer(new_element, ty.span().clone())
        },

        TyKind::Function {
            params,
            return_type,
        } => {
            let new_params: Vec<Ty> = params
                .iter()
                .map(|p| replace_type_params_except(p, preserved, span))
                .collect();
            let new_return = replace_type_params_except(return_type, preserved, span);
            Ty::function(new_params, new_return, ty.span().clone())
        },

        TyKind::Struct {
            symbol,
            substitutions,
        } => {
            let mut new_subs = Substitutions::new();
            for (key, sub_ty) in substitutions.iter() {
                new_subs.insert(*key, replace_type_params_except(sub_ty, preserved, span));
            }
            Ty::generic_struct(symbol.clone(), new_subs, ty.span().clone())
        },

        TyKind::Enum {
            symbol,
            substitutions,
        } => {
            let mut new_subs = Substitutions::new();
            for (key, sub_ty) in substitutions.iter() {
                new_subs.insert(*key, replace_type_params_except(sub_ty, preserved, span));
            }
            Ty::generic_enum(symbol.clone(), new_subs, ty.span().clone())
        },

        TyKind::Protocol {
            symbol,
            substitutions,
        } => {
            let mut new_subs = Substitutions::new();
            for (key, sub_ty) in substitutions.iter() {
                new_subs.insert(*key, replace_type_params_except(sub_ty, preserved, span));
            }
            Ty::generic_protocol(symbol.clone(), new_subs, ty.span().clone())
        },

        // Other types - return as-is
        _ => ty.clone(),
    }
}

// =============================================================================
// Type-Directed Conformance Helpers
// =============================================================================

/// Get the ImplementsBehavior from a symbol if it has one.
///
/// This behavior links a struct method/initializer to the protocol method it implements.
#[allow(dead_code)]
pub fn get_implements_behavior(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Option<ImplementsBehavior> {
    for behavior in symbol.metadata().behaviors() {
        if behavior.kind() == KestrelBehaviorKind::Implements
            && let Some(implements) = behavior.as_ref().downcast_ref::<ImplementsBehavior>()
        {
            return Some(implements.clone());
        }
    }
    None
}

/// Find all conformances for a specific protocol from a list of conformances.
///
/// Returns conformances where the protocol symbol ID matches the given protocol ID.
/// For example, given conformances `[Convertible[Int8], Equatable, Convertible[Int32]]`
/// and protocol ID for `Convertible`, returns `[Convertible[Int8], Convertible[Int32]]`.
#[allow(dead_code)]
pub fn find_conformances_for_protocol(conformances: &[Ty], protocol_id: SymbolId) -> Vec<&Ty> {
    conformances
        .iter()
        .filter(|ty| {
            if let TyKind::Protocol { symbol, .. } = ty.kind() {
                symbol.metadata().id() == protocol_id
            } else {
                false
            }
        })
        .collect()
}

/// Get the type argument at a specific index from a protocol conformance.
///
/// For a conformance like `Convertible[Int32]`, this extracts the `Int32` type argument.
/// The index refers to the position of the type parameter in the protocol definition.
///
/// Returns `None` if:
/// - The type is not a protocol
/// - The protocol has no type parameters
/// - The index is out of bounds
#[allow(dead_code)]
pub fn get_conformance_type_arg(conformance_ty: &Ty, param_index: usize) -> Option<Ty> {
    if let TyKind::Protocol {
        symbol,
        substitutions,
    } = conformance_ty.kind()
    {
        // Get the protocol's type parameters to find the ID at the given index
        let type_params = symbol.type_parameters();
        let param = type_params.get(param_index)?;
        let param_id = param.metadata().id();

        // Look up the substituted type for this parameter
        substitutions.get(param_id).cloned()
    } else {
        None
    }
}

/// Find the best matching initializer using type-directed selection.
///
/// When multiple initializers match by label/arity (e.g., multiple `init(from:)` from
/// different `Convertible[X]` conformances), this function selects the one whose
/// parameter type matches the actual argument type.
///
/// # Arguments
/// * `candidates` - Initializers that match by label/arity
/// * `arg_types` - The actual argument types being passed
/// * `_struct_symbol` - The struct being instantiated (reserved for future use)
///
/// # Returns
/// The index of the best matching initializer, or `None` if:
/// - No type-directed match is found (falls back to first match)
/// - Multiple exact matches exist (ambiguity - should report error)
pub fn find_type_directed_match(
    candidates: &[(usize, &Arc<dyn Symbol<KestrelLanguage>>, CallableBehavior)],
    arg_types: &[Ty],
    _struct_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Option<usize> {
    // Skip if no arguments to match on
    if arg_types.is_empty() {
        return None;
    }

    let mut matches: Vec<usize> = Vec::new();

    for (candidate_idx, (_, _init_sym, callable)) in candidates.iter().enumerate() {
        let params = callable.parameters();
        if params.is_empty() {
            continue;
        }

        // Check if the first parameter type matches the first argument type
        // This is a simple direct type comparison - if the init's parameter type
        // is assignable from the argument type, it's a match
        let param_ty = &params[0].ty;

        if arg_types[0].is_assignable_to(param_ty) {
            // Push the index within candidates, not the original index
            matches.push(candidate_idx);
        }
    }

    // Return the single match, or None if zero or multiple matches
    if matches.len() == 1 {
        Some(matches[0])
    } else {
        None // Either no matches (fall back to first) or ambiguous (should error)
    }
}
