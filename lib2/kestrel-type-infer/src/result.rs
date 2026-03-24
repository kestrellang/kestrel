//! Type inference output: fully resolved types and resolution tables.
//!
//! `TypedBody` is the final output of type inference for a single body.
//! All TyVars have been resolved to `ResolvedTy`.

use std::collections::HashMap;

use kestrel_hecs::Entity;
use kestrel_hir::body::HirExprId;
use kestrel_hir::res::LocalId;

use crate::ctx::InferCtx;
use crate::error::InferError;
use crate::ty::{TyKind, TySlot, TyVar};

/// Result of type inference for a single body.
#[derive(Clone, Debug)]
pub struct TypedBody {
    /// Type of every expression.
    pub expr_types: HashMap<HirExprId, ResolvedTy>,

    /// Type of every local variable.
    pub local_types: HashMap<LocalId, ResolvedTy>,

    /// Resolved entity for MethodCall/Field expressions.
    /// Used by codegen to know which function to call.
    pub resolutions: HashMap<HirExprId, Entity>,

    /// Promotion info for expressions that need wrapping.
    /// Codegen inserts FromValue.from() calls at these sites.
    pub promotions: HashMap<HirExprId, ResolvedPromotion>,

    /// Inferred type arguments for generic calls.
    pub type_args: HashMap<HirExprId, Vec<ResolvedTy>>,

    /// Errors accumulated during inference.
    pub errors: Vec<InferError>,

    /// Human-readable error descriptions with resolved types.
    pub error_details: Vec<String>,
}

/// Manual Hash: hash each map as sorted (key, value) pairs for determinism.
/// Uses raw index values for sorting since Idx<T> doesn't implement Ord.
impl std::hash::Hash for TypedBody {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.errors.len().hash(state);

        // Hash expr_types
        let mut expr_pairs: Vec<_> = self.expr_types.iter().collect();
        expr_pairs.sort_by_key(|(k, _)| k.raw());
        for (k, v) in &expr_pairs {
            k.hash(state);
            v.hash(state);
        }

        // Hash local_types
        let mut local_pairs: Vec<_> = self.local_types.iter().collect();
        local_pairs.sort_by_key(|(k, _)| k.raw());
        for (k, v) in &local_pairs {
            k.hash(state);
            v.hash(state);
        }

        // Hash resolutions
        let mut res_pairs: Vec<_> = self.resolutions.iter().collect();
        res_pairs.sort_by_key(|(k, _)| k.raw());
        for (k, v) in &res_pairs {
            k.hash(state);
            v.hash(state);
        }
    }
}

/// A fully resolved type (no TyVars).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResolvedTy {
    Named {
        entity: Entity,
        args: Vec<ResolvedTy>,
    },
    Param {
        entity: Entity,
    },
    Tuple(Vec<ResolvedTy>),
    Function {
        params: Vec<ResolvedTy>,
        ret: Box<ResolvedTy>,
    },
    Never,
    Error,
}

/// Resolved promotion info (no TyVars).
#[derive(Clone, Debug)]
pub struct ResolvedPromotion {
    pub method: Entity,
    pub target: ResolvedTy,
}

/// Resolve a TyVar into a fully concrete ResolvedTy.
/// Unresolved TyVars default to Error (shouldn't happen after solving).
fn resolve_to_concrete(ctx: &InferCtx<'_>, tv: TyVar) -> ResolvedTy {
    let resolved = ctx.resolve(tv);
    match &ctx.types[resolved.0 as usize] {
        TySlot::Resolved(kind) => kind_to_resolved(ctx, kind),
        TySlot::Unresolved { .. } => ResolvedTy::Error,
        TySlot::Redirect(_) => unreachable!("resolve() follows redirects"),
    }
}

/// Convert a TyKind to a ResolvedTy, recursively resolving any TyVar args.
fn kind_to_resolved(ctx: &InferCtx<'_>, kind: &TyKind) -> ResolvedTy {
    match kind {
        TyKind::Named { entity, args } => ResolvedTy::Named {
            entity: *entity,
            args: args.iter().map(|&tv| resolve_to_concrete(ctx, tv)).collect(),
        },
        TyKind::Param { entity } => ResolvedTy::Param { entity: *entity },
        TyKind::Tuple(elems) => {
            ResolvedTy::Tuple(elems.iter().map(|&tv| resolve_to_concrete(ctx, tv)).collect())
        }
        TyKind::Function { params, ret } => ResolvedTy::Function {
            params: params.iter().map(|&tv| resolve_to_concrete(ctx, tv)).collect(),
            ret: Box::new(resolve_to_concrete(ctx, *ret)),
        },
        TyKind::Never => ResolvedTy::Never,
        TyKind::Error => ResolvedTy::Error,
    }
}

/// Build the final TypedBody from the completed InferCtx.
pub fn build_result(ctx: &InferCtx<'_>) -> TypedBody {
    let expr_types = ctx
        .expr_types
        .iter()
        .map(|(&id, &tv)| (id, resolve_to_concrete(ctx, tv)))
        .collect();

    let local_types = ctx
        .local_types
        .iter()
        .map(|(&id, &tv)| (id, resolve_to_concrete(ctx, tv)))
        .collect();

    let resolutions = ctx.resolutions.clone();

    let promotions = ctx
        .promotions
        .iter()
        .map(|(&id, info)| {
            (
                id,
                ResolvedPromotion {
                    method: info.method,
                    target: resolve_to_concrete(ctx, info.target_ty),
                },
            )
        })
        .collect();

    let type_args = ctx
        .type_args
        .iter()
        .map(|(&id, tvs)| {
            (
                id,
                tvs.iter().map(|&tv| resolve_to_concrete(ctx, tv)).collect(),
            )
        })
        .collect();

    let error_details = ctx.errors.iter().map(|err| describe_error(ctx, err)).collect();

    TypedBody {
        expr_types,
        local_types,
        resolutions,
        promotions,
        type_args,
        errors: ctx.errors.clone(),
        error_details,
    }
}

/// Describe a TyVar as a short type name string (for diagnostics).
fn describe_tyvar(ctx: &InferCtx<'_>, tv: TyVar) -> String {
    let resolved = ctx.resolve(tv);
    match &ctx.types[resolved.0 as usize] {
        TySlot::Resolved(kind) => describe_tykind(ctx, kind),
        TySlot::Unresolved { .. } => "?".into(),
        TySlot::Redirect(_) => "?redirect".into(),
    }
}

/// Describe a TyKind as a short type name string.
fn describe_tykind(ctx: &InferCtx<'_>, kind: &TyKind) -> String {
    match kind {
        TyKind::Named { entity, args } => {
            let name = ctx.query_ctx
                .get::<kestrel_ast_builder::Name>(*entity)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| {
                    // Show NodeKind + entity for unnamed entities
                    let kind = ctx.query_ctx.get::<kestrel_ast_builder::NodeKind>(*entity)
                        .map(|k| format!("{:?}", k))
                        .unwrap_or_else(|| "?".into());
                    format!("{}({:?})", kind, entity)
                });
            if args.is_empty() {
                name
            } else {
                let arg_strs: Vec<_> = args.iter().map(|&tv| describe_tyvar(ctx, tv)).collect();
                format!("{}[{}]", name, arg_strs.join(", "))
            }
        }
        TyKind::Param { entity } => {
            ctx.query_ctx
                .get::<kestrel_ast_builder::Name>(*entity)
                .map(|n| n.0.clone())
                .unwrap_or("Param".into())
        }
        TyKind::Tuple(elems) => {
            if elems.is_empty() {
                "()".into()
            } else {
                let strs: Vec<_> = elems.iter().map(|&tv| describe_tyvar(ctx, tv)).collect();
                format!("({})", strs.join(", "))
            }
        }
        TyKind::Function { params, ret } => {
            let p: Vec<_> = params.iter().map(|&tv| describe_tyvar(ctx, tv)).collect();
            format!("({}) -> {}", p.join(", "), describe_tyvar(ctx, *ret))
        }
        TyKind::Never => "Never".into(),
        TyKind::Error => "Error".into(),
    }
}

/// Build a human-readable error description with resolved types.
fn describe_error(ctx: &InferCtx<'_>, err: &InferError) -> String {
    match err {
        InferError::TypeMismatch { expected, got, .. } => {
            format!("expected {} got {}", describe_tyvar(ctx, *expected), describe_tyvar(ctx, *got))
        }
        InferError::DoesNotConform { ty, protocol, .. } => {
            let ty_name = describe_tyvar(ctx, *ty);
            let proto_name = ctx.query_ctx
                .get::<kestrel_ast_builder::Name>(*protocol)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| format!("{:?}", protocol));
            format!("{} !: {}", ty_name, proto_name)
        }
        InferError::NoMember { receiver, name, .. } => {
            format!("{}.{} not found", describe_tyvar(ctx, *receiver), name)
        }
        InferError::AmbiguousMember { receiver, name, .. } => {
            format!("{}.{} ambiguous", describe_tyvar(ctx, *receiver), name)
        }
        InferError::MemberNotVisible { receiver, name, .. } => {
            format!("{}.{} not visible", describe_tyvar(ctx, *receiver), name)
        }
        InferError::NoAssociatedType { container, name, .. } => {
            format!("{}.{} no assoc type", describe_tyvar(ctx, *container), name)
        }
        InferError::ImplicitMemberNotFound { expected, name, .. } => {
            format!(".{} not found on {}", name, describe_tyvar(ctx, *expected))
        }
        InferError::InfiniteType { .. } => "infinite type".into(),
        InferError::FromHir { .. } => "from-hir".into(),
        InferError::ArgCountMismatch { expected, got, .. } => {
            format!("expected {} argument(s), got {}", expected, got)
        }
        InferError::LabelMismatch { expected, got, .. } => {
            let exp = expected.as_deref().unwrap_or("_");
            let g = got.as_deref().unwrap_or("_");
            format!("wrong label: expected '{}', got '{}'", exp, g)
        }
        InferError::InstanceMethodAsStatic { name, .. } => {
            format!("instance method '{}' cannot be called on a type", name)
        }
        InferError::TypeParamAsValue { .. } => {
            "type parameter cannot be used as a value".into()
        }
    }
}
