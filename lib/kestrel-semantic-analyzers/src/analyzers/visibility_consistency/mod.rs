//! Analyzer for visibility consistency
//!
//! Ensures that public APIs don't expose less-visible types:
//! - Public functions can't have private/internal return types
//! - Public functions can't have private/internal parameter types
//! - Public type aliases can't alias private/internal types
//! - Public fields can't have private/internal types

use std::sync::Arc;

use kestrel_semantic_model::ResolvedAliasedType;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::visibility::Visibility;
use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::{
    AliasedTypeLessVisibleError, FieldTypeLessVisibleError, ParameterTypeLessVisibleError,
    ReturnTypeLessVisibleError,
};

pub struct VisibilityConsistencyAnalyzer;

impl VisibilityConsistencyAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl Default for VisibilityConsistencyAnalyzer {
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

fn get_symbol_visibility_level(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> VisibilityLevel {
    let vis = symbol
        .metadata()
        .get_behavior::<VisibilityBehavior>()
        .and_then(|vb| vb.visibility().cloned());
    VisibilityLevel::from_visibility(vis.as_ref())
}

fn get_visibility_level_from_symbol<S: Symbol<KestrelLanguage>>(
    symbol: &Arc<S>,
) -> VisibilityLevel {
    let vis = symbol
        .metadata()
        .get_behavior::<VisibilityBehavior>()
        .and_then(|vb| vb.visibility().cloned());
    VisibilityLevel::from_visibility(vis.as_ref())
}

/// Check if a type exposes a less-visible symbol, returns the offending type name and visibility
fn find_less_visible_type(
    ty: &Ty,
    required_level: VisibilityLevel,
) -> Option<(String, VisibilityLevel)> {
    match ty.kind() {
        TyKind::TypeParameter(_) => None,
        TyKind::Struct {
            symbol: struct_symbol,
            substitutions,
        } => {
            let level = get_visibility_level_from_symbol(struct_symbol);
            if level < required_level {
                return Some((struct_symbol.metadata().name().value.clone(), level));
            }
            for (_, arg_ty) in substitutions.iter() {
                if let Some(result) = find_less_visible_type(arg_ty, required_level) {
                    return Some(result);
                }
            }
            None
        }
        TyKind::Enum {
            symbol: enum_symbol,
            substitutions,
        } => {
            let level = get_visibility_level_from_symbol(enum_symbol);
            if level < required_level {
                return Some((enum_symbol.metadata().name().value.clone(), level));
            }
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
        TyKind::Array(element_type) | TyKind::Pointer(element_type) => {
            find_less_visible_type(element_type, required_level)
        }
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
        TyKind::AssociatedType {
            symbol: assoc_symbol,
            container,
        } => {
            // Associated types in protocols inherit the protocol's visibility
            // (they have no explicit visibility keyword in syntax)
            let level = {
                let direct_level = get_visibility_level_from_symbol(assoc_symbol);

                // Check if parent is a protocol with higher visibility
                if let Some(parent) = assoc_symbol.metadata().parent() {
                    if parent.metadata().kind() == KestrelSymbolKind::Protocol {
                        let protocol_level = get_symbol_visibility_level(&parent);
                        // Use the higher of direct or protocol visibility
                        std::cmp::max(direct_level, protocol_level)
                    } else {
                        direct_level
                    }
                } else {
                    direct_level
                }
            };

            if level < required_level {
                return Some((assoc_symbol.metadata().name().value.clone(), level));
            }
            if let Some(container_ty) = container {
                if let Some(result) = find_less_visible_type(container_ty, required_level) {
                    return Some(result);
                }
            }
            None
        }
        TyKind::UnresolvedFunction {
            param_info,
            return_type,
        } => {
            use kestrel_semantic_tree::ty::ParamInfo;

            // Check return type
            if let Some(result) = find_less_visible_type(return_type, required_level) {
                return Some(result);
            }
            // Check param info types
            match param_info {
                ParamInfo::ImplicitIt { it_type } => {
                    find_less_visible_type(it_type, required_level)
                }
                ParamInfo::Explicit { param_types } => {
                    for pt in param_types {
                        if let Some(result) = find_less_visible_type(pt, required_level) {
                            return Some(result);
                        }
                    }
                    None
                }
                ParamInfo::Unconstrained => None,
            }
        }
        TyKind::Unit
        | TyKind::Never
        | TyKind::Int(_)
        | TyKind::Float(_)
        | TyKind::Bool
        | TyKind::String
        | TyKind::Error
        | TyKind::SelfType
        | TyKind::Infer => None,
    }
}

impl Analyzer for VisibilityConsistencyAnalyzer {
    fn name(&self) -> &'static str {
        "visibility_consistency"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        let kind = symbol.metadata().kind();
        let symbol_level = get_symbol_visibility_level(symbol);

        // Check if this is a method in a public protocol
        let is_method_in_public_protocol = kind == KestrelSymbolKind::Function
            && symbol.metadata().parent().map_or(false, |p| {
                p.metadata().kind() == KestrelSymbolKind::Protocol
                    && get_symbol_visibility_level(&p) == VisibilityLevel::Public
            });

        if symbol_level == VisibilityLevel::Public || is_method_in_public_protocol {
            match kind {
                KestrelSymbolKind::Function => check_function_visibility(symbol, ctx),
                KestrelSymbolKind::TypeAlias => check_type_alias_visibility(symbol, ctx),
                KestrelSymbolKind::Field => check_field_visibility(symbol, ctx),
                _ => {}
            }
        }
    }
}

fn check_function_visibility(symbol: &Arc<dyn Symbol<KestrelLanguage>>, ctx: &mut AnalysisContext) {
    let name = &symbol.metadata().name().value;
    let span = symbol.metadata().declaration_span().clone();

    let Ok(func_sym) = symbol.clone().downcast_arc::<FunctionSymbol>() else {
        return;
    };
    let Some(callable) = func_sym.callable() else {
        return;
    };

    if let Some((_type_name, type_level)) =
        find_less_visible_type(callable.return_type(), VisibilityLevel::Public)
    {
        ctx.report(ReturnTypeLessVisibleError {
            span: span.clone(),
            function_name: name.clone(),
            function_visibility: "public".to_string(),
            return_type_visibility: type_level.name().to_string(),
        });
    }

    for param in callable.parameters() {
        if let Some((_type_name, type_level)) =
            find_less_visible_type(&param.ty, VisibilityLevel::Public)
        {
            ctx.report(ParameterTypeLessVisibleError {
                span: span.clone(),
                function_name: name.clone(),
                function_visibility: "public".to_string(),
                param_type_visibility: type_level.name().to_string(),
            });
        }
    }
}

fn check_type_alias_visibility(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    ctx: &mut AnalysisContext,
) {
    let name = &symbol.metadata().name().value;
    let span = symbol.metadata().declaration_span().clone();
    let alias_id = symbol.metadata().id();
    let Some(resolved) = ctx.model.query(ResolvedAliasedType {
        type_alias_id: alias_id,
    }) else {
        return;
    };
    if let Some((_type_name, type_level)) =
        find_less_visible_type(&resolved, VisibilityLevel::Public)
    {
        ctx.report(AliasedTypeLessVisibleError {
            span,
            alias_name: name.clone(),
            alias_visibility: "public".to_string(),
            aliased_type_visibility: type_level.name().to_string(),
        });
    }
}

fn check_field_visibility(symbol: &Arc<dyn Symbol<KestrelLanguage>>, ctx: &mut AnalysisContext) {
    let name = &symbol.metadata().name().value;
    let span = symbol.metadata().declaration_span().clone();
    if let Some(typed) = symbol.metadata().get_behavior::<TypedBehavior>() {
        if let Some((_type_name, type_level)) =
            find_less_visible_type(typed.ty(), VisibilityLevel::Public)
        {
            ctx.report(FieldTypeLessVisibleError {
                span,
                field_name: name.clone(),
                field_visibility: "public".to_string(),
                field_type_visibility: type_level.name().to_string(),
            });
        }
    }
}
