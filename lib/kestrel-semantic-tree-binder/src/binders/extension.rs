//! Extension declaration resolver
//!
//! Extensions add methods and protocol conformances to existing types.
//! Unlike structs, extensions don't have a name - they're identified by their target type.

use std::sync::Arc;

use kestrel_semantic_model::{ResolveTypePath, TypePathResolution};
use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Constraint, Ty, TyKind, WhereClause};
use kestrel_span::Spanned;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::diagnostics::{NotAProtocolContext, UnresolvedTypeError};
use crate::resolution::TypeResolver;
use crate::syntax::helpers::resolve_conformance_list;
use kestrel_syntax_tree::utils::{extract_path_segments, find_child, get_node_span};

/// Binder for extension declarations
pub struct ExtensionBinder;

impl DeclarationBinder for ExtensionBinder {
    fn bind_signature(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        let symbol_id = symbol.metadata().id();
        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // Extract doc comment
        if let Some(doc) = crate::binders::utils::doc_comment::extract_doc_comment(syntax) {
            symbol.metadata().add_behavior(doc);
        }

        // Resolve the target type from the Ty node
        let target_result = resolve_extension_target(syntax, &source, file_id, symbol_id, context);

        if let Some((target_ty, target_symbol, type_arguments, referenced_params)) = target_result {
            // Get the target's where clause constraints (inherited)
            let inherited_where_clause = target_symbol.where_clause();

            // Resolve extension's own where clause
            let extension_where_clause = crate::binders::utils::generics::resolve_where_clause(
                syntax,
                &source,
                file_id,
                symbol_id,
                context,
                &referenced_params,
            );

            // Combine inherited and extension constraints
            let mut combined_where_clause =
                combine_where_clauses(inherited_where_clause, extension_where_clause);

            // For protocol extensions, add implicit Self: Protocol bound
            if target_symbol.is_protocol() {
                // Find the synthetic Self type parameter we created
                if let Some(self_param) = referenced_params
                    .iter()
                    .find(|p| p.metadata().name().value == "Self")
                {
                    let self_constraint = Constraint::type_bound(
                        self_param.metadata().id(),
                        "Self".to_string(),
                        self_param.metadata().span().clone(),
                        vec![target_ty.clone()],
                    );
                    combined_where_clause.add_constraint(self_constraint);
                }
            }

            // Create and add ExtensionTargetBehavior
            let target_behavior = ExtensionTargetBehavior::new(
                target_ty.clone(),
                type_arguments,
                referenced_params,
                combined_where_clause,
            );
            symbol.metadata().add_behavior(target_behavior);

            // Extension registration in ExtensionRegistry is deferred to
            // register_extensions() pass (after all bind_signatures complete).
        }

        // Resolve conformances from syntax
        resolve_conformance_list(
            syntax,
            &source,
            file_id,
            symbol,
            symbol_id,
            context,
            NotAProtocolContext::Conformance,
        );
    }
}

/// Symbol that can be extended (struct, enum, or protocol)
enum ExtendableSymbol {
    Struct(Arc<StructSymbol>),
    Enum(Arc<EnumSymbol>),
    Protocol(Arc<ProtocolSymbol>),
}

impl ExtendableSymbol {
    fn type_parameters(&self) -> Vec<Arc<TypeParameterSymbol>> {
        match self {
            ExtendableSymbol::Struct(s) => s.type_parameters(),
            ExtendableSymbol::Enum(e) => e.type_parameters(),
            ExtendableSymbol::Protocol(p) => p.type_parameters(),
        }
    }

    fn id(&self) -> semantic_tree::symbol::SymbolId {
        match self {
            ExtendableSymbol::Struct(s) => s.metadata().id(),
            ExtendableSymbol::Enum(e) => e.metadata().id(),
            ExtendableSymbol::Protocol(p) => p.metadata().id(),
        }
    }

    fn where_clause(&self) -> WhereClause {
        match self {
            ExtendableSymbol::Struct(s) => s.where_clause(),
            ExtendableSymbol::Enum(e) => e.where_clause(),
            // Protocols don't inherit their where clause to extensions
            // Protocol extensions have their own independent where clauses
            ExtendableSymbol::Protocol(_) => WhereClause::new(),
        }
    }

    fn is_protocol(&self) -> bool {
        matches!(self, ExtendableSymbol::Protocol(_))
    }
}

/// Resolve the extension's target type from syntax.
///
/// Returns (target_type, target_symbol, type_arguments, referenced_type_params)
///
/// For generic extensions like `extend Pair[T, U]`, the type arguments T and U are
/// resolved as references to the struct's type parameters, not looked up in scope.
fn resolve_extension_target(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
) -> Option<(Ty, ExtendableSymbol, Vec<Ty>, Vec<Arc<TypeParameterSymbol>>)> {
    // Find the Ty node (target type expression)
    let ty_node = find_child(syntax, SyntaxKind::Ty)?;
    let ty_span = get_node_span(&ty_node, file_id);

    // Find TyPath within Ty
    let ty_path_node = ty_node
        .children()
        .find(|c| c.kind() == SyntaxKind::TyPath)?;

    // Find Path within TyPath to get the base type name
    let path_node = ty_path_node
        .children()
        .find(|c| c.kind() == SyntaxKind::Path)?;

    let segments = extract_path_segments(&path_node);
    if segments.is_empty() {
        return None;
    }

    // First, resolve just the base type (without type arguments) to get the struct
    let base_resolution = ctx.model.query(ResolveTypePath {
        path: segments.clone(),
        context: context_id,
    });
    let target_symbol = match base_resolution {
        TypePathResolution::Resolved(ty) => match ty.kind() {
            TyKind::Struct { symbol, .. } => ExtendableSymbol::Struct(symbol.clone()),
            TyKind::Enum { symbol, .. } => ExtendableSymbol::Enum(symbol.clone()),
            TyKind::Protocol { symbol, .. } => ExtendableSymbol::Protocol(symbol.clone()),
            _ => {
                ctx.diagnostics.throw(CannotExtendTypeError {
                    span: ty_span.clone(),
                    type_name: format_type_kind(ty.kind()),
                });
                return None;
            },
        },
        TypePathResolution::NotFound { segment, .. } => {
            ctx.diagnostics.throw(UnresolvedTypeError {
                span: ty_span.clone(),
                type_name: segment,
            });
            return None;
        },
        _ => return None,
    };

    // Get the target's type parameters
    let target_type_params = target_symbol.type_parameters();

    // Now resolve the type arguments, using the target's type params as available references
    let type_args = resolve_extension_type_arguments(
        &ty_path_node,
        source,
        file_id,
        &target_type_params,
        context_id,
        ctx,
    )?;

    // Collect referenced type parameters from the type arguments.
    // If no explicit type arguments are given (e.g., `extend Pointer` instead of `extend Pointer[T]`),
    // implicitly reference all of the target's type parameters so that where clauses can use them.
    // This enables conditional conformances like `extend Pointer: FFISafe where T: FFISafe`.
    let mut referenced_params = if type_args.is_empty() && !target_type_params.is_empty() {
        // No explicit type args - reference all of the target's type parameters
        target_type_params.clone()
    } else {
        collect_referenced_type_params(&type_args)
    };

    // For protocol extensions, create a synthetic "Self" type parameter
    // This allows Self and Self.Item to be resolved as type parameters with protocol bounds
    if matches!(&target_symbol, ExtendableSymbol::Protocol(_)) {
        let self_param = Arc::new(TypeParameterSymbol::new(
            Spanned::new("Self".to_string(), ty_span.clone()),
            ty_span.clone(),
            None,
        ));
        // Register the synthetic Self with the symbol registry so SymbolFor can find it
        ctx.model.registry().register(self_param.clone());
        ctx.model.invalidate_cache();

        referenced_params.push(self_param);
    }

    // Build the final type with substitutions
    // For generic extensions like `extend Pair[T, U]`, validate that type parameters
    // appear in their declared positions (T must be in position 0, U in position 1)
    let mut substitutions = kestrel_semantic_tree::ty::Substitutions::new();

    // If no explicit type arguments are given (e.g., `extend Pointer` instead of `extend Pointer[T]`),
    // create self-referential substitutions (T -> T) for each type parameter.
    // This ensures the extension's target type is `Pointer[T]` (fully generic), not `Pointer[]` (no type args).
    if type_args.is_empty() && !target_type_params.is_empty() {
        for param in &target_type_params {
            let param_id = param.metadata().id();
            let param_ty = Ty::type_parameter(param.clone(), ty_span.clone());
            substitutions.insert(param_id, param_ty);
        }
    } else {
        for (index, (param, arg)) in target_type_params.iter().zip(type_args.iter()).enumerate() {
            let param_id = param.metadata().id();

            // Check if arg is a type parameter reference
            if let kestrel_semantic_tree::ty::TyKind::TypeParameter(arg_param) = arg.kind() {
                let arg_param_id = arg_param.metadata().id();

                // Check if it's a different type parameter in this position
                // (not self-referential, e.g., U in T's position)
                if arg_param_id != param_id {
                    // Check if it's actually one of the target's type parameters but in wrong position
                    let is_target_param_wrong_position =
                        target_type_params
                            .iter()
                            .enumerate()
                            .any(|(other_index, other_param)| {
                                other_param.metadata().id() == arg_param_id && other_index != index
                            });

                    if is_target_param_wrong_position {
                        // Error: type parameter in wrong position (e.g., extend Pair[U, T])
                        let param_name = param.metadata().name().value.clone();
                        let arg_name = arg_param.metadata().name().value.clone();
                        ctx.diagnostics.throw(
                            crate::diagnostics::TypeParameterWrongPositionError {
                                span: ty_span.clone(),
                                message: format!(
                                    "type parameter '{}' used in wrong position (expected '{}' in position {})",
                                    arg_name, param_name, index
                                ),
                            });
                        return None;
                    }
                }
            }

            // Add all substitutions, including self-referential ones (T -> T)
            // The cycle detection in Substitutions::apply will handle self-referential cases gracefully
            substitutions.insert(param_id, arg.clone());
        }
    }

    // Build the resolved type based on target symbol type
    let resolved_ty = match &target_symbol {
        ExtendableSymbol::Struct(struct_sym) => {
            Ty::generic_struct(struct_sym.clone(), substitutions, ty_span)
        },
        ExtendableSymbol::Enum(enum_sym) => {
            Ty::generic_enum(enum_sym.clone(), substitutions, ty_span)
        },
        ExtendableSymbol::Protocol(protocol_sym) => {
            // Protocol extensions don't use substitutions - they apply to all conforming types
            Ty::protocol(protocol_sym.clone(), ty_span)
        },
    };

    Some((resolved_ty, target_symbol, type_args, referenced_params))
}

/// Resolve type arguments for an extension target.
///
/// This handles the special case where type arguments like T and U in `extend Pair[T, U]`
/// should resolve to the struct's type parameters, not be looked up in scope.
///
/// Returns None if there's a type parameter count mismatch, indicating an error was emitted.
fn resolve_extension_type_arguments(
    ty_path_node: &SyntaxNode,
    source: &str,
    file_id: usize,
    struct_type_params: &[Arc<TypeParameterSymbol>],
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
) -> Option<Vec<Ty>> {
    let arg_list = match ty_path_node
        .children()
        .find(|c| c.kind() == SyntaxKind::TypeArgumentList)
    {
        Some(list) => list,
        None => return Some(Vec::new()),
    };

    // Count the type arguments in the syntax
    let ty_nodes: Vec<_> = arg_list
        .children()
        .filter(|c| c.kind() == SyntaxKind::Ty)
        .collect();
    let arg_count = ty_nodes.len();
    let expected_count = struct_type_params.len();

    // Check for type parameter count mismatch
    if arg_count != expected_count {
        let arg_list_span = get_node_span(&arg_list, file_id);
        ctx.diagnostics.throw(WrongTypeParameterCountError {
            span: arg_list_span,
            expected: expected_count,
            actual: arg_count,
        });
        return None;
    }

    let mut type_args = Vec::new();

    for ty_node in ty_nodes {
        let ty_span = get_node_span(&ty_node, file_id);

        // Try to extract a simple identifier from this type argument
        let simple_name = extract_simple_type_name(&ty_node);

        if let Some(name) = simple_name {
            // Check if this name matches any of the struct's type parameters
            if let Some(type_param) = struct_type_params
                .iter()
                .find(|p| p.metadata().name().value == name)
            {
                // This is a reference to the struct's type parameter
                type_args.push(Ty::type_parameter(type_param.clone(), ty_span));
                continue;
            }
        }

        // Not a type parameter reference - resolve as a normal type
        let mut type_resolver =
            TypeResolver::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
        type_args.push(type_resolver.resolve(&ty_node));
    }

    Some(type_args)
}

/// Extract a simple type name from a Ty node, if it's just a simple identifier.
/// Returns None if the type is complex (has type arguments, is a function type, etc.)
fn extract_simple_type_name(ty_node: &SyntaxNode) -> Option<String> {
    // Find TyPath
    let ty_path = ty_node
        .children()
        .find(|c| c.kind() == SyntaxKind::TyPath)?;

    // Check it doesn't have type arguments (which would make it not a simple name)
    if ty_path
        .children()
        .any(|c| c.kind() == SyntaxKind::TypeArgumentList)
    {
        return None;
    }

    // Find Path
    let path = ty_path.children().find(|c| c.kind() == SyntaxKind::Path)?;

    // Extract segments
    let segments = extract_path_segments(&path);

    // Must be exactly one segment for a simple type name
    if segments.len() == 1 {
        Some(segments[0].clone())
    } else {
        None
    }
}

/// Collect type parameter symbols referenced in type arguments
fn collect_referenced_type_params(type_args: &[Ty]) -> Vec<Arc<TypeParameterSymbol>> {
    let mut params = Vec::new();

    for ty in type_args {
        if let TyKind::TypeParameter(symbol) = ty.kind() {
            params.push(symbol.clone());
        }
        // Note: We could recursively search nested types if needed
    }

    params
}

/// Format a TyKind for error messages
fn format_type_kind(kind: &TyKind) -> String {
    match kind {
        TyKind::Int { .. } => "lang.i*".to_string(),
        TyKind::Float { .. } => "lang.f*".to_string(),
        TyKind::Bool => "lang.i1".to_string(),
        TyKind::String => "lang.str".to_string(),
        TyKind::Protocol { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::Struct { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::TypeAlias { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::TypeParameter(symbol) => symbol.metadata().name().value.clone(),
        TyKind::SelfType => "Self".to_string(),
        TyKind::Error => "<error>".to_string(),
        _ => "<unknown>".to_string(),
    }
}

/// Error for trying to extend a non-extendable type
#[derive(Debug, Clone)]
pub struct CannotExtendTypeError {
    pub span: kestrel_span::Span,
    pub type_name: String,
}

impl kestrel_reporting::IntoDiagnostic for CannotExtendTypeError {
    fn into_diagnostic(&self) -> kestrel_reporting::Diagnostic<usize> {
        kestrel_reporting::Diagnostic::error()
            .with_message(format!(
                "cannot extend type '{}' - only structs, enums, and protocols can be extended",
                self.type_name
            ))
            .with_labels(vec![
                kestrel_reporting::Label::primary(self.span.file_id, self.span.range())
                    .with_message("not a struct, enum, or protocol type"),
            ])
            .with_notes(vec![
                "Extensions can only be applied to struct, enum, and protocol types".to_string(),
            ])
    }
}

/// Error for wrong number of type parameters in extension target
#[derive(Debug, Clone)]
pub struct WrongTypeParameterCountError {
    pub span: kestrel_span::Span,
    pub expected: usize,
    pub actual: usize,
}

impl kestrel_reporting::IntoDiagnostic for WrongTypeParameterCountError {
    fn into_diagnostic(&self) -> kestrel_reporting::Diagnostic<usize> {
        let message = if self.actual > self.expected {
            format!(
                "too many type parameters: expected {}, found {}",
                self.expected, self.actual
            )
        } else {
            format!(
                "too few type parameters: expected {}, found {}",
                self.expected, self.actual
            )
        };
        kestrel_reporting::Diagnostic::error()
            .with_message(message)
            .with_labels(vec![
                kestrel_reporting::Label::primary(self.span.file_id, self.span.range())
                    .with_message(format!("expected {} type parameter(s)", self.expected)),
            ])
    }
}

/// Combine inherited where clause from struct with extension's own where clause.
fn combine_where_clauses(inherited: WhereClause, extension: WhereClause) -> WhereClause {
    let mut combined_constraints = inherited.constraints.to_vec();
    combined_constraints.extend(extension.constraints.iter().cloned());
    WhereClause::with_constraints(combined_constraints)
}
