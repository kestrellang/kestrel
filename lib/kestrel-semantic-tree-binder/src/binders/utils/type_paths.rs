use kestrel_semantic_model::{ResolveTypePath, TypePathResolution};
use kestrel_semantic_tree::ty::{Ty, TyKind};
use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::declaration_binder::BindingContext;
use crate::diagnostics::{NotAProtocolContext, NotAProtocolError, UnresolvedTypeError};

fn apply_default_protocol_type_arguments_for_bound(ty: Ty) -> Ty {
    let TyKind::Protocol {
        symbol,
        substitutions,
    } = ty.kind()
    else {
        return ty;
    };

    let type_params = symbol.type_parameters();
    if type_params.is_empty() {
        return ty;
    }

    let mut new_subs = substitutions.clone();
    let mut changed = false;

    for param in &type_params {
        let param_id = param.metadata().id();
        if new_subs.contains(param_id) {
            continue;
        }
        if let Some(default_ty) = param.default() {
            new_subs.insert(param_id, default_ty.clone());
            changed = true;
        }
    }

    if changed {
        Ty::generic_protocol(symbol.clone(), new_subs, ty.span().clone())
    } else {
        ty
    }
}

pub(crate) fn resolve_protocol_bound_path(
    segments: &[String],
    span: Span,
    context_id: SymbolId,
    ctx: &mut BindingContext,
) -> Ty {
    if segments.is_empty() {
        return Ty::error(span);
    }

    let bound_name = segments.join(".");
    match ctx.model.query(ResolveTypePath {
        path: segments.to_vec(),
        context: context_id,
    }) {
        TypePathResolution::Resolved(resolved_ty) => match resolved_ty.kind() {
            TyKind::Protocol { .. } => apply_default_protocol_type_arguments_for_bound(resolved_ty),
            TyKind::Struct { symbol, .. } => {
                ctx.diagnostics.throw(NotAProtocolError {
                    span: span.clone(),
                    name: symbol.metadata().name().value.clone(),
                    context: NotAProtocolContext::Bound,
                });
                Ty::error(span)
            },
            TyKind::TypeAlias { symbol, .. } => {
                ctx.diagnostics.throw(NotAProtocolError {
                    span: span.clone(),
                    name: symbol.metadata().name().value.clone(),
                    context: NotAProtocolContext::Bound,
                });
                Ty::error(span)
            },
            _ => {
                ctx.diagnostics.throw(NotAProtocolError {
                    span: span.clone(),
                    name: bound_name,
                    context: NotAProtocolContext::Bound,
                });
                Ty::error(span)
            },
        },
        TypePathResolution::NotFound { .. } => {
            ctx.diagnostics.throw(UnresolvedTypeError {
                span: span.clone(),
                type_name: bound_name,
            });
            Ty::error(span)
        },
        TypePathResolution::Ambiguous { .. } | TypePathResolution::NotAType { .. } => {
            ctx.diagnostics.throw(NotAProtocolError {
                span: span.clone(),
                name: bound_name,
                context: NotAProtocolContext::Bound,
            });
            Ty::error(span)
        },
    }
}
