//! Type resolution from syntax nodes
//!
//! This module provides `TypeResolver` which resolves types from syntax nodes
//! during the bind phase.

use std::sync::Arc;

use kestrel_prelude::primitives;
use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_model::{ResolveTypePath, SemanticModel, TypePathResolution};
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Substitutions, Ty, TyKind};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::diagnostics::{
    AmbiguousTypeError, NotATypeError, NotGenericError, TooFewTypeArgumentsError,
    TooManyTypeArgumentsError, UnresolvedTypeError,
};
use kestrel_syntax_tree::utils::{extract_path_segments, get_node_span};

/// Resolves types from syntax nodes during the bind phase
///
/// The resolver maintains context about where resolution is happening,
/// including the semantic model for lookups, diagnostics for errors, and
/// the current scope context.
///
/// # Example
///
/// ```ignore
/// let mut resolver = TypeResolver::new(model, diagnostics, file_id, source, context_id);
/// let ty = resolver.resolve(&ty_node);
/// ```
pub struct TypeResolver<'a> {
    model: &'a SemanticModel,
    diagnostics: &'a mut DiagnosticContext,
    source: &'a str,
    file_id: usize,
    context_id: SymbolId,
}

impl<'a> TypeResolver<'a> {
    /// Create a new type resolver
    pub fn new(
        model: &'a SemanticModel,
        diagnostics: &'a mut DiagnosticContext,
        source: &'a str,
        file_id: usize,
        context_id: SymbolId,
    ) -> Self {
        Self {
            model,
            diagnostics,
            source,
            file_id,
            context_id,
        }
    }

    /// Resolve a type from a Ty syntax node
    pub fn resolve(&mut self, ty_node: &SyntaxNode) -> Ty {
        let ty_span = get_node_span(ty_node, self.file_id);

        // Try TyPath (with type arguments support)
        if let Some(ty_path_node) = ty_node
            .children()
            .find(|child| child.kind() == SyntaxKind::TyPath)
        {
            return self.resolve_ty_path(&ty_path_node);
        }

        // Try TyUnit
        if ty_node
            .children()
            .any(|child| child.kind() == SyntaxKind::TyUnit)
        {
            return Ty::unit(ty_span);
        }

        // Try TyNever
        if ty_node
            .children()
            .any(|child| child.kind() == SyntaxKind::TyNever)
        {
            return Ty::never(ty_span);
        }

        // Try TyFunction
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
                    param_types.push(self.resolve(&param_ty_node));
                }
            }

            let return_ty = fn_ty_node
                .children()
                .filter(|c| c.kind() == SyntaxKind::Ty)
                .last()
                .map(|ty| self.resolve(&ty))
                .unwrap_or_else(|| Ty::unit(ty_span.clone()));

            return Ty::function(param_types, return_ty, ty_span);
        }

        // Try TyTuple
        if let Some(tuple_node) = ty_node
            .children()
            .find(|child| child.kind() == SyntaxKind::TyTuple)
        {
            let element_types: Vec<Ty> = tuple_node
                .children()
                .filter(|c| c.kind() == SyntaxKind::Ty)
                .map(|ty| self.resolve(&ty))
                .collect();

            return Ty::tuple(element_types, ty_span);
        }

        // Try TyArray
        if let Some(array_node) = ty_node
            .children()
            .find(|child| child.kind() == SyntaxKind::TyArray)
        {
            if let Some(element_ty_node) =
                array_node.children().find(|c| c.kind() == SyntaxKind::Ty)
            {
                let element_ty = self.resolve(&element_ty_node);
                return Ty::array(element_ty, ty_span);
            }
            return Ty::error(ty_span);
        }

        // Try TyInferred (_)
        if ty_node
            .children()
            .any(|child| child.kind() == SyntaxKind::TyInferred)
        {
            return Ty::infer(ty_span);
        }

        // Fallback: error type
        Ty::error(ty_span)
    }

    /// Resolve type from a node that contains a Ty child
    pub fn resolve_from_parent(&mut self, node: &SyntaxNode) -> Ty {
        if let Some(ty_node) = node.children().find(|c| c.kind() == SyntaxKind::Ty) {
            return self.resolve(&ty_node);
        }
        Ty::error(Span::from(0..0))
    }

    /// Apply type arguments to a generic type
    pub fn apply_type_arguments(&mut self, resolved_ty: &Ty, type_args: Vec<Ty>, span: Span) -> Ty {
        match resolved_ty.kind() {
            TyKind::Struct { symbol, .. } => {
                let type_params = symbol.type_parameters();
                let type_name = symbol.metadata().name().value.clone();
                self.apply_type_args_to_generic(
                    &type_params,
                    &type_name,
                    type_args,
                    span.clone(),
                    |subs| Ty::generic_struct(symbol.clone(), subs, span),
                )
            }

            TyKind::Protocol { symbol, .. } => {
                let type_params = symbol.type_parameters();
                let type_name = symbol.metadata().name().value.clone();
                self.apply_type_args_to_generic(
                    &type_params,
                    &type_name,
                    type_args,
                    span.clone(),
                    |subs| Ty::generic_protocol(symbol.clone(), subs, span),
                )
            }

            TyKind::TypeAlias { symbol, .. } => {
                let type_params = symbol.type_parameters();
                let type_name = symbol.metadata().name().value.clone();
                self.apply_type_args_to_generic(
                    &type_params,
                    &type_name,
                    type_args,
                    span.clone(),
                    |subs| Ty::generic_type_alias(symbol.clone(), subs, span),
                )
            }

            TyKind::Enum { symbol, .. } => {
                let type_params = symbol.type_parameters();
                let type_name = symbol.metadata().name().value.clone();
                self.apply_type_args_to_generic(
                    &type_params,
                    &type_name,
                    type_args,
                    span.clone(),
                    |subs| Ty::generic_enum(symbol.clone(), subs, span),
                )
            }

            // Non-generic types with type arguments is an error
            _ => {
                let type_name = match resolved_ty.kind() {
                    TyKind::Int(_) => primitives::INT.to_string(),
                    TyKind::Float(_) => primitives::FLOAT.to_string(),
                    TyKind::Bool => primitives::BOOL.to_string(),
                    TyKind::String => primitives::STRING.to_string(),
                    TyKind::Unit => "()".to_string(),
                    TyKind::Never => "Never".to_string(),
                    TyKind::TypeParameter(p) => p.metadata().name().value.clone(),
                    _ => "type".to_string(),
                };
                self.diagnostics.throw(NotGenericError {
                    span: span.clone(),
                    type_name,
                });
                Ty::error(span)
            }
        }
    }

    /// Resolve a TyPath node, handling type arguments if present
    fn resolve_ty_path(&mut self, ty_path_node: &SyntaxNode) -> Ty {
            let ty_span = get_node_span(ty_path_node, self.file_id);

        if let Some(path_node) = ty_path_node
            .children()
            .find(|child| child.kind() == SyntaxKind::Path)
        {
            let segments = extract_path_segments(&path_node);

            if !segments.is_empty() {
                let type_args = self.extract_type_arguments(ty_path_node);
                let resolved = self.resolve_path(&segments, ty_span.clone());

                if !type_args.is_empty() && !resolved.is_error() {
                    return self.apply_type_arguments(&resolved, type_args, ty_span);
                }

                return resolved;
            }
        }

        Ty::error(ty_span)
    }

    /// Resolve a type path and emit diagnostics on failure
    fn resolve_path(&mut self, segments: &[String], ty_span: Span) -> Ty {
        match self.model.query(ResolveTypePath {
            path: segments.to_vec(),
            context: self.context_id,
        }) {
            TypePathResolution::Resolved(resolved_ty) => resolved_ty,
            TypePathResolution::NotFound { segment, .. } => {
                self.diagnostics.throw(UnresolvedTypeError {
                    span: ty_span.clone(),
                    type_name: segment,
                });
                Ty::error(ty_span)
            }
            TypePathResolution::Ambiguous {
                segment,
                candidates,
                ..
            } => {
                self.diagnostics.throw(AmbiguousTypeError {
                    span: ty_span.clone(),
                    type_name: segment,
                    candidate_count: candidates.len(),
                });
                Ty::error(ty_span)
            }
            TypePathResolution::NotAType { .. } => {
                self.diagnostics.throw(NotATypeError {
                    span: ty_span.clone(),
                    name: segments.join("."),
                });
                Ty::error(ty_span)
            }
        }
    }

    /// Extract type arguments from a TyPath node
    fn extract_type_arguments(&mut self, ty_path_node: &SyntaxNode) -> Vec<Ty> {
        if let Some(arg_list) = ty_path_node
            .children()
            .find(|c| c.kind() == SyntaxKind::TypeArgumentList)
        {
            arg_list
                .children()
                .filter(|c| c.kind() == SyntaxKind::Ty)
                .map(|ty| self.resolve(&ty))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Apply type arguments to a generic type (helper)
    fn apply_type_args_to_generic<F>(
        &mut self,
        type_params: &[Arc<TypeParameterSymbol>],
        type_name: &str,
        type_args: Vec<Ty>,
        span: Span,
        make_ty: F,
    ) -> Ty
    where
        F: FnOnce(Substitutions) -> Ty,
    {
        let max_args = type_params.len();
        let min_args = type_params.iter().filter(|p| !p.has_default()).count();
        let actual = type_args.len();

        // Non-generic type with type args
        if max_args == 0 {
            self.diagnostics.throw(NotGenericError {
                span: span.clone(),
                type_name: type_name.to_string(),
            });
            return Ty::error(span);
        }

        // Check arity with defaults
        if actual < min_args {
            self.diagnostics.throw(TooFewTypeArgumentsError {
                span: span.clone(),
                type_name: type_name.to_string(),
                min_expected: min_args,
                got: actual,
            });
            return Ty::error(span);
        }
        if actual > max_args {
            self.diagnostics.throw(TooManyTypeArgumentsError {
                span: span.clone(),
                type_name: type_name.to_string(),
                max_expected: max_args,
                got: actual,
            });
            return Ty::error(span);
        }

        // Build substitutions, using defaults for missing args
        let mut substitutions = Substitutions::new();
        for (i, param) in type_params.iter().enumerate() {
            let ty = if i < type_args.len() {
                type_args[i].clone()
            } else if let Some(default) = param.default() {
                default.clone()
            } else {
                Ty::error(span.clone())
            };
            substitutions.insert(param.metadata().id(), ty);
        }

        make_ty(substitutions)
    }
}

/// Extract a type from a Ty syntax node without resolution (placeholder types)
///
/// This is used during the build phase when we don't have access to the database.
/// Type paths are returned as error types - they will be resolved during bind phase.
pub fn extract_type_from_ty_node(ty_node: &SyntaxNode, source: &str) -> Ty {
    let _ = source;
    let ty_span = get_node_span(ty_node, 0);

    // Try TyPath
    if let Some(ty_path_node) = ty_node
        .children()
        .find(|child| child.kind() == SyntaxKind::TyPath)
    {
        if let Some(path_node) = ty_path_node
            .children()
            .find(|child| child.kind() == SyntaxKind::Path)
        {
            let segments: Vec<String> = extract_path_segments(&path_node);
            if !segments.is_empty() {
                return Ty::error(ty_span);
            }
        }
    }

    // Try TyUnit
    if ty_node
        .children()
        .any(|child| child.kind() == SyntaxKind::TyUnit)
    {
        return Ty::unit(ty_span);
    }

    // Try TyNever
    if ty_node
        .children()
        .any(|child| child.kind() == SyntaxKind::TyNever)
    {
        return Ty::never(ty_span);
    }

    // Try TyFunction
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
                param_types.push(extract_type_from_ty_node(&param_ty_node, source));
            }
        }

        let return_ty = fn_ty_node
            .children()
            .filter(|c| c.kind() == SyntaxKind::Ty)
            .last()
            .map(|ty| extract_type_from_ty_node(&ty, source))
            .unwrap_or_else(|| Ty::unit(ty_span.clone()));

        return Ty::function(param_types, return_ty, ty_span);
    }

    // Try TyTuple
    if let Some(tuple_node) = ty_node
        .children()
        .find(|child| child.kind() == SyntaxKind::TyTuple)
    {
        let element_types: Vec<Ty> = tuple_node
            .children()
            .filter(|c| c.kind() == SyntaxKind::Ty)
            .map(|ty| extract_type_from_ty_node(&ty, source))
            .collect();

        return Ty::tuple(element_types, ty_span);
    }

    // Try TyInferred (_)
    if ty_node
        .children()
        .any(|child| child.kind() == SyntaxKind::TyInferred)
    {
        return Ty::infer(ty_span);
    }

    // Fallback: error type
    Ty::error(ty_span)
}

/// Extract type from a node that contains a Ty child (without resolution)
pub fn extract_type_from_node(node: &SyntaxNode, source: &str) -> Ty {
    let _ = (node, source);
    Ty::error(Span::from(0..0))
}

// =============================================================================
// Legacy API for backwards compatibility
// =============================================================================

/// Context for type resolution from syntax during the bind phase (legacy alias)
///
/// This is a type alias for backwards compatibility. New code should use `TypeResolver` directly.
pub type TypeSyntaxContext<'a> = TypeResolver<'a>;

/// Resolve a type from a Ty syntax node during bind phase (legacy function)
///
/// This is provided for backwards compatibility. New code should use `TypeResolver::resolve()` directly.
pub fn resolve_type_from_ty_node(ty_node: &SyntaxNode, ctx: &mut TypeSyntaxContext) -> Ty {
    ctx.resolve(ty_node)
}
