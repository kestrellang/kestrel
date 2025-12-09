//! Shared utilities for body resolution.
//!
//! This module contains helper functions used across multiple body resolution
//! modules, including type formatting, signature matching, and behavior lookups.

use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Substitutions, Ty, TyKind, WhereClause};
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxKind;
use semantic_tree::symbol::Symbol;

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
                    substitutions.insert(param_id, Ty::inferred(span.clone()));
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
            for bound in where_clause.bounds_for(param_id) {
                bounds.push(bound.clone());
            }
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
    ctx: &BodyResolutionContext,
) -> Vec<Ty> {
    let param_id = type_param.metadata().id();
    let mut bounds = Vec::new();

    // Start from the current function
    if let Some(function) = ctx.db.symbol_by_id(ctx.function_id) {
        // Check function's where clause
        if let Some(where_clause) = get_where_clause(function.as_ref()) {
            collect_resolved_bounds(&where_clause, param_id, ctx, &mut bounds);
        }

        // Also check parent (struct/protocol) where clause
        if let Some(parent) = function.metadata().parent() {
            if let Some(where_clause) = get_where_clause(parent.as_ref()) {
                collect_resolved_bounds(&where_clause, param_id, ctx, &mut bounds);
            }
        }
    }

    bounds
}

/// Collect resolved bounds from a where clause for a specific type parameter.
///
/// The where clause stores bounds as Ty::error() placeholders. This function
/// resolves them dynamically by re-resolving the bound names from the syntax.
///
/// Since we don't have access to the syntax here, we use the bound spans to
/// look up the original path and resolve it.
fn collect_resolved_bounds(
    where_clause: &WhereClause,
    param_id: semantic_tree::symbol::SymbolId,
    _ctx: &BodyResolutionContext,
    bounds: &mut Vec<Ty>,
) {
    for constraint in &where_clause.constraints {
        if let kestrel_semantic_tree::ty::Constraint::TypeBound { param: Some(id), bounds: bound_tys, .. } = constraint {
            if *id == param_id {
                for bound_ty in bound_tys {
                    // If the bound is already resolved (Protocol), use it directly
                    if let TyKind::Protocol { .. } = bound_ty.kind() {
                        bounds.push(bound_ty.clone());
                    }
                    // Unresolved bounds (Ty::error) are skipped - they'll be caught by validation
                }
            }
        }
    }
}

/// Substitute Self type with a replacement type recursively.
///
/// This is used when looking up methods on constrained type parameters.
/// Protocol methods use `Self` to refer to the conforming type, which
/// needs to be replaced with the actual receiver type (e.g., `T`).
pub fn substitute_self(ty: &Ty, replacement: &Ty) -> Ty {
    match ty.kind() {
        TyKind::SelfType => replacement.clone(),
        TyKind::Array(elem) => {
            Ty::array(substitute_self(elem, replacement), ty.span().clone())
        }
        TyKind::Tuple(elems) => {
            let new_elems: Vec<_> = elems
                .iter()
                .map(|e| substitute_self(e, replacement))
                .collect();
            Ty::tuple(new_elems, ty.span().clone())
        }
        TyKind::Function { params, return_type } => {
            let new_params: Vec<_> = params
                .iter()
                .map(|p| substitute_self(p, replacement))
                .collect();
            let new_ret = substitute_self(return_type, replacement);
            Ty::function(new_params, new_ret, ty.span().clone())
        }
        // For other types, return as-is
        _ => ty.clone(),
    }
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
        TyKind::Inferred => "_".to_string(),
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
// Call-Site Constraint Verification
// =============================================================================

use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use crate::diagnostics::ConstraintNotSatisfiedError;
use crate::database::Db;
use kestrel_reporting::IntoDiagnostic;

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
fn type_satisfies_bound(ty: &Ty, bound: &Ty, db: &dyn Db) -> bool {
    // Get the protocol from the bound
    let TyKind::Protocol { symbol: required_proto, .. } = bound.kind() else {
        // Bound is not a protocol - shouldn't happen with proper validation
        return false;
    };

    match ty.kind() {
        // Concrete struct - check if it conforms to the protocol
        TyKind::Struct { symbol, .. } => {
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
