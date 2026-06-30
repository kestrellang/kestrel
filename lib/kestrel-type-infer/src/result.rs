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
use crate::ty::{LiteralKind, TyKind, TySlot, TyVar};

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

    /// MethodCall exprs where the call resolved through a field access.
    /// Maps expr → field entity. MIR lowering interposes a field projection
    /// before the call so the receiver is the field value, not `self`.
    pub field_subscripts: HashMap<HirExprId, Entity>,

    /// Promotion info for expressions that need wrapping.
    /// Codegen inserts FromValue.from() calls at these sites.
    pub promotions: HashMap<HirExprId, ResolvedPromotion>,

    /// Inferred type arguments for generic calls.
    pub type_args: HashMap<HirExprId, Vec<ResolvedTy>>,

    /// Errors accumulated during inference.
    pub errors: Vec<InferError>,

    /// Human-readable error descriptions with resolved types.
    pub error_details: Vec<String>,

    /// If this body has an opaque return type (`some P`), the resolved
    /// concrete type that the body's return expressions unified to.
    /// Used by MIR lowering to substitute the opaque type with concrete.
    pub opaque_concrete_type: Option<ResolvedTy>,

    /// `Indirection`-peel plan for member-access exprs that resolved through a
    /// smart-pointer's pointee (`wrapper.m` → `wrapper.pointeeRef().m`). The
    /// `Vec` is the peel CHAIN, outer→inner, for nested wrappers
    /// (`RcBox[Pointer[T]]`). MIR lowering replays it: project the receiver
    /// through each `pointeeRef()`/`pointeeMutRef()` before the member access.
    /// Single source of truth — MIR never re-derives the peel (mirrors
    /// `resolutions` / `ClosureCaptures`).
    pub indirection_peels: HashMap<HirExprId, Vec<IndirectionPeel>>,
}

/// One level of an `Indirection` member peel: the concrete pointee accessors
/// on the wrapper type at this level. `mut_method` is `None` when the wrapper
/// is `Indirection` but not `MutableIndirection` (read-only) — a write through
/// it is the `indirection_no_mutating_accessor` error.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IndirectionPeel {
    /// `pointeeRef()` on the wrapper — yields `&Target` for reads.
    pub read_method: Entity,
    /// `pointeeMutRef()` on the wrapper — yields `&mutating Target` for
    /// writes / RMW / mutating-method receivers; `None` if read-only.
    pub mut_method: Option<Entity>,
    /// The concrete pointee type (`Indirection.Target`) at this level. MIR
    /// builds the `&Target` / `&mutating Target` result type of the accessor
    /// call from this, and projects the member off it.
    pub target: ResolvedTy,
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

        // Hash opaque_concrete_type
        self.opaque_concrete_type.hash(state);

        // Hash indirection_peels (sorted by expr for determinism)
        let mut peel_pairs: Vec<_> = self.indirection_peels.iter().collect();
        peel_pairs.sort_by_key(|(k, _)| k.raw());
        for (k, v) in &peel_pairs {
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
    /// Abstract `Self` inside `extend P` / `protocol P`. Preserved through
    /// inference output so MIR receives `MirTy::SelfType` and monomorphization
    /// substitutes it with the caller's concrete self type.
    SelfType {
        entity: Entity,
    },
    /// Abstract associated-type projection: `base.assoc` (e.g. `I.Item`).
    /// Preserved so MIR lowering can build `MirTy::AssociatedProjection`
    /// with the correct base instead of defaulting to `SelfType`.
    AssocProjection {
        base: Box<ResolvedTy>,
        assoc: Entity,
    },
    Tuple(Vec<ResolvedTy>),
    Function {
        params: Vec<ResolvedTy>,
        /// Parallel to `params`; carries the `mutating` convention to MIR.
        conventions: Vec<kestrel_ast::ParamConvention>,
        ret: Box<ResolvedTy>,
    },
    /// Opaque return type from a call to a function with `some P` return.
    /// Preserved through inference output so MIR lowering can resolve it
    /// to the concrete type by querying InferBody on the origin.
    Opaque {
        origin: Entity,
        bounds: Vec<(Entity, Vec<ResolvedTy>)>,
        origin_args: Vec<ResolvedTy>,
        index: u32,
    },
    /// Second-class reference `&T` / `&mutating T` — a ret_borrow call
    /// result's expression type. MIR lowering PEELS it (a ref-typed value
    /// registers as a @guaranteed pointee value), so consumers must not key
    /// place-ness on this alone: the callee's `CallableRefReturn` is ground
    /// truth (solver ordering can pin a result var to the pointee first).
    Ref {
        pointee: Box<ResolvedTy>,
        mutating: bool,
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
///
/// ResolvedTy is what downstream consumers (MIR, codegen) see. It keeps a
/// single `Named` variant because downstream already dispatches via NodeKind.
/// A leftover `TypeAlias` or `AssocProjection` indicates inference couldn't
/// reduce/resolve it — those surface as Error so consumers don't see ambiguous types.
fn kind_to_resolved(ctx: &InferCtx<'_>, kind: &TyKind) -> ResolvedTy {
    match kind {
        TyKind::Struct { entity, args }
        | TyKind::Enum { entity, args }
        | TyKind::Protocol { entity, args } => ResolvedTy::Named {
            entity: *entity,
            args: args
                .iter()
                .map(|&tv| resolve_to_concrete(ctx, tv))
                .collect(),
        },
        TyKind::TypeAlias { entity, args } => ResolvedTy::Named {
            entity: *entity,
            args: args
                .iter()
                .map(|&tv| resolve_to_concrete(ctx, tv))
                .collect(),
        },
        TyKind::Param { entity } => ResolvedTy::Param { entity: *entity },
        TyKind::SelfType { entity } => ResolvedTy::SelfType { entity: *entity },
        TyKind::AssocProjection { base, assoc } => ResolvedTy::AssocProjection {
            base: Box::new(resolve_to_concrete(ctx, *base)),
            assoc: *assoc,
        },
        TyKind::Tuple(elems) => ResolvedTy::Tuple(
            elems
                .iter()
                .map(|&tv| resolve_to_concrete(ctx, tv))
                .collect(),
        ),
        TyKind::Function {
            params,
            conventions,
            ret,
        } => ResolvedTy::Function {
            params: params
                .iter()
                .map(|&tv| resolve_to_concrete(ctx, tv))
                .collect(),
            conventions: conventions.clone(),
            ret: Box::new(resolve_to_concrete(ctx, *ret)),
        },
        TyKind::Opaque {
            origin,
            bounds,
            origin_args,
            index,
        } => ResolvedTy::Opaque {
            origin: *origin,
            bounds: bounds
                .iter()
                .map(|(e, args)| {
                    (
                        *e,
                        args.iter()
                            .map(|&tv| resolve_to_concrete(ctx, tv))
                            .collect(),
                    )
                })
                .collect(),
            origin_args: origin_args
                .iter()
                .map(|&tv| resolve_to_concrete(ctx, tv))
                .collect(),
            index: *index,
        },
        TyKind::Ref { pointee, mutating } => ResolvedTy::Ref {
            pointee: Box::new(resolve_to_concrete(ctx, *pointee)),
            mutating: *mutating,
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

    // If this body has an opaque return type, resolve the concrete TyVar
    let opaque_concrete_type = ctx
        .opaque_return
        .as_ref()
        .map(|info| resolve_to_concrete(ctx, info.concrete_tv));

    TypedBody {
        expr_types,
        local_types,
        resolutions,
        field_subscripts: ctx.field_subscripts.clone(),
        promotions,
        type_args,
        errors: ctx.errors.clone(),
        error_details: ctx.error_details.clone(),
        opaque_concrete_type,
        indirection_peels: ctx
            .indirection_peels
            .iter()
            .map(|(&expr, peels)| {
                let resolved: Vec<IndirectionPeel> = peels
                    .iter()
                    .map(|p| IndirectionPeel {
                        read_method: p.read_method,
                        mut_method: p.mut_method,
                        target: resolve_to_concrete(ctx, p.target_tv),
                    })
                    .collect();
                (expr, resolved)
            })
            .collect(),
    }
}

/// Describe a TyVar as a short type name string (for diagnostics).
pub(crate) fn describe_tyvar(ctx: &InferCtx<'_>, tv: TyVar) -> String {
    let resolved = ctx.resolve(tv);
    match &ctx.types[resolved.0 as usize] {
        TySlot::Resolved(kind) => describe_tykind(ctx, kind),
        TySlot::Unresolved { literal: Some(lit) } => literal_kind_name(*lit).into(),
        TySlot::Unresolved { literal: None } => "?".into(),
        TySlot::Redirect(_) => "?redirect".into(),
    }
}

/// Human-readable name for a literal kind (used when the TyVar never got a concrete type).
fn literal_kind_name(lit: LiteralKind) -> &'static str {
    match lit {
        LiteralKind::Integer => "integer literal",
        LiteralKind::Float => "float literal",
        LiteralKind::String => "string literal",
        LiteralKind::Bool => "bool literal",
        LiteralKind::Char => "char literal",
        LiteralKind::Null => "null literal",
        LiteralKind::Array => "array literal",
        LiteralKind::Dictionary => "dictionary literal",
        LiteralKind::StringInterpolation => "string interpolation",
    }
}

/// Describe a TyKind as a short type name string.
fn describe_tykind(ctx: &InferCtx<'_>, kind: &TyKind) -> String {
    match kind {
        TyKind::Struct { entity, args }
        | TyKind::Enum { entity, args }
        | TyKind::Protocol { entity, args }
        | TyKind::TypeAlias { entity, args } => {
            let name = ctx
                .query_ctx
                .get::<kestrel_ast_builder::Name>(*entity)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| {
                    // Show NodeKind + entity for unnamed entities
                    let kind = ctx
                        .query_ctx
                        .get::<kestrel_ast_builder::NodeKind>(*entity)
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
        },
        TyKind::Param { entity } => ctx
            .query_ctx
            .get::<kestrel_ast_builder::Name>(*entity)
            .map(|n| n.0.clone())
            .unwrap_or("Param".into()),
        TyKind::SelfType { .. } => "Self".into(),
        TyKind::AssocProjection { base, assoc } => {
            let base_str = describe_tyvar(ctx, *base);
            let assoc_name = ctx
                .query_ctx
                .get::<kestrel_ast_builder::Name>(*assoc)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| format!("{:?}", assoc));
            format!("{}.{}", base_str, assoc_name)
        },
        TyKind::Ref { pointee, mutating } => {
            let prefix = if *mutating { "&mutating " } else { "&" };
            format!("{prefix}{}", describe_tyvar(ctx, *pointee))
        },
        TyKind::Tuple(elems) => {
            if elems.is_empty() {
                "()".into()
            } else {
                let strs: Vec<_> = elems.iter().map(|&tv| describe_tyvar(ctx, tv)).collect();
                format!("({})", strs.join(", "))
            }
        },
        TyKind::Function { params, ret, .. } => {
            let p: Vec<_> = params.iter().map(|&tv| describe_tyvar(ctx, tv)).collect();
            format!("({}) -> {}", p.join(", "), describe_tyvar(ctx, *ret))
        },
        TyKind::Opaque { bounds, .. } => {
            if bounds.is_empty() {
                "some ?".into()
            } else {
                let bound_names: Vec<String> = bounds
                    .iter()
                    .map(|(e, _)| {
                        ctx.query_ctx
                            .get::<kestrel_ast_builder::Name>(*e)
                            .map(|n| n.0.clone())
                            .unwrap_or_else(|| format!("{:?}", e))
                    })
                    .collect();
                format!("some {}", bound_names.join(" and "))
            }
        },
        TyKind::Never => "Never".into(),
        TyKind::Error => "Error".into(),
    }
}

/// Build a human-readable error description with resolved types.
///
/// Callers must invoke this *at error-report time*, before cascade-suppression
/// poisoning rewrites any of the referenced TyVars to `TyKind::Error`. See
/// `InferCtx::report_error` for the standard entry point.
pub(crate) fn describe_error(ctx: &InferCtx<'_>, err: &InferError) -> String {
    match err {
        InferError::TypeMismatch { expected, got, .. } => {
            format!(
                "expected {} got {}",
                describe_tyvar(ctx, *expected),
                describe_tyvar(ctx, *got)
            )
        },
        InferError::DoesNotConform { ty, protocol, .. } => {
            let ty_name = describe_tyvar(ctx, *ty);
            let proto_name = ctx
                .query_ctx
                .get::<kestrel_ast_builder::Name>(*protocol)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| format!("{:?}", protocol));
            // Static failures get a "because" detail — deep in generic
            // instantiation chains the bare `T !: Static` is miserable.
            if let Some(why) = describe_static_failure(ctx, *ty, *protocol) {
                return format!("{} !: {} ({})", ty_name, proto_name, why);
            }
            // A ref TYPE ARGUMENT conforms to exactly what `extend &T: P`
            // declares — when none does, a bare `&T !: P` reads like the
            // pointee is at fault, so name the actual rule.
            if matches!(
                ctx.slot(ctx.resolve(*ty)),
                crate::ty::TySlot::Resolved(TyKind::Ref { .. })
            ) {
                return format!(
                    "{} !: {} (a reference satisfies only the protocols declared by an \
                     `extend &T:` / `extend &mutating T:` extension)",
                    ty_name, proto_name
                );
            }
            format!("{} !: {}", ty_name, proto_name)
        },
        InferError::NoMember {
            receiver,
            name,
            is_call,
            ..
        } => {
            // Wording mirrors lib1: delegating-init misses say "no method
            // 'init' on type 'T'", while all other member misses (regular
            // method calls, field/property access) say "no member 'X' on
            // type 'T'". The init wording is special-cased because lib1's
            // `resolve_delegating_init` path produced its own diagnostic.
            if name == "subscript" {
                return format!(
                    "no matching subscript on type '{}'",
                    describe_tyvar(ctx, *receiver)
                );
            }
            let kind = if *is_call && name == "init" {
                "method"
            } else {
                "member"
            };
            format!(
                "no {} '{}' on type '{}'",
                kind,
                name,
                describe_tyvar(ctx, *receiver)
            )
        },
        InferError::AmbiguousMember { receiver, name, .. } => match receiver {
            Some(recv) => format!("{}.{} ambiguous", describe_tyvar(ctx, *recv), name),
            // Receiver-less (overloaded free-function call): no receiver type to
            // name — rendering the synthetic placeholder produced "Error.f
            // ambiguous" (#210).
            None => format!("multiple candidates for '{}'", name),
        },
        InferError::MemberNotVisible { receiver, name, .. } => {
            format!("{}.{} not visible", describe_tyvar(ctx, *receiver), name)
        },
        InferError::MemberIsStatic { receiver, name, .. } => {
            format!(
                "'{}' is a static member of '{}' and cannot be used on an instance",
                name,
                describe_tyvar(ctx, *receiver)
            )
        },
        InferError::NoAssociatedType {
            container, name, ..
        } => {
            format!("{}.{} no assoc type", describe_tyvar(ctx, *container), name)
        },
        InferError::ImplicitMemberNotFound { expected, name, .. } => {
            format!(".{} not found on {}", name, describe_tyvar(ctx, *expected))
        },
        InferError::InfiniteType { .. } => "infinite type".into(),
        InferError::FromHir { .. } => "from-hir".into(),
        InferError::ArgCountMismatch { expected, got, .. } => {
            format!("expected {} argument(s), got {}", expected, got)
        },
        InferError::LabelMismatch { expected, got, .. } => {
            let exp = expected.as_deref().unwrap_or("_");
            let g = got.as_deref().unwrap_or("_");
            format!("wrong label: expected '{}', got '{}'", exp, g)
        },
        InferError::InstanceMethodAsStatic { name, .. } => {
            format!("instance method '{}' cannot be called on a type", name)
        },
        InferError::TypeParamAsValue { .. } => "type parameter cannot be used as a value".into(),
        InferError::TypeArgCountMismatch { expected, got, .. } => {
            if *got < *expected {
                format!("too few type arguments: expected {}, got {}", expected, got)
            } else {
                format!(
                    "too many type arguments: expected {}, got {}",
                    expected, got
                )
            }
        },
        InferError::NoMatchingOverload { name, .. } => {
            format!("no matching overload for '{}'", name)
        },
        InferError::MemberwiseInitArity {
            struct_name,
            expected,
            got,
            ..
        } => format!(
            "struct '{}' has {} field(s), but {} argument(s) were provided",
            struct_name, expected, got
        ),
        InferError::MemberwiseInitLabel {
            struct_name,
            expected,
            got,
            ..
        } => {
            let got_desc = got
                .as_deref()
                .map(|s| format!("'{}'", s))
                .unwrap_or_else(|| "unlabeled".into());
            format!(
                "argument for struct '{}' has {} label, but expected '{}'",
                struct_name, got_desc, expected
            )
        },
        InferError::ItWrongArity { expected, .. } => {
            format!(
                "implicit 'it' parameter used in {}-parameter context",
                expected
            )
        },
        InferError::LiteralNotAccepted { ty, literal, .. } => {
            format!(
                "type '{}' does not accept {}",
                describe_tyvar(ctx, *ty),
                literal_kind_name(*literal)
            )
        },
        InferError::UnresolvedTypeParam { param, .. } => {
            let name = ctx
                .query_ctx
                .get::<kestrel_ast_builder::Name>(*param)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| "_".into());
            format!("cannot infer type parameter '{}'", name)
        },
        InferError::CannotInferType { .. } => "could not infer type".into(),
        InferError::TupleIndexOnNonTuple {
            receiver, index, ..
        } => format!(
            "cannot index into non-tuple type '{}' with .{}",
            describe_tyvar(ctx, *receiver),
            index
        ),
        InferError::TupleIndexOutOfBounds { arity, index, .. } => format!(
            "tuple index {} out of bounds for {}-element tuple",
            index, arity
        ),
        InferError::MemberAccessOnPrimitive { receiver, name, .. } => format!(
            "cannot access member '{}' on type '{}'",
            name,
            describe_tyvar(ctx, *receiver)
        ),
        InferError::MethodNotCalled {
            receiver, method, ..
        } => format!(
            "method '{}' on '{}' must be called",
            method,
            describe_tyvar(ctx, *receiver)
        ),
        InferError::CircularOpaqueReturn { .. } => "circular opaque return type".into(),
        InferError::RefFunctionAsValue { .. } => {
            "a reference-returning function is not a value".into()
        },
        InferError::RefInTypeArgument { .. } => {
            "a reference cannot be a generic type argument".into()
        },
        InferError::ConventionMismatch { .. } => {
            "cannot pass a mutating closure where a non-mutating parameter is expected".into()
        },
    }
}

/// "Because" detail for a failed `Static` bound (references 2a): names the
/// declaration, the offending stored member, or the offending type argument.
/// `None` for non-Static protocols or shapes with nothing specific to say.
fn describe_static_failure(ctx: &InferCtx<'_>, ty: TyVar, protocol: Entity) -> Option<String> {
    use kestrel_semantics::{NominalStaticness, Staticness, StaticnessReason};

    if !crate::solver::is_static_builtin(ctx, protocol) {
        return None;
    }
    let resolved = ctx.resolve(ty);
    let TySlot::Resolved(kind) = ctx.slot(resolved) else {
        return None;
    };
    match kind.clone() {
        TyKind::Ref { .. } => Some("a reference is never Static".into()),
        TyKind::Param { .. } => {
            Some("the type parameter is relaxed with 'not Static', so it may hold references".into())
        },
        TyKind::Struct { entity, args } | TyKind::Enum { entity, args } => {
            let info = ctx.query_ctx.query(NominalStaticness {
                entity,
                root: ctx.root,
            });
            match info.reason {
                StaticnessReason::DeclaredNotStatic => Some("declared 'not Static'".into()),
                StaticnessReason::NonStaticChild(child) => {
                    let word = match ctx.query_ctx.get::<kestrel_ast_builder::NodeKind>(child) {
                        Some(kestrel_ast_builder::NodeKind::EnumCase) => "case",
                        _ => "field",
                    };
                    let name = ctx
                        .query_ctx
                        .get::<kestrel_ast_builder::Name>(child)
                        .map(|n| n.0.clone())
                        .unwrap_or_else(|| "<unnamed>".into());
                    Some(format!("{} '{}' is non-Static", word, name))
                },
                StaticnessReason::Default => {
                    // Conditional base: the failure came from a gating arg —
                    // re-fold to name the first offender.
                    let Staticness::ConditionalOn(positions) = info.staticness else {
                        return None;
                    };
                    let bad = positions
                        .iter()
                        .filter_map(|&i| args.get(i))
                        .find(|&&arg| !crate::solver::solver_ty_is_static(ctx, arg, 0))?;
                    Some(format!(
                        "type argument '{}' is non-Static",
                        describe_tyvar(ctx, *bad)
                    ))
                },
            }
        },
        // Element-wise tuple fold: name the first non-Static element so
        // `Array[(&T, Int64)]` failures say WHY.
        TyKind::Tuple(elems) => {
            let bad = elems
                .iter()
                .find(|&&e| !crate::solver::solver_ty_is_static(ctx, e, 0))?;
            Some(format!(
                "tuple element '{}' is non-Static",
                describe_tyvar(ctx, *bad)
            ))
        },
        _ => None,
    }
}
