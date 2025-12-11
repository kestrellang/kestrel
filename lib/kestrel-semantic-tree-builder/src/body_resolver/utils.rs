//! Shared utilities for body resolution.
//!
//! This module contains helper functions used across multiple body resolution
//! modules, including type formatting, signature matching, and behavior lookups.

use std::sync::Arc;

use kestrel_reporting::IntoDiagnostic;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::expr::{ExprKind, Expression};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Substitutions, Ty, TyKind, WhereClause};
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxKind;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::diagnostics::{TypeParameterCannotBeUsedAsValueError, UnsupportedGenericProtocolBoundError};
use super::context::BodyResolutionContext;

/// Check if a syntax kind is an expression kind
pub fn is_expression_kind(kind: SyntaxKind) -> bool {
    matches!(kind,
        SyntaxKind::Expression
        | SyntaxKind::ExprUnit
        | SyntaxKind::ExprInteger
        | SyntaxKind::ExprFloat
        | SyntaxKind::ExprString
        | SyntaxKind::ExprBool
        | SyntaxKind::ExprArray
        | SyntaxKind::ExprTuple
        | SyntaxKind::ExprGrouping
        | SyntaxKind::ExprPath
        | SyntaxKind::ExprUnary
        | SyntaxKind::ExprPostfix
        | SyntaxKind::ExprBinary
        | SyntaxKind::ExprNull
        | SyntaxKind::ExprCall
        | SyntaxKind::ExprAssignment
        | SyntaxKind::ExprIf
        | SyntaxKind::ExprWhile
        | SyntaxKind::ExprLoop
        | SyntaxKind::ExprBreak
        | SyntaxKind::ExprContinue
        | SyntaxKind::ExprReturn
        | SyntaxKind::ExprTupleIndex
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
        let type_param_name = ctx.db.symbol_by_id(*symbol_id)
            .map(|s| s.metadata().name().value.clone())
            .unwrap_or_else(|| "T".to_string());

        let error = TypeParameterCannotBeUsedAsValueError {
            span: expr.span.clone(),
            type_param_name,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic(ctx.file_id));

        return Expression::error(expr.span);
    }
    expr
}

/// Check if a callable signature matches the given arity and labels
pub fn matches_signature(callable: &CallableBehavior, arity: usize, labels: &[Option<String>]) -> bool {
    let params = callable.parameters();

    // Check arity
    if params.len() != arity {
        return false;
    }

    // Check labels match
    for (param, label) in params.iter().zip(labels.iter()) {
        let param_label = param.external_label();
        let label_ref = label.as_deref();
        if param_label != label_ref {
            return false;
        }
    }

    true
}

/// Get the CallableBehavior from a symbol if it has one
pub fn get_callable_behavior(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<CallableBehavior> {
    for behavior in symbol.metadata().behaviors() {
        if behavior.kind() == KestrelBehaviorKind::Callable {
            if let Some(callable) = behavior.as_ref().downcast_ref::<CallableBehavior>() {
                return Some(callable.clone());
            }
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
        Ok(struct_arc) => Ty::r#struct(struct_arc, span),
        Err(_) => {
            // This shouldn't happen if we're calling this on a struct symbol
            Ty::error(span)
        }
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
/// * `_ctx` - The body resolution context (unused but kept for API consistency)
pub fn create_struct_type_with_type_args(
    struct_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    type_args: &[Ty],
    span: Span,
    _ctx: &super::context::BodyResolutionContext,
) -> Ty {
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

            // Fill in any missing type parameters with inferred type
            for param in type_params {
                let param_id = param.metadata().id();
                if !substitutions.contains(param_id) {
                    substitutions.insert(param_id, Ty::type_var(span.clone()));
                }
            }

            Ty::generic_struct(struct_arc, substitutions, span)
        }
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
                    substitutions.insert(param_id, Ty::type_var(span.clone()));
                }
            }

            Ty::generic_struct(struct_arc, substitutions, span)
        }
        Err(_) => Ty::error(span),
    }
}

/// Get the type from a field symbol's TypedBehavior
fn get_field_type(field: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<Ty> {
    for behavior in field.metadata().behaviors() {
        if behavior.kind() == KestrelBehaviorKind::Typed {
            if let Some(typed) = behavior.as_ref().downcast_ref::<TypedBehavior>() {
                return Some(typed.ty().clone());
            }
        }
    }
    None
}

/// Get the container symbol from a type (for member lookup)
pub fn get_type_container(ty: &Ty, ctx: &BodyResolutionContext) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
    match ty.kind() {
        TyKind::Struct { symbol, .. } => Some(symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>),
        TyKind::Protocol { symbol, .. } => Some(symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>),
        TyKind::SelfType => {
            // Resolve Self to the containing struct/protocol
            // Get the function symbol, then its parent (which should be the struct/protocol)
            let function = ctx.db.symbol_by_id(ctx.function_id)?;
            let parent = function.metadata().parent()?;
            match parent.metadata().kind() {
                KestrelSymbolKind::Struct | KestrelSymbolKind::Protocol => Some(parent),
                KestrelSymbolKind::Extension => {
                    // For extension methods, Self refers to the target type
                    // Get the ExtensionTargetBehavior and return the target struct
                    use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
                    if let Some(target_beh) = parent.extension_target_behavior() {
                        match target_beh.target_type().kind() {
                            TyKind::Struct { symbol, .. } => {
                                Some(symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>)
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        // TypeParameter is handled separately via get_type_parameter_bounds
        // to support multiple protocol bounds
        _ => None,
    }
}

/// Get the where clause from a symbol that can have one (Function, Struct, Protocol).
///
/// Returns None if the symbol doesn't have a where clause or can't be downcast.
/// Note: Returns a cloned WhereClause since FunctionSymbol now uses RwLock.
pub fn get_where_clause(symbol: &dyn Symbol<KestrelLanguage>) -> Option<WhereClause> {
    // Try FunctionSymbol
    if let Some(func) = symbol.as_any().downcast_ref::<FunctionSymbol>() {
        return Some(func.where_clause());
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
fn extract_bounds_for_param(where_clause: &WhereClause, param_id: semantic_tree::symbol::SymbolId) -> Vec<Ty> {
    where_clause.bounds_for(param_id).into_iter().cloned().collect()
}

/// Filter and extract only resolved Protocol bounds for a type parameter.
///
/// The where clause stores bounds as Ty::error() placeholders during BUILD phase.
/// This function filters to only include bounds that have been resolved to Protocol types.
/// Unresolved bounds (Ty::error) are skipped - they'll be caught by validation.
fn filter_resolved_bounds(where_clause: &WhereClause, param_id: semantic_tree::symbol::SymbolId) -> Vec<Ty> {
    extract_bounds_for_param(where_clause, param_id)
        .into_iter()
        .filter(|ty| matches!(ty.kind(), TyKind::Protocol { .. }))
        .collect()
}

/// Get the protocol bounds for a type parameter from the current resolution context.
///
/// This looks up the where clause from the current function (and its parent struct/protocol)
/// to find all protocol bounds for the given type parameter.
///
/// Returns a list of protocol types that the type parameter is constrained to.
pub fn get_type_parameter_bounds(
    type_param: &Arc<TypeParameterSymbol>,
) -> Vec<Ty> {
    let param_id = type_param.metadata().id();
    let mut bounds = Vec::new();

    // Walk up from the type parameter's parent to find where clauses
    // Note: The parent may be incorrectly set during symbol building,
    // so we also try to get bounds directly from symbols that own this type parameter
    let mut current: Option<Arc<dyn Symbol<KestrelLanguage>>> = type_param.metadata().parent();

    while let Some(parent) = current {
        if let Some(where_clause) = get_where_clause(parent.as_ref()) {
            bounds.extend(extract_bounds_for_param(&where_clause, param_id));
        }
        current = parent.metadata().parent();
    }

    bounds
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
    use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;

    let mut bounds = Vec::new();

    // Start from the current function
    if let Some(function) = ctx.db.symbol_by_id(ctx.function_id) {
        // Check function's where clause
        if let Some(where_clause) = get_where_clause(function.as_ref()) {
            bounds.extend(filter_resolved_bounds(&where_clause, param_id));
        }

        // Also check parent (struct/protocol/extension) where clause
        if let Some(parent) = function.metadata().parent() {
            // For extensions, get the combined where clause from ExtensionTargetBehavior
            // This includes both inherited struct constraints AND extension's own constraints
            if parent.metadata().kind() == KestrelSymbolKind::Extension {
                if let Some(target_beh) = parent.extension_target_behavior() {
                    let where_clause = target_beh.where_clause();
                    bounds.extend(filter_resolved_bounds(where_clause, param_id));
                }
            } else if let Some(where_clause) = get_where_clause(parent.as_ref()) {
                bounds.extend(filter_resolved_bounds(&where_clause, param_id));
            }
        }
    }

    // Validate that no bounds use generic protocols (with type arguments)
    // Filter them out and emit errors
    bounds = bounds.into_iter().filter(|bound| {
        if let TyKind::Protocol { symbol, substitutions } = bound.kind() {
            if !substitutions.is_empty() {
                // This is a generic protocol bound like Container[E]
                let protocol_name = symbol.metadata().name().value.clone();
                let error = UnsupportedGenericProtocolBoundError {
                    span: bound.span().clone(),
                    protocol_name,
                };
                ctx.diagnostics.add_diagnostic(error.into_diagnostic(ctx.file_id));
                return false; // Filter out this bound
            }
        }
        true
    }).collect();

    bounds
}

/// Recursively apply a transformation function to a type.
///
/// For composite types (Array, Tuple, Function), recursively applies the transformation
/// to nested types. For base types, returns the type unchanged.
///
/// The transformation function should return Some(new_type) to replace a type,
/// or None to use default traversal.
fn apply_type_transformation<F>(ty: &Ty, transform: &F) -> Ty
where
    F: Fn(&Ty) -> Option<Ty>,
{
    // Check if transform handles this type directly
    if let Some(transformed) = transform(ty) {
        return transformed;
    }

    // Otherwise, recursively traverse composite types
    match ty.kind() {
        TyKind::Array(element) => {
            Ty::array(apply_type_transformation(element, transform), ty.span().clone())
        }
        TyKind::Tuple(elements) => {
            let new_elements: Vec<Ty> = elements
                .iter()
                .map(|e| apply_type_transformation(e, transform))
                .collect();
            Ty::tuple(new_elements, ty.span().clone())
        }
        TyKind::Function { params, return_type } => {
            let new_params: Vec<Ty> = params
                .iter()
                .map(|p| apply_type_transformation(p, transform))
                .collect();
            let new_return = apply_type_transformation(return_type, transform);
            Ty::function(new_params, new_return, ty.span().clone())
        }
        // Base types - return as-is
        _ => ty.clone(),
    }
}

/// Substitute Self type with a replacement type recursively.
///
/// This is used when looking up methods on constrained type parameters.
/// Protocol methods use `Self` to refer to the conforming type, which
/// needs to be replaced with the actual receiver type (e.g., `T`).
pub fn substitute_self(ty: &Ty, replacement: &Ty) -> Ty {
    apply_type_transformation(ty, &|t| {
        if matches!(t.kind(), TyKind::SelfType) {
            Some(replacement.clone())
        } else {
            None
        }
    })
}

/// Format a type for error messages
pub fn format_type(ty: &Ty) -> String {
    match ty.kind() {
        TyKind::Unit => "()".to_string(),
        TyKind::Never => "!".to_string(),
        TyKind::Bool => "Bool".to_string(),
        TyKind::String => "String".to_string(),
        TyKind::Int(bits) => format!("{:?}", bits),
        TyKind::Float(bits) => format!("{:?}", bits),
        TyKind::Tuple(elements) => {
            let items: Vec<_> = elements.iter().map(format_type).collect();
            format!("({})", items.join(", "))
        }
        TyKind::Array(elem) => format!("[{}]", format_type(elem)),
        TyKind::Function { params, return_type } => {
            let params_str: Vec<_> = params.iter().map(format_type).collect();
            format!("({}) -> {}", params_str.join(", "), format_type(return_type))
        }
        TyKind::Struct { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::Protocol { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::TypeParameter(param) => param.metadata().name().value.clone(),
        TyKind::TypeAlias { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::AssociatedType { symbol, container } => {
            match container {
                Some(container_ty) => format!("{}.{}", format_type(container_ty), symbol.metadata().name().value),
                None => symbol.metadata().name().value.clone(),
            }
        }
        TyKind::SelfType => "Self".to_string(),
        TyKind::TypeVar(_) => "_".to_string(),
        TyKind::Error => "<error>".to_string(),
    }
}

/// Substitute type parameters in a type with their concrete types.
///
/// This recursively traverses a type and replaces any TypeParameter with
/// the corresponding concrete type from the substitutions map.
pub fn substitute_type(ty: &Ty, substitutions: &Substitutions) -> Ty {
    match ty.kind() {
        TyKind::TypeParameter(param) => {
            let param_id = param.metadata().id();
            substitutions.get(param_id).cloned().unwrap_or_else(|| ty.clone())
        }
        TyKind::Array(element) => {
            Ty::array(substitute_type(element, substitutions), ty.span().clone())
        }
        TyKind::Tuple(elements) => {
            let new_elements: Vec<Ty> = elements.iter()
                .map(|e| substitute_type(e, substitutions))
                .collect();
            Ty::tuple(new_elements, ty.span().clone())
        }
        TyKind::Function { params, return_type } => {
            let new_params: Vec<Ty> = params.iter()
                .map(|p| substitute_type(p, substitutions))
                .collect();
            let new_return = substitute_type(return_type, substitutions);
            Ty::function(new_params, new_return, ty.span().clone())
        }
        TyKind::Struct { symbol, substitutions: inner_subs } => {
            // Apply our substitutions to the inner substitutions
            let mut new_subs = Substitutions::new();
            for (id, inner_ty) in inner_subs.iter() {
                new_subs.insert(*id, substitute_type(inner_ty, substitutions));
            }
            Ty::generic_struct(symbol.clone(), new_subs, ty.span().clone())
        }
        // For simple types, just return a clone
        _ => ty.clone(),
    }
}

/// Format a symbol kind for error messages
pub fn format_symbol_kind(kind: KestrelSymbolKind) -> String {
    match kind {
        KestrelSymbolKind::AssociatedType => "associated type".to_string(),
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
        }

        // For array types, recurse into element type
        TyKind::Array(elem_ty) => {
            if let TyKind::Array(arg_elem) = arg_ty.kind() {
                infer_from_type(elem_ty, arg_elem, type_params, substitutions);
            }
        }

        // For tuple types, recurse into each element
        TyKind::Tuple(elems) => {
            if let TyKind::Tuple(arg_elems) = arg_ty.kind() {
                for (pe, ae) in elems.iter().zip(arg_elems.iter()) {
                    infer_from_type(pe, ae, type_params, substitutions);
                }
            }
        }

        // For function types, recurse into params and return type
        TyKind::Function { params, return_type } => {
            if let TyKind::Function { params: arg_params, return_type: arg_ret } = arg_ty.kind() {
                for (pp, ap) in params.iter().zip(arg_params.iter()) {
                    infer_from_type(pp, ap, type_params, substitutions);
                }
                infer_from_type(return_type, arg_ret, type_params, substitutions);
            }
        }

        // For struct types with substitutions, match the inner type arguments
        TyKind::Struct { symbol: param_struct, substitutions: param_subs } => {
            if let TyKind::Struct { symbol: arg_struct, substitutions: arg_subs } = arg_ty.kind() {
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
        }

        // Other types don't contribute to inference
        _ => {}
    }
}

// =============================================================================
// Call-Site Constraint Verification
// =============================================================================

use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use crate::diagnostics::ConstraintNotSatisfiedError;
use crate::database::Db;

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
/// * `db` - Database for symbol lookup
/// * `file_id` - File ID for diagnostics
/// * `diagnostics` - Diagnostic context for reporting errors
///
/// # Returns
/// `true` if all constraints are satisfied, `false` otherwise
pub fn verify_type_argument_constraints(
    type_params: &[Arc<TypeParameterSymbol>],
    type_args: &[Ty],
    where_clause: &WhereClause,
    call_span: Span,
    db: &dyn Db,
    file_id: usize,
    diagnostics: &mut kestrel_reporting::DiagnosticContext,
) -> bool {
    use semantic_tree::symbol::SymbolId;

    let mut all_satisfied = true;

    for (param, arg) in type_params.iter().zip(type_args.iter()) {
        let param_id = param.metadata().id();
        let bounds = where_clause.bounds_for(param_id);

        for bound in bounds {
            if !type_satisfies_bound(arg, bound, db) {
                // Report constraint not satisfied
                let param_name = param.metadata().name().value.clone();
                let type_name = format_type(arg);
                let constraint_name = format_type(bound);

                let error = ConstraintNotSatisfiedError {
                    call_span: call_span.clone(),
                    type_name,
                    constraint_name,
                    type_param_name: param_name,
                    constraint_span: Some(bound.span().clone()),
                };
                diagnostics.add_diagnostic(error.into_diagnostic(file_id));
                all_satisfied = false;
            }
        }
    }

    all_satisfied
}

/// Check if a type satisfies a protocol bound.
///
/// This checks if a concrete type conforms to a protocol, either directly
/// or transitively through other constraints.
pub fn type_satisfies_bound(ty: &Ty, bound: &Ty, db: &dyn Db) -> bool {
    // Get the protocol from the bound
    let TyKind::Protocol { symbol: required_proto, .. } = bound.kind() else {
        // Bound is not a protocol - shouldn't happen with proper validation
        return false;
    };

    match ty.kind() {
        // Concrete struct - check if it conforms to the protocol
        TyKind::Struct { symbol, .. } => {
            // Check direct conformances
            if let Some(conformances) = symbol.conformances_behavior() {
                for conf in conformances.conformances() {
                    if let TyKind::Protocol { symbol: conf_proto, .. } = conf.kind() {
                        if conf_proto.metadata().id() == required_proto.metadata().id() {
                            return true;
                        }
                        // TODO: Check inherited protocols
                    }
                }
            }

            // Also check extension conformances
            let struct_id = symbol.metadata().id();
            let extensions = db.get_extensions_for(struct_id);
            for extension in extensions {
                if let Some(conformances) = extension.conformances_behavior() {
                    for conf in conformances.conformances() {
                        if let TyKind::Protocol { symbol: conf_proto, .. } = conf.kind() {
                            if conf_proto.metadata().id() == required_proto.metadata().id() {
                                return true;
                            }
                            // TODO: Check inherited protocols
                        }
                    }
                }
            }

            false
        }

        // Type parameter - check if its bounds satisfy the required bound
        TyKind::TypeParameter(param) => {
            let param_bounds = get_type_parameter_bounds(param);
            for pb in &param_bounds {
                if let TyKind::Protocol { symbol: pb_proto, .. } = pb.kind() {
                    if pb_proto.metadata().id() == required_proto.metadata().id() {
                        return true;
                    }
                    // TODO: Check protocol inheritance
                }
            }
            false
        }

        // Primitive types - check for built-in protocol conformances
        // Currently primitives don't conform to user-defined protocols
        TyKind::Int(_) | TyKind::Float(_) | TyKind::Bool | TyKind::String => {
            // TODO: Add built-in protocol conformances (Equatable, etc.)
            false
        }

        // Other types don't satisfy protocol bounds
        _ => false,
    }
}
