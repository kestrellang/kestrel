//! Analyzer for generic type parameter constraints
//!
//! Validates generic type declarations and usages:
//! - Duplicate type parameter names
//! - Default ordering (defaults must come after non-defaults)
//! - Bounds reference valid protocols (future)

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_model::GenericsDataFor;
use kestrel_semantic_tree::error::{
    DefaultOrderingError, DuplicateTypeParameterError, NonProtocolBoundError,
    UndeclaredTypeParameterError,
};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Constraint, TyKind, WhereClause};
use kestrel_span::Span;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

pub struct GenericsAnalyzer;

impl GenericsAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl Default for GenericsAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for GenericsAnalyzer {
    fn name(&self) -> &'static str {
        "generics"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        let symbol_id = symbol.metadata().id();
        let Some(generics) = ctx.model.query(GenericsDataFor { symbol_id }) else {
            return;
        };
        let type_params = generics.type_params;
        let where_clause = generics.where_clause;
        validate_type_parameters(&type_params, ctx);
        validate_where_clause(&where_clause, &type_params, ctx);
    }
}

fn validate_type_parameters(type_params: &[Arc<TypeParameterSymbol>], ctx: &mut AnalysisContext) {
    if type_params.is_empty() {
        return;
    }
    check_duplicate_type_params(type_params, ctx);
    check_default_ordering(type_params, ctx);
}

fn check_duplicate_type_params(
    type_params: &[Arc<TypeParameterSymbol>],
    ctx: &mut AnalysisContext,
) {
    let mut seen: HashMap<String, Span> = HashMap::new();
    for param in type_params {
        let name = param.metadata().name().value.clone();
        let span = param.metadata().name().span.clone();
        if let Some(original_span) = seen.get(&name) {
            ctx.report(DuplicateTypeParameterError {
                name: name.clone(),
                duplicate_span: span,
                original_span: original_span.clone(),
            });
        } else {
            seen.insert(name, span);
        }
    }
}

fn check_default_ordering(type_params: &[Arc<TypeParameterSymbol>], ctx: &mut AnalysisContext) {
    let mut first_with_default: Option<&Arc<TypeParameterSymbol>> = None;
    for param in type_params {
        if param.default().is_some() {
            if first_with_default.is_none() {
                first_with_default = Some(param);
            }
        } else if let Some(prev_with_default) = first_with_default {
            ctx.report(DefaultOrderingError {
                param_with_default: prev_with_default.metadata().name().value.clone(),
                param_without_default: param.metadata().name().value.clone(),
                with_default_span: prev_with_default.metadata().name().span.clone(),
                without_default_span: param.metadata().name().span.clone(),
            });
            break;
        }
    }
}

fn validate_where_clause(
    where_clause: &WhereClause,
    type_params: &[Arc<TypeParameterSymbol>],
    ctx: &mut AnalysisContext,
) {
    if where_clause.is_empty() {
        return;
    }
    for constraint in &where_clause.constraints {
        validate_constraint(constraint, type_params, ctx);
    }
}

fn validate_constraint(
    constraint: &Constraint,
    type_params: &[Arc<TypeParameterSymbol>],
    ctx: &mut AnalysisContext,
) {
    match constraint {
        Constraint::TypeBound {
            param,
            param_name,
            param_span,
            bounds,
        } => {
            if param.is_none() {
                let available: Vec<String> = type_params
                    .iter()
                    .map(|p| p.metadata().name().value.clone())
                    .collect();
                ctx.report(UndeclaredTypeParameterError {
                    name: param_name.clone(),
                    span: param_span.clone(),
                    available,
                });
            }
            for bound in bounds {
                validate_bound_type(bound, ctx);
            }
        }
        Constraint::InheritedAssociatedTypeBound { bounds, .. } => {
            for bound in bounds {
                validate_bound_type(bound, ctx);
            }
        }
        Constraint::NegativeBound {
            param,
            param_name,
            param_span,
            bound,
        } => {
            // Validate that the type parameter exists
            if param.is_none() {
                let available: Vec<String> = type_params
                    .iter()
                    .map(|p| p.metadata().name().value.clone())
                    .collect();
                ctx.report(UndeclaredTypeParameterError {
                    name: param_name.clone(),
                    span: param_span.clone(),
                    available,
                });
            }
            // Validate the negative bound is a valid protocol
            validate_bound_type(bound, ctx);
        }
        Constraint::TypeEquality { .. } => { /* validated elsewhere */ }
    }
}

fn validate_bound_type(bound: &kestrel_semantic_tree::ty::Ty, ctx: &mut AnalysisContext) {
    match bound.kind() {
        TyKind::Protocol { .. } => {}
        TyKind::Error => {}
        TyKind::Struct { symbol, .. } => {
            ctx.report(NonProtocolBoundError {
                type_name: symbol.metadata().name().value.clone(),
                type_kind: "struct".to_string(),
                span: bound.span().clone(),
            });
        }
        TyKind::TypeAlias { symbol, .. } => {
            ctx.report(NonProtocolBoundError {
                type_name: symbol.metadata().name().value.clone(),
                type_kind: "type alias".to_string(),
                span: bound.span().clone(),
            });
        }
        TyKind::TypeParameter(param) => {
            ctx.report(NonProtocolBoundError {
                type_name: param.metadata().name().value.clone(),
                type_kind: "type parameter".to_string(),
                span: bound.span().clone(),
            });
        }
        _ => {
            ctx.report(NonProtocolBoundError {
                type_name: format!("{:?}", bound.kind()),
                type_kind: "invalid type".to_string(),
                span: bound.span().clone(),
            });
        }
    }
}
