//! Validator for visibility consistency
//!
//! Ensures that public APIs don't expose less-visible types:
//! - Public functions can't have private/internal return types
//! - Public functions can't have private/internal parameter types
//! - Public type aliases can't alias private/internal types
//! - Public fields can't have private/internal types

use std::sync::Arc;

use kestrel_semantic_tree::behavior::visibility::Visibility;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasTypedBehavior;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::Symbol;

use crate::diagnostics::{
    AliasedTypeLessVisibleError, FieldTypeLessVisibleError, ParameterTypeLessVisibleError,
    ReturnTypeLessVisibleError,
};
use crate::validation::{SymbolContext, Validator};

/// Validator that ensures visibility consistency
pub struct VisibilityConsistencyValidator;

impl VisibilityConsistencyValidator {
    const NAME: &'static str = "visibility_consistency";

    pub fn new() -> Self {
        Self
    }
}

impl Default for VisibilityConsistencyValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Visibility level for comparison (higher = more visible)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum VisibilityLevel {
    Private = 1,
    Fileprivate = 2,
    Internal = 3,
    Public = 4,
}

impl VisibilityLevel {
    fn from_visibility(vis: Option<&Visibility>) -> Self {
        match vis {
            Some(Visibility::Public) => VisibilityLevel::Public,
            Some(Visibility::Internal) => VisibilityLevel::Internal,
            Some(Visibility::Fileprivate) => VisibilityLevel::Fileprivate,
            Some(Visibility::Private) => VisibilityLevel::Private,
            None => VisibilityLevel::Internal, // Default is internal
        }
    }

    fn name(&self) -> &'static str {
        match self {
            VisibilityLevel::Public => "public",
            VisibilityLevel::Internal => "internal",
            VisibilityLevel::Fileprivate => "fileprivate",
            VisibilityLevel::Private => "private",
        }
    }
}

/// Get the visibility level of a symbol
fn get_symbol_visibility_level(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> VisibilityLevel {
    let vis = symbol
        .visibility_behavior()
        .and_then(|vb| vb.visibility().cloned());
    VisibilityLevel::from_visibility(vis.as_ref())
}

/// Get visibility level from a concrete symbol type
fn get_visibility_level_from_symbol<S: Symbol<KestrelLanguage>>(
    symbol: &Arc<S>,
) -> VisibilityLevel {
    let vis = symbol
        .visibility_behavior()
        .and_then(|vb| vb.visibility().cloned());
    VisibilityLevel::from_visibility(vis.as_ref())
}

/// Check if a type exposes a less-visible symbol, returns the offending type name and visibility
fn find_less_visible_type(
    ty: &Ty,
    required_level: VisibilityLevel,
) -> Option<(String, VisibilityLevel)> {
    match ty.kind() {
        TyKind::TypeParameter(_) => {
            // Type parameters are always valid - they are placeholders
            None
        }
        TyKind::Struct {
            symbol: struct_symbol,
            substitutions,
        } => {
            let level = get_visibility_level_from_symbol(struct_symbol);
            if level < required_level {
                return Some((struct_symbol.metadata().name().value.clone(), level));
            }
            // Also check visibility of type arguments
            for (_, arg_ty) in substitutions.iter() {
                if let Some(result) = find_less_visible_type(arg_ty, required_level) {
                    return Some(result);
                }
            }
            None
        }
        TyKind::Protocol {
            symbol: protocol_symbol,
            substitutions,
        } => {
            let level = get_visibility_level_from_symbol(protocol_symbol);
            if level < required_level {
                return Some((protocol_symbol.metadata().name().value.clone(), level));
            }
            // Also check visibility of type arguments
            for (_, arg_ty) in substitutions.iter() {
                if let Some(result) = find_less_visible_type(arg_ty, required_level) {
                    return Some(result);
                }
            }
            None
        }
        TyKind::TypeAlias {
            symbol: alias_symbol,
            substitutions,
        } => {
            let level = get_visibility_level_from_symbol(alias_symbol);
            if level < required_level {
                return Some((alias_symbol.metadata().name().value.clone(), level));
            }
            // Also check visibility of type arguments
            for (_, arg_ty) in substitutions.iter() {
                if let Some(result) = find_less_visible_type(arg_ty, required_level) {
                    return Some(result);
                }
            }
            None
        }
        TyKind::Tuple(elements) => {
            for elem in elements {
                if let Some(result) = find_less_visible_type(elem, required_level) {
                    return Some(result);
                }
            }
            None
        }
        TyKind::Array(element_type) => find_less_visible_type(element_type, required_level),
        TyKind::Function {
            params,
            return_type,
        } => {
            for param in params {
                if let Some(result) = find_less_visible_type(param, required_level) {
                    return Some(result);
                }
            }
            find_less_visible_type(return_type, required_level)
        }
        TyKind::AssociatedType { symbol: assoc_symbol, container } => {
            let level = get_visibility_level_from_symbol(assoc_symbol);
            if level < required_level {
                return Some((assoc_symbol.metadata().name().value.clone(), level));
            }
            // Also check visibility of container type if present
            if let Some(container_ty) = container {
                if let Some(result) = find_less_visible_type(container_ty, required_level) {
                    return Some(result);
                }
            }
            None
        }
        // Primitive types and special types don't have visibility issues
        TyKind::Unit
        | TyKind::Never
        | TyKind::Int(_)
        | TyKind::Float(_)
        | TyKind::Bool
        | TyKind::String
        | TyKind::Error
        | TyKind::SelfType
        | TyKind::TypeVar(_) => None,
    }
}

impl Validator for VisibilityConsistencyValidator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn validate_symbol(&self, ctx: &SymbolContext<'_>) {
        let kind = ctx.symbol.metadata().kind();
        let symbol_level = get_symbol_visibility_level(ctx.symbol);

        // Check if this is a method in a public protocol
        let is_method_in_public_protocol = kind == KestrelSymbolKind::Function
            && ctx.symbol.metadata().parent().map_or(false, |p| {
                p.metadata().kind() == KestrelSymbolKind::Protocol
                    && get_symbol_visibility_level(&p) == VisibilityLevel::Public
            });

        // Check public symbols (they have the strictest requirements)
        // Also check methods in public protocols (they inherit the public requirement)
        if symbol_level == VisibilityLevel::Public || is_method_in_public_protocol {
            match kind {
                KestrelSymbolKind::Function => {
                    check_function_visibility(ctx);
                }
                KestrelSymbolKind::TypeAlias => {
                    check_type_alias_visibility(ctx);
                }
                KestrelSymbolKind::Field => {
                    check_field_visibility(ctx);
                }
                _ => {}
            }
        }
    }
}

/// Check that a public function doesn't expose less-visible types
fn check_function_visibility(ctx: &SymbolContext<'_>) {
    use kestrel_semantic_tree::symbol::function::FunctionSymbol;

    let name = &ctx.symbol.metadata().name().value;
    let span = ctx.symbol.metadata().declaration_span().clone();

    // Get callable behavior from FunctionSymbol directly
    let func_sym = ctx.symbol.as_ref().downcast_ref::<FunctionSymbol>();

    if let Some(func_sym) = func_sym {
        let callable = match func_sym.callable() {
            Some(c) => c,
            None => return,
        };
        // Check return type
        if let Some((_type_name, type_level)) =
            find_less_visible_type(callable.return_type(), VisibilityLevel::Public)
        {
            ctx.diagnostics().get().throw(
                ReturnTypeLessVisibleError {
                    span: span.clone(),
                    function_name: name.clone(),
                    function_visibility: "public".to_string(),
                    return_type_visibility: type_level.name().to_string(),
                });
        }

        // Check parameter types
        for param in callable.parameters() {
            if let Some((_type_name, type_level)) =
                find_less_visible_type(&param.ty, VisibilityLevel::Public)
            {
                ctx.diagnostics().get().throw(ParameterTypeLessVisibleError {
                    span: span.clone(),
                    function_name: name.clone(),
                    function_visibility: "public".to_string(),
                    param_type_visibility: type_level.name().to_string(),
                });
            }
        }
    }
}

/// Check that a public type alias doesn't expose a less-visible type
fn check_type_alias_visibility(ctx: &SymbolContext<'_>) {
    let name = &ctx.symbol.metadata().name().value;
    let span = ctx.symbol.metadata().declaration_span().clone();

    // Get TypeAliasTypedBehavior for the resolved aliased type
    let behaviors = ctx.symbol.metadata().behaviors();
    let typed = behaviors.iter().find_map(|b| {
        if matches!(b.kind(), KestrelBehaviorKind::TypeAliasTyped) {
            b.as_ref().downcast_ref::<TypeAliasTypedBehavior>()
        } else {
            None
        }
    });

    if let Some(typed) = typed {
        if let Some((_type_name, type_level)) =
            find_less_visible_type(typed.resolved_ty(), VisibilityLevel::Public)
        {
            ctx.diagnostics().get().throw(AliasedTypeLessVisibleError {
                span,
                alias_name: name.clone(),
                alias_visibility: "public".to_string(),
                aliased_type_visibility: type_level.name().to_string(),
            });
        }
    }
}

/// Check that a public field doesn't expose a less-visible type
fn check_field_visibility(ctx: &SymbolContext<'_>) {
    let name = &ctx.symbol.metadata().name().value;
    let span = ctx.symbol.metadata().declaration_span().clone();

    // Get TypedBehavior for the field type using the extension trait
    if let Some(typed) = ctx.symbol.typed_behavior() {
        if let Some((_type_name, type_level)) =
            find_less_visible_type(typed.ty(), VisibilityLevel::Public)
        {
            ctx.diagnostics().get().throw(FieldTypeLessVisibleError {
                span,
                field_name: name.clone(),
                field_visibility: "public".to_string(),
                field_type_visibility: type_level.name().to_string(),
            });
        }
    }
}
