//! Pure type resolution from syntax nodes
//!
//! This module provides diagnostic-free type resolution for use in queries.
//! It mirrors the logic in `TypeResolver` (binder crate) but returns `Ty::error`
//! on failures instead of emitting diagnostics. The binder's `TypeResolver` remains
//! the authoritative source for diagnostic reporting.

use std::sync::Arc;

use kestrel_prelude::lang;
use kestrel_semantic_tree::builtins::LanguageFeature;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{FloatBits, IntBits, Substitutions, Ty, TyKind};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use kestrel_syntax_tree::utils::{extract_path_segments, get_node_span};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::ResolveTypePath;
use crate::resolution::TypePathResolution;

/// Resolve a type from a Ty syntax node without emitting diagnostics.
///
/// This is the query-compatible counterpart of `TypeResolver::resolve()`.
/// On any resolution failure, returns `Ty::error(span)` silently.
pub fn resolve_type_from_syntax_node(
    model: &SemanticModel,
    ty_node: &SyntaxNode,
    context_id: SymbolId,
    file_id: usize,
) -> Ty {
    let ty_span = get_node_span(ty_node, file_id);

    // TyPath (with type arguments support)
    if let Some(ty_path_node) = ty_node
        .children()
        .find(|child| child.kind() == SyntaxKind::TyPath)
    {
        return resolve_ty_path(model, &ty_path_node, context_id, file_id);
    }

    // TyUnit
    if ty_node
        .children()
        .any(|child| child.kind() == SyntaxKind::TyUnit)
    {
        return Ty::unit(ty_span);
    }

    // TyNever
    if ty_node
        .children()
        .any(|child| child.kind() == SyntaxKind::TyNever)
    {
        return Ty::never(ty_span);
    }

    // TyFunction
    if let Some(fn_ty_node) = ty_node
        .children()
        .find(|child| child.kind() == SyntaxKind::TyFunction)
    {
        let mut param_types = Vec::new();
        if let Some(ty_list) = fn_ty_node
            .children()
            .find(|child| child.kind() == SyntaxKind::TyList)
        {
            for param_ty_node in ty_list.children().filter(|c| c.kind() == SyntaxKind::Ty) {
                param_types.push(resolve_type_from_syntax_node(
                    model,
                    &param_ty_node,
                    context_id,
                    file_id,
                ));
            }
        }

        let return_ty = fn_ty_node
            .children()
            .filter(|c| c.kind() == SyntaxKind::Ty)
            .last()
            .map(|ty| resolve_type_from_syntax_node(model, &ty, context_id, file_id))
            .unwrap_or_else(|| Ty::unit(ty_span.clone()));

        return Ty::function(param_types, return_ty, ty_span);
    }

    // TyTuple
    if let Some(tuple_node) = ty_node
        .children()
        .find(|child| child.kind() == SyntaxKind::TyTuple)
    {
        let element_types: Vec<Ty> = tuple_node
            .children()
            .filter(|c| c.kind() == SyntaxKind::Ty)
            .map(|ty| resolve_type_from_syntax_node(model, &ty, context_id, file_id))
            .collect();

        return Ty::tuple(element_types, ty_span);
    }

    // TyArray — [T] desugars to ArrayTypeOperator[T]
    if let Some(array_node) = ty_node
        .children()
        .find(|child| child.kind() == SyntaxKind::TyArray)
    {
        if let Some(element_ty_node) =
            array_node.children().find(|c| c.kind() == SyntaxKind::Ty)
        {
            let element_ty =
                resolve_type_from_syntax_node(model, &element_ty_node, context_id, file_id);
            return resolve_type_operator(
                model,
                LanguageFeature::ArrayTypeOperator,
                vec![element_ty],
                ty_span,
                context_id,
                file_id,
            );
        }
        return Ty::error(ty_span);
    }

    // TyDictionary — [K: V] desugars to DictionaryTypeOperator[K, V]
    if let Some(dict_node) = ty_node
        .children()
        .find(|child| child.kind() == SyntaxKind::TyDictionary)
    {
        let ty_nodes: Vec<_> = dict_node
            .children()
            .filter(|c| c.kind() == SyntaxKind::Ty)
            .collect();
        if ty_nodes.len() >= 2 {
            let key_ty =
                resolve_type_from_syntax_node(model, &ty_nodes[0], context_id, file_id);
            let value_ty =
                resolve_type_from_syntax_node(model, &ty_nodes[1], context_id, file_id);
            return resolve_type_operator(
                model,
                LanguageFeature::DictionaryTypeOperator,
                vec![key_ty, value_ty],
                ty_span,
                context_id,
                file_id,
            );
        }
        return Ty::error(ty_span);
    }

    // TyOptional — T? desugars to OptionalTypeOperator[T]
    if let Some(optional_node) = ty_node
        .children()
        .find(|child| child.kind() == SyntaxKind::TyOptional)
    {
        if let Some(base_ty_node) = optional_node
            .children()
            .find(|c| c.kind() == SyntaxKind::Ty)
        {
            let base_ty =
                resolve_type_from_syntax_node(model, &base_ty_node, context_id, file_id);
            return resolve_type_operator(
                model,
                LanguageFeature::OptionalTypeOperator,
                vec![base_ty],
                ty_span,
                context_id,
                file_id,
            );
        }
        return Ty::error(ty_span);
    }

    // TyResult — T throws E desugars to ResultTypeOperator[T, E]
    if let Some(result_node) = ty_node
        .children()
        .find(|child| child.kind() == SyntaxKind::TyResult)
    {
        let ty_nodes: Vec<_> = result_node
            .children()
            .filter(|c| c.kind() == SyntaxKind::Ty)
            .collect();
        if ty_nodes.len() >= 2 {
            let success_ty =
                resolve_type_from_syntax_node(model, &ty_nodes[0], context_id, file_id);
            let error_ty =
                resolve_type_from_syntax_node(model, &ty_nodes[1], context_id, file_id);
            return resolve_type_operator(
                model,
                LanguageFeature::ResultTypeOperator,
                vec![success_ty, error_ty],
                ty_span,
                context_id,
                file_id,
            );
        }
        return Ty::error(ty_span);
    }

    // TyInferred (_)
    if ty_node
        .children()
        .any(|child| child.kind() == SyntaxKind::TyInferred)
    {
        return Ty::infer(ty_span);
    }

    // Fallback: error type
    Ty::error(ty_span)
}

/// Resolve a TyPath node, handling type arguments if present.
fn resolve_ty_path(
    model: &SemanticModel,
    ty_path_node: &SyntaxNode,
    context_id: SymbolId,
    file_id: usize,
) -> Ty {
    let ty_span = get_node_span(ty_path_node, file_id);

    let Some(path_node) = ty_path_node
        .children()
        .find(|child| child.kind() == SyntaxKind::Path)
    else {
        return Ty::error(ty_span);
    };

    let segments = extract_path_segments(&path_node);
    if segments.is_empty() {
        return Ty::error(ty_span);
    }

    // Check for lang.* built-in primitive types
    if segments.len() == 2 && segments[0] == lang::LANG {
        if segments[1] == lang::PTR {
            let type_args =
                extract_type_arguments(model, ty_path_node, context_id, file_id);
            match type_args {
                Some(args) if args.len() == 1 => {
                    return Ty::pointer(args.into_iter().next().unwrap(), ty_span);
                },
                _ => return Ty::error(ty_span),
            }
        }

        // lang.* scalar types — reject type arguments silently
        if let Some(ty) = resolve_lang_scalar(&segments[1], ty_span.clone()) {
            return ty;
        }
    }

    let type_args_opt = extract_type_arguments(model, ty_path_node, context_id, file_id);

    let resolved = match model.query(ResolveTypePath {
        path: segments.to_vec(),
        context: context_id,
    }) {
        TypePathResolution::Resolved(ty) => ty,
        _ => return Ty::error(ty_span),
    };

    if resolved.is_error() {
        return resolved;
    }

    let is_potentially_generic = matches!(
        resolved.kind(),
        TyKind::Struct { .. }
            | TyKind::Protocol { .. }
            | TyKind::TypeAlias { .. }
            | TyKind::Enum { .. }
    );

    match type_args_opt {
        // Explicit type arguments provided
        Some(type_args) if is_potentially_generic => {
            apply_type_arguments(model, &resolved, type_args, ty_span, context_id, file_id)
        },
        // Type arguments on a non-generic type
        Some(type_args) if !type_args.is_empty() => Ty::error(ty_span),
        // No brackets — apply inferred type arguments for generic types
        None if is_potentially_generic => {
            apply_inferred_type_arguments(&resolved, ty_span, model, context_id, file_id)
        },
        _ => resolved,
    }
}

/// Resolve a lang.* scalar type name to a Ty.
fn resolve_lang_scalar(name: &str, span: Span) -> Option<Ty> {
    match name {
        lang::I1 => Some(Ty::bool(span)),
        lang::I8 => Some(Ty::int(IntBits::I8, span)),
        lang::I16 => Some(Ty::int(IntBits::I16, span)),
        lang::I32 => Some(Ty::int(IntBits::I32, span)),
        lang::I64 => Some(Ty::int(IntBits::I64, span)),
        lang::F16 => Some(Ty::float(FloatBits::F16, span)),
        lang::F32 => Some(Ty::float(FloatBits::F32, span)),
        lang::F64 => Some(Ty::float(FloatBits::F64, span)),
        lang::STR => Some(Ty::string(span)),
        _ => None,
    }
}

/// Resolve a type operator by looking up the builtin type alias and applying type arguments.
fn resolve_type_operator(
    model: &SemanticModel,
    feature: LanguageFeature,
    type_args: Vec<Ty>,
    span: Span,
    context_id: SymbolId,
    file_id: usize,
) -> Ty {
    let builtin_registry = model.builtin_registry();
    let Some(symbol_id) = builtin_registry.type_alias(feature) else {
        return Ty::error(span);
    };

    let Some(symbol) = model.registry().get(symbol_id) else {
        return Ty::error(span);
    };

    let Ok(type_alias_arc) = symbol.into_any_arc().downcast::<TypeAliasSymbol>() else {
        return Ty::error(span);
    };

    let base_ty = Ty::type_alias(type_alias_arc, span.clone());
    apply_type_arguments(model, &base_ty, type_args, span, context_id, file_id)
}

/// Extract type arguments from a TyPath node.
///
/// Returns `None` if there are no type argument brackets.
/// Returns `Some(vec)` if brackets are present (may be empty).
fn extract_type_arguments(
    model: &SemanticModel,
    ty_path_node: &SyntaxNode,
    context_id: SymbolId,
    file_id: usize,
) -> Option<Vec<Ty>> {
    ty_path_node
        .children()
        .find(|c| c.kind() == SyntaxKind::TypeArgumentList)
        .map(|arg_list| {
            arg_list
                .children()
                .filter(|c| c.kind() == SyntaxKind::Ty)
                .map(|ty| resolve_type_from_syntax_node(model, &ty, context_id, file_id))
                .collect()
        })
}

/// Apply type arguments to a generic type.
fn apply_type_arguments(
    model: &SemanticModel,
    resolved_ty: &Ty,
    type_args: Vec<Ty>,
    span: Span,
    context_id: SymbolId,
    file_id: usize,
) -> Ty {
    match resolved_ty.kind() {
        TyKind::Struct { symbol, .. } => {
            let type_params = symbol.type_parameters();
            let defining_context = symbol.metadata().id();
            apply_type_args_to_generic(
                model,
                &type_params,
                type_args,
                span.clone(),
                defining_context,
                context_id,
                file_id,
                |subs| Ty::generic_struct(symbol.clone(), subs, span),
            )
        },
        TyKind::Protocol { symbol, .. } => {
            let type_params = symbol.type_parameters();
            let defining_context = symbol.metadata().id();
            apply_type_args_to_generic(
                model,
                &type_params,
                type_args,
                span.clone(),
                defining_context,
                context_id,
                file_id,
                |subs| Ty::generic_protocol(symbol.clone(), subs, span),
            )
        },
        TyKind::TypeAlias { symbol, .. } => {
            let type_params = symbol.type_parameters();
            let defining_context = symbol.metadata().id();
            apply_type_args_to_generic(
                model,
                &type_params,
                type_args,
                span.clone(),
                defining_context,
                context_id,
                file_id,
                |subs| Ty::generic_type_alias(symbol.clone(), subs, span),
            )
        },
        TyKind::Enum { symbol, .. } => {
            let type_params = symbol.type_parameters();
            let defining_context = symbol.metadata().id();
            apply_type_args_to_generic(
                model,
                &type_params,
                type_args,
                span.clone(),
                defining_context,
                context_id,
                file_id,
                |subs| Ty::generic_enum(symbol.clone(), subs, span),
            )
        },
        // Non-generic types with type arguments
        _ => {
            if type_args.is_empty() {
                resolved_ty.clone()
            } else {
                Ty::error(span)
            }
        },
    }
}

/// Apply type arguments if all type parameters have defaults (raw reference without brackets).
fn apply_inferred_type_arguments(
    resolved_ty: &Ty,
    span: Span,
    model: &SemanticModel,
    context_id: SymbolId,
    file_id: usize,
) -> Ty {
    let type_params = match resolved_ty.kind() {
        TyKind::Struct { symbol, .. } => symbol.type_parameters(),
        TyKind::Protocol { symbol, .. } => symbol.type_parameters(),
        TyKind::TypeAlias { symbol, .. } => symbol.type_parameters(),
        TyKind::Enum { symbol, .. } => symbol.type_parameters(),
        _ => return resolved_ty.clone(),
    };

    if type_params.is_empty() {
        return resolved_ty.clone();
    }

    let type_args = (0..type_params.len())
        .map(|_| Ty::infer(span.clone()))
        .collect();

    apply_type_arguments(model, resolved_ty, type_args, span, context_id, file_id)
}

/// Apply type arguments to a generic type — pure version without diagnostics.
///
/// On arity mismatch, returns `Ty::error` instead of emitting diagnostics.
#[allow(clippy::too_many_arguments)]
fn apply_type_args_to_generic<F>(
    model: &SemanticModel,
    type_params: &[Arc<TypeParameterSymbol>],
    type_args: Vec<Ty>,
    span: Span,
    defining_context: SymbolId,
    _context_id: SymbolId,
    _file_id: usize,
    make_ty: F,
) -> Ty
where
    F: FnOnce(Substitutions) -> Ty,
{
    let max_args = type_params.len();
    let actual = type_args.len();

    if max_args == 0 {
        if !type_args.is_empty() {
            return Ty::error(span);
        }
        return make_ty(Substitutions::new());
    }

    // Check arity
    let min_args = type_params.iter().take_while(|p| !p.has_default()).count();

    if actual < min_args || actual > max_args {
        return Ty::error(span);
    }

    // Build substitutions, filling in defaults for missing trailing arguments.
    let mut substitutions = Substitutions::new();
    for (i, param) in type_params.iter().enumerate() {
        let arg = if i < type_args.len() {
            type_args[i].clone()
        } else {
            // Use the default
            let default_ty = param.default().expect("missing default for type parameter");
            // Resolve UnresolvedPath defaults in the defining type's scope
            if let TyKind::UnresolvedPath { segments } = default_ty.kind() {
                match model.query(ResolveTypePath {
                    path: segments.to_vec(),
                    context: defining_context,
                }) {
                    TypePathResolution::Resolved(resolved_ty) => resolved_ty,
                    _ => Ty::error(default_ty.span().clone()),
                }
            } else {
                default_ty.clone()
            }
        };
        substitutions.insert(param.metadata().id(), arg);
    }

    make_ty(substitutions)
}
