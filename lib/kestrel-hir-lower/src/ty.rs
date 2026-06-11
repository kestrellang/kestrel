//! Type lowering: AstType → HirTy.
//!
//! Resolves type paths to entities and expands sugar types
//! (Array, Optional, Dictionary, Result) into Struct types.
//!
//! The standalone `lower_ast_type` function is the shared implementation
//! used both during body lowering (via LowerCtx) and by type inference
//! for declaration-level types (Callable params, TypeAnnotation, etc.).

use kestrel_ast::AstType;
use kestrel_ast_builder::{
    Callable, Computed, DeclSpan, ExtensionTarget, Gettable, MutatingAccessor, NodeKind, Settable,
    TypeAnnotation, TypeParams,
};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::ty::HirTy;
use kestrel_name_res::{ExtensionTargetEntity, ResolveTypePath, TypeResolution};
use kestrel_reporting::{Diagnostic, Label};
use kestrel_span::Span;

use crate::ctx::LowerCtx;

// ===== Standalone type lowering =====

/// Lower an AstType to HirTy using name resolution.
///
/// This is the shared core used by both body lowering (LowerCtx::lower_type)
/// and declaration-level type queries (LowerTypeAnnotation, LowerCallableTypes).
pub fn lower_ast_type(ctx: &QueryContext<'_>, owner: Entity, root: Entity, ty: &AstType) -> HirTy {
    match ty {
        AstType::Named { segments, span } => {
            let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
            let result = ctx.query(ResolveTypePath {
                segments: seg_names,
                context: owner,
                root,
            });

            match result {
                TypeResolution::Found(entity) => {
                    build_hir_ty_for_entity(ctx, owner, root, entity, segments, span)
                },
                TypeResolution::SelfType => {
                    if let Some(ty) = lower_self_hir_ty(ctx, owner, root, span) {
                        ty
                    } else {
                        ctx.accumulate(
                            Diagnostic::error()
                                .with_message("'Self' is not valid in this scope")
                                .with_labels(vec![Label::primary(span.file_id, span.range())]),
                        );
                        HirTy::Error(span.clone())
                    }
                },
                TypeResolution::NotFound(ref seg) => {
                    let type_name = segments
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join(".");
                    ctx.accumulate(
                        Diagnostic::error()
                            .with_message(format!("cannot find type '{type_name}' in this scope"))
                            .with_labels(vec![
                                Label::primary(span.file_id, span.range())
                                    .with_message(format!("not found (failed at '{seg}')")),
                            ]),
                    );
                    HirTy::Error(span.clone())
                },
                TypeResolution::NotAType(entity) => {
                    let type_name = segments
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join(".");
                    let mut labels = vec![
                        Label::primary(span.file_id, span.range()).with_message("expected a type"),
                    ];
                    if let Some(decl) = ctx.get::<DeclSpan>(entity) {
                        let kind = ctx
                            .get::<NodeKind>(entity)
                            .map(|k| format!("{k:?}"))
                            .unwrap_or_else(|| "symbol".to_string());
                        labels.push(
                            Label::secondary(decl.0.file_id, decl.0.range())
                                .with_message(format!("'{type_name}' is a {kind}, not a type")),
                        );
                    }
                    ctx.accumulate(
                        Diagnostic::error()
                            .with_message(format!("'{type_name}' is not a type"))
                            .with_labels(labels),
                    );
                    HirTy::Error(span.clone())
                },
            }
        },

        AstType::Tuple(types, span) => {
            let lowered: Vec<HirTy> = types
                .iter()
                .map(|t| lower_ast_type(ctx, owner, root, t))
                .collect();
            HirTy::Tuple(lowered, span.clone())
        },

        AstType::Function {
            params,
            param_conventions,
            return_type,
            span,
        } => {
            let lowered_params: Vec<HirTy> = params
                .iter()
                .map(|t| lower_ast_type(ctx, owner, root, t))
                .collect();
            let lowered_ret = Box::new(lower_ast_type(ctx, owner, root, return_type));
            HirTy::Function {
                params: lowered_params,
                param_conventions: param_conventions.clone(),
                ret: lowered_ret,
                span: span.clone(),
            }
        },

        // Sugar types → resolve standard library entity + Struct
        AstType::Array(elem, span) => {
            lower_sugar_type(ctx, owner, root, "Array", &[elem.as_ref()], span)
        },
        AstType::Optional(inner, span) => {
            lower_sugar_type(ctx, owner, root, "Optional", &[inner.as_ref()], span)
        },
        AstType::Dictionary(key, val, span) => lower_sugar_type(
            ctx,
            owner,
            root,
            "Dictionary",
            &[key.as_ref(), val.as_ref()],
            span,
        ),
        AstType::Result { ok, err, span } => lower_sugar_type(
            ctx,
            owner,
            root,
            "Result",
            &[ok.as_ref(), err.as_ref()],
            span,
        ),
        AstType::Unit(span) => HirTy::Tuple(Vec::new(), span.clone()),
        AstType::Never(span) => HirTy::Never(span.clone()),
        AstType::Inferred(span) => HirTy::Infer(span.clone()),
        AstType::Some { bounds, span } => {
            let hir_bounds: Vec<HirTy> = bounds
                .iter()
                .map(|b| lower_ast_type(ctx, owner, root, b))
                .collect();
            HirTy::Opaque {
                bounds: hir_bounds,
                span: span.clone(),
            }
        },

        // Lowered structurally so the pointee still gets name resolution
        // (and its diagnostics); the Ref itself is rejected and rewritten to
        // Error by `reject_ref_types` at every lowering-query boundary.
        AstType::Ref {
            inner,
            mutating,
            span,
        } => HirTy::Ref {
            inner: Box::new(lower_ast_type(ctx, owner, root, inner)),
            mutating: *mutating,
            span: span.clone(),
        },
    }
}

/// Build a HirTy for a resolved entity, dispatching on NodeKind.
///
/// - `TypeParameter` → `HirTy::Param`
/// - `TypeAlias` whose parent is a Protocol → `HirTy::AssocProjection` (abstract associated type)
/// - `TypeAlias` otherwise → `HirTy::AliasUse` (regular alias; inference reduces)
/// - `Struct` / `Enum` / `Protocol` → corresponding variant
///
/// Shared between the Found and SelfType resolution branches.
fn build_hir_ty_for_entity(
    ctx: &QueryContext<'_>,
    owner: Entity,
    root: Entity,
    entity: Entity,
    segments: &[kestrel_ast::PathSegment],
    span: &Span,
) -> HirTy {
    let kind = ctx.get::<NodeKind>(entity).cloned();

    // Type parameter: no args, no projection.
    if kind == Some(NodeKind::TypeParameter) {
        return HirTy::Param(entity, span.clone());
    }

    // Lower type arguments from all segments. Applies to Struct/Enum/Protocol/AliasUse.
    let args: Vec<HirTy> = segments
        .iter()
        .flat_map(|s| s.type_args.iter())
        .map(|a| lower_ast_type(ctx, owner, root, a))
        .collect();

    // Validate arity for non-alias, non-associated entities.
    let args = match validate_arity(ctx, entity, args, segments, span) {
        Ok(args) => args,
        Err(err) => return err,
    };
    let args = fill_type_arg_defaults(ctx, root, entity, args);

    match kind {
        Some(NodeKind::TypeAlias) => {
            // An associated type (TypeAlias whose parent is a Protocol) must
            // carry its base so the solver can project it through the concrete
            // receiver. This stays true even when the associated type has a
            // default annotation: conforming types may override that default,
            // so uses in protocol requirements must remain projections.
            let parent_is_protocol = ctx
                .parent_of(entity)
                .and_then(|p| ctx.get::<NodeKind>(p).cloned())
                == Some(NodeKind::Protocol);
            if parent_is_protocol {
                let base = build_assoc_projection_base(ctx, owner, root, segments, span);
                return HirTy::AssocProjection {
                    base: Box::new(base),
                    assoc: entity,
                    span: span.clone(),
                };
            }
            // Trivial (non-generic, bound-free) aliases with a concrete
            // TypeAnnotation are eagerly expanded — avoids constraint bloat
            // for `type Fd = Int32` style declarations.
            if is_trivial_alias(ctx, entity)
                && args.is_empty()
                && let Some(ann) = ctx.get::<TypeAnnotation>(entity)
            {
                return lower_ast_type(ctx, owner, root, &ann.0);
            }
            // Non-associated aliases (parameterized or constrained) flow as
            // AliasUse. The solver reduces concrete ones via Reduce.
            HirTy::AliasUse {
                entity,
                args,
                span: span.clone(),
            }
        },
        Some(NodeKind::Enum) => HirTy::Enum {
            entity,
            args,
            span: span.clone(),
        },
        Some(NodeKind::Protocol) => HirTy::Protocol {
            entity,
            args,
            span: span.clone(),
        },
        // Struct is the default for Typed entities without a more specific kind
        // (covers Struct, Module-owned foreign types, lang.* intrinsics, etc.).
        _ => HirTy::Struct {
            entity,
            args,
            span: span.clone(),
        },
    }
}

/// A type-naming RHS that may itself BE a ref (stage 2d): an assoc-type
/// binding (`type Item = &T`) or a where-clause equality RHS
/// (`where I.Item = &Int64`). Keeps a top-level ref (mirror of the
/// function-return carve) and walks everything else with the aggregate
/// policy — bare refs in nested non-aggregate positions still reject.
pub fn reject_ref_types_allowing_top_ref(ctx: &QueryContext<'_>, ty: HirTy) -> HirTy {
    if let HirTy::Ref {
        inner,
        mutating,
        span,
    } = ty
    {
        let inner = reject_ref_types(ctx, *inner, RefPosition::Other, RefPolicy::AllowAggregate);
        return HirTy::Ref {
            inner: Box::new(inner),
            mutating,
            span,
        };
    }
    reject_ref_types(ctx, ty, RefPosition::Other, RefPolicy::AllowAggregate)
}

/// True if an alias entity is trivial — has no type params, no protocol bounds,
/// no where clause. These aliases can be safely expanded at HIR lowering.
fn is_trivial_alias(ctx: &QueryContext<'_>, entity: Entity) -> bool {
    let has_type_params = ctx
        .get::<TypeParams>(entity)
        .map(|tp| !tp.0.is_empty())
        .unwrap_or(false);
    if has_type_params {
        return false;
    }
    if ctx
        .get::<kestrel_ast_builder::Conformances>(entity)
        .is_some()
    {
        return false;
    }
    if ctx
        .get::<kestrel_ast_builder::WhereClause>(entity)
        .is_some()
    {
        return false;
    }
    true
}

/// Build the `base` of an AssocProjection when we've resolved a path to an
/// associated type (TypeAlias inside a Protocol).
///
/// - Multi-segment (`T.Item`, `Self.Item`): lower segments[..last] as a type path.
/// - Single-segment (`Item` used bare inside the owning protocol): base = Self.
fn build_assoc_projection_base(
    ctx: &QueryContext<'_>,
    owner: Entity,
    root: Entity,
    segments: &[kestrel_ast::PathSegment],
    span: &Span,
) -> HirTy {
    if segments.len() >= 2 {
        // Lower segments[..last] as a standalone path.
        let prefix = AstType::Named {
            segments: segments[..segments.len() - 1].to_vec(),
            span: span.clone(),
        };
        lower_ast_type(ctx, owner, root, &prefix)
    } else {
        // Bare `Item` inside the protocol — base is Self.
        lower_self_hir_ty(ctx, owner, root, span).unwrap_or_else(|| HirTy::Error(span.clone()))
    }
}

/// Lower `Self` (in type position) to a fully-parameterized `HirTy`.
///
/// Walks up from `owner` to the nearest Struct/Enum/Protocol or Extension.
/// Returns `None` only if `Self` appears outside any type/extension/protocol
/// scope — the caller emits the diagnostic.
///
/// Critically, the resulting `HirTy` carries the *right* type args:
///   - `extend Box[lang.i64] { ... Self ... }` → `Box[i64]`
///   - `extend Box[T] { ... Self ... }`        → `Box[T]` (T = extension's type param)
///   - `struct Box[T] { ... Self ... }`        → `Box[T]` (T = struct's type param)
///   - `protocol P { ... Self ... }`           → `HirTy::SelfType(P)` (abstract; witness substitutes)
///
/// Without these args, `-> Self` in a generic-target extension lowers to the
/// bare entity (e.g. `Box`) and unifies as the unparameterized type, producing
/// "expected Box got Box[i64]" at call sites.
fn lower_self_hir_ty(
    ctx: &QueryContext<'_>,
    owner: Entity,
    root: Entity,
    span: &Span,
) -> Option<HirTy> {
    let mut current = Some(owner);
    while let Some(entity) = current {
        match ctx.get::<NodeKind>(entity).cloned() {
            Some(NodeKind::Extension) => {
                // Lower the extension's target AstType directly — it already
                // carries any type args from the source (e.g. `Box[lang.i64]`
                // or `Box[T]`). Resolve in the extension's own scope so its
                // type parameters are visible.
                let target = ctx.get::<ExtensionTarget>(entity)?;
                let target_ast = target.0.clone();
                let mut ty = lower_ast_type(ctx, entity, root, &target_ast);
                // Override the span so diagnostics point at the `Self` use site
                // rather than the extension header.
                override_span(&mut ty, span);
                // Protocol target: emit SelfType, not Protocol — Self in a
                // protocol-extension body must remain abstract for witness
                // substitution.
                if let HirTy::Protocol { entity: p, .. } = ty {
                    return Some(HirTy::SelfType(p, span.clone()));
                }
                return Some(ty);
            },
            Some(NodeKind::Struct) => {
                return Some(HirTy::Struct {
                    entity,
                    args: self_param_args(ctx, entity, span),
                    span: span.clone(),
                });
            },
            Some(NodeKind::Enum) => {
                return Some(HirTy::Enum {
                    entity,
                    args: self_param_args(ctx, entity, span),
                    span: span.clone(),
                });
            },
            Some(NodeKind::Protocol) => {
                return Some(HirTy::SelfType(entity, span.clone()));
            },
            _ => current = ctx.parent_of(entity),
        }
    }
    None
}

/// Type-param entities of `entity` lifted to `HirTy::Param` so that
/// `Self` inside a generic struct/enum body refers to the parameterized
/// shape (`Box[T]`), not the bare entity.
fn self_param_args(ctx: &QueryContext<'_>, entity: Entity, span: &Span) -> Vec<HirTy> {
    ctx.get::<TypeParams>(entity)
        .map(|tp| {
            tp.0.iter()
                .map(|&p| HirTy::Param(p, span.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// Replace the top-level span of `ty` with `span`. Used when reusing a
/// lowered AstType (e.g. an extension target) as the body of `Self`.
fn override_span(ty: &mut HirTy, span: &Span) {
    match ty {
        HirTy::Struct { span: s, .. }
        | HirTy::Enum { span: s, .. }
        | HirTy::Protocol { span: s, .. }
        | HirTy::AliasUse { span: s, .. }
        | HirTy::Tuple(_, s)
        | HirTy::Function { span: s, .. }
        | HirTy::Param(_, s)
        | HirTy::SelfType(_, s)
        | HirTy::AssocProjection { span: s, .. }
        | HirTy::Error(s)
        | HirTy::Infer(s)
        | HirTy::Never(s)
        | HirTy::Opaque { span: s, .. }
        | HirTy::Ref { span: s, .. } => *s = span.clone(),
    }
}

// ===== Reference-type rejection (stage 0.5) =====

/// The type position a reference type occurs in — reported if the type
/// handed to `reject_ref_types` is itself a `Ref`. Callers pass the
/// outermost position; the walk sets nested positions (tuple element,
/// generic arg, function-type param/return) structurally as it descends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefPosition {
    /// Parameter position — the one PERMANENT rejection (references-gaps.md
    /// §10.6): conventions are the only parameter spelling, forever.
    Param,
    /// Return position — carved out (made legal) in stage 1.
    Return,
    /// `var`/`let` annotation.
    Binding,
    /// Struct/enum field (incl. enum case payload).
    Field,
    /// Tuple element (set by the walk).
    TupleElement,
    /// Generic type argument (`Array[&T]`; set by the walk).
    GenericArg,
    /// Function-type return (`() -> &T`; set by the walk).
    FunctionReturn,
    /// Anything else (alias RHS, where-clause, protocol bound, ...).
    Other,
}

impl RefPosition {
    fn code_and_message(self) -> (&'static str, &'static str) {
        match self {
            // Permanent rule — worded as "are not", never "not yet".
            RefPosition::Param => (
                "E480",
                "parameters are not reference-typed; spell the convention instead — `x: T` borrows, `mutating x: T` mutably borrows",
            ),
            RefPosition::Return => ("E481", "reference return types are not supported yet"),
            RefPosition::Binding => ("E482", "references cannot be stored in bindings"),
            RefPosition::Field => ("E483", "references cannot be stored in fields"),
            RefPosition::TupleElement => ("E484", "references cannot be stored in tuples"),
            RefPosition::GenericArg => ("E485", "references cannot be used as type arguments"),
            RefPosition::FunctionReturn => (
                "E486",
                "reference returns are not supported in function types yet",
            ),
            RefPosition::Other => ("E489", "reference types cannot be used here"),
        }
    }
}

/// Whether the walk ACCEPTS refs at aggregate positions (stage 2b) or
/// rejects every position (stage 0.5). The policy is per ENTRY POINT, not
/// per position: a Strict entry (alias RHS, assoc-type default, protocol
/// bound, extension target, where-clause type) rejects refs even inside
/// nested type args, because those whole annotations feed machinery that
/// has no ref story yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefPolicy {
    Strict,
    /// Refs are legal at {Field, TupleElement, GenericArg} positions —
    /// enum payloads route as Field. Bare-position rules (E480 param,
    /// E482 binding, E486 fn-type return, E487 nesting, E489 other) and
    /// the protocol-arg rejection still hold.
    AllowAggregate,
}

/// THE type-position walk (stage 0.5): rejects `HirTy::Ref` with a
/// position-specific diagnostic and rewrites it to `HirTy::Error`. Called
/// at every lowering-query boundary. Stage 1 carved out only
/// `RefPosition::Return`; stage 2b accepts aggregate positions under
/// `RefPolicy::AllowAggregate`.
pub fn reject_ref_types(
    ctx: &QueryContext<'_>,
    ty: HirTy,
    pos: RefPosition,
    policy: RefPolicy,
) -> HirTy {
    match ty {
        HirTy::Ref {
            inner,
            mutating,
            span,
        } => {
            let nested = matches!(*inner, HirTy::Ref { .. });
            // Stage 2b carve: a ref in an aggregate slot is an honest type.
            // Nesting (`& &T`) stays banned; the pointee subtree is still
            // walked (its own aggregate args may carry refs; its bare
            // positions still reject).
            if !nested
                && policy == RefPolicy::AllowAggregate
                && matches!(
                    pos,
                    RefPosition::Field | RefPosition::TupleElement | RefPosition::GenericArg
                )
            {
                let inner = reject_ref_types(ctx, *inner, RefPosition::Other, policy);
                return HirTy::Ref {
                    inner: Box::new(inner),
                    mutating,
                    span,
                };
            }
            let (code, message) = if nested {
                // `&&T` / `&mutating &T`: one diagnostic for the whole
                // cluster — fixing the nesting then surfaces the positional
                // error, if any (E-REF-08).
                ("E487", "a reference cannot reference a reference")
            } else {
                pos.code_and_message()
            };
            ctx.accumulate(
                Diagnostic::error()
                    .with_code(code)
                    .with_message(message)
                    .with_labels(vec![Label::primary(span.file_id, span.range())]),
            );
            // Peel any nested refs without further positional noise, then
            // keep walking the pointee so deeper refs still get diagnosed.
            let mut core = *inner;
            while let HirTy::Ref { inner, .. } = core {
                core = *inner;
            }
            reject_ref_types(ctx, core, RefPosition::Other, policy);
            HirTy::Error(span)
        },
        HirTy::Struct { entity, args, span } => HirTy::Struct {
            entity,
            args: reject_ref_args(ctx, args, policy),
            span,
        },
        HirTy::Enum { entity, args, span } => HirTy::Enum {
            entity,
            args: reject_ref_args(ctx, args, policy),
            span,
        },
        // Protocol type args stay Strict regardless of policy: a ref
        // protocol arg instantiates witnesses at the ref (2d, not 2b).
        HirTy::Protocol { entity, args, span } => HirTy::Protocol {
            entity,
            args: reject_ref_args(ctx, args, RefPolicy::Strict),
            span,
        },
        // Alias args stay Strict too: trivial aliases expand at the use
        // site BEFORE this walk, so a ref smuggled through an alias arg
        // would bypass the positional rules (the R5 smuggling class).
        HirTy::AliasUse { entity, args, span } => HirTy::AliasUse {
            entity,
            args: reject_ref_args(ctx, args, RefPolicy::Strict),
            span,
        },
        HirTy::Tuple(elems, span) => HirTy::Tuple(
            elems
                .into_iter()
                .map(|e| reject_ref_types(ctx, e, RefPosition::TupleElement, policy))
                .collect(),
            span,
        ),
        HirTy::Function {
            params,
            param_conventions,
            ret,
            span,
        } => HirTy::Function {
            params: params
                .into_iter()
                .map(|p| reject_ref_types(ctx, p, RefPosition::Param, policy))
                .collect(),
            param_conventions,
            ret: Box::new(reject_ref_types(
                ctx,
                *ret,
                RefPosition::FunctionReturn,
                policy,
            )),
            span,
        },
        HirTy::AssocProjection { base, assoc, span } => HirTy::AssocProjection {
            base: Box::new(reject_ref_types(ctx, *base, RefPosition::Other, policy)),
            assoc,
            span,
        },
        HirTy::Opaque { bounds, span } => HirTy::Opaque {
            bounds: bounds
                .into_iter()
                .map(|b| reject_ref_types(ctx, b, RefPosition::Other, RefPolicy::Strict))
                .collect(),
            span,
        },
        leaf @ (HirTy::Param(..)
        | HirTy::SelfType(..)
        | HirTy::Never(_)
        | HirTy::Infer(_)
        | HirTy::Error(_)) => leaf,
    }
}

fn reject_ref_args(ctx: &QueryContext<'_>, args: Vec<HirTy>, policy: RefPolicy) -> Vec<HirTy> {
    args.into_iter()
        .map(|a| reject_ref_types(ctx, a, RefPosition::GenericArg, policy))
        .collect()
}

/// Validate that the number of type arguments matches the entity's TypeParams.
/// Returns `Err(HirTy::Error)` with an accumulated diagnostic on mismatch.
fn validate_arity(
    ctx: &QueryContext<'_>,
    entity: Entity,
    args: Vec<HirTy>,
    segments: &[kestrel_ast::PathSegment],
    span: &Span,
) -> Result<Vec<HirTy>, HirTy> {
    if args.is_empty() {
        return Ok(args);
    }
    let Some(tp) = ctx.get::<TypeParams>(entity) else {
        return Ok(args);
    };
    let total = tp.0.len();
    let required =
        tp.0.iter()
            .filter(|&&p| ctx.get::<TypeAnnotation>(p).is_none())
            .count();
    if args.len() < required {
        let type_name = segments
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(".");
        ctx.accumulate(
            Diagnostic::error()
                .with_message(format!("too few type arguments for '{type_name}'"))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message(format!("expected at least {required}, got {}", args.len())),
                ]),
        );
        Err(HirTy::Error(span.clone()))
    } else if args.len() > total {
        let type_name = segments
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(".");
        ctx.accumulate(
            Diagnostic::error()
                .with_message(format!("too many type arguments for '{type_name}'"))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message(format!("expected at most {total}, got {}", args.len())),
                ]),
        );
        Err(HirTy::Error(span.clone()))
    } else {
        Ok(args)
    }
}

/// Lower a sugar type (Array, Optional, etc.) by resolving the std type entity.
fn lower_sugar_type(
    ctx: &QueryContext<'_>,
    owner: Entity,
    root: Entity,
    name: &str,
    type_args: &[&AstType],
    span: &Span,
) -> HirTy {
    let lowered_args: Vec<HirTy> = type_args
        .iter()
        .map(|t| lower_ast_type(ctx, owner, root, t))
        .collect();

    if let Some(entity) = resolve_std_type(ctx, owner, root, name) {
        let args = fill_type_arg_defaults(ctx, root, entity, lowered_args);
        // Dispatch by NodeKind — Optional is an enum, Array/Dictionary are structs.
        match ctx.get::<NodeKind>(entity).cloned() {
            Some(NodeKind::Enum) => HirTy::Enum {
                entity,
                args,
                span: span.clone(),
            },
            Some(NodeKind::Protocol) => HirTy::Protocol {
                entity,
                args,
                span: span.clone(),
            },
            _ => HirTy::Struct {
                entity,
                args,
                span: span.clone(),
            },
        }
    } else {
        ctx.accumulate(
            Diagnostic::error()
                .with_message(format!("{name} is not defined"))
                .with_labels(vec![Label::primary(span.file_id, span.range())])
                .with_notes(vec!["is the standard library imported?".to_string()]),
        );
        HirTy::Error(span.clone())
    }
}

/// Fill in default type arguments for type parameters beyond user-provided args.
/// Defaults are stored as `TypeAnnotation` on the type param entity and lowered in
/// the defining scope so e.g. `H = DefaultHasher` resolves in the declaring module.
/// Stops on the first type param without a default (defaults must be trailing).
fn fill_type_arg_defaults(
    ctx: &QueryContext<'_>,
    root: Entity,
    entity: Entity,
    mut args: Vec<HirTy>,
) -> Vec<HirTy> {
    let Some(tp) = ctx.get::<TypeParams>(entity) else {
        return args;
    };
    if args.len() >= tp.0.len() {
        return args;
    }
    let type_params = tp.0.clone();
    for &param in type_params.iter().skip(args.len()) {
        match ctx.query(LowerTypeAnnotation {
            entity: param,
            root,
        }) {
            Some(default_ty) => args.push(default_ty),
            None => break,
        }
    }
    args
}

/// Resolve a well-known standard library type name (e.g. "Array", "Optional").
fn resolve_std_type(
    ctx: &QueryContext<'_>,
    owner: Entity,
    root: Entity,
    name: &str,
) -> Option<Entity> {
    let result = ctx.query(ResolveTypePath {
        segments: vec![name.to_string()],
        context: owner,
        root,
    });
    match result {
        TypeResolution::Found(entity) => Some(entity),
        _ => None,
    }
}

// ===== LowerCtx delegation =====

impl LowerCtx<'_> {
    /// Lower an AST type to an HIR type. Delegates to the standalone function.
    /// In-body types here are overwhelmingly expression type arguments
    /// (`make[T]()`), so refs default to the generic-arg rejection; binding
    /// annotations and closure params use `lower_type_in` instead.
    pub fn lower_type(&mut self, ty: &AstType) -> HirTy {
        self.lower_type_in(ty, RefPosition::GenericArg)
    }

    /// Lower an AST type, classifying a top-level `&T` by `pos`. In-body
    /// positions accept aggregate refs (stage 2b): explicit `f[&T]` type
    /// args and `let o: Optional[&T]` annotations are legal; the bare
    /// positions (E482 binding, E480 closure param) still reject by `pos`.
    pub fn lower_type_in(&mut self, ty: &AstType, pos: RefPosition) -> HirTy {
        let lowered = lower_ast_type(self.ctx, self.owner, self.root, ty);
        reject_ref_types(self.ctx, lowered, pos, RefPolicy::AllowAggregate)
    }
}

// ===== Queries =====

/// Query: lower a declaration entity's TypeAnnotation to HirTy.
///
/// Reads the `TypeAnnotation` component and resolves the AstType
/// to an HirTy using name resolution in the entity's scope.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LowerTypeAnnotation {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for LowerTypeAnnotation {
    type Output = Option<HirTy>;

    fn describe(&self) -> String {
        format!("LowerTypeAnnotation(entity={:?})", self.entity)
    }

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<HirTy> {
        let type_ann = ctx.get::<TypeAnnotation>(self.entity)?;
        let nk = ctx.get::<NodeKind>(self.entity);

        // E490: `throws` desugars the return to `Result` sugar — a ref can
        // never ride an effect wrapper (the ret_borrow ABI is not
        // expressible in an enum payload). Two shapes: `Result{ok: Ref}`
        // (init effect) and `Ref{inner: Result}` (`-> &T throws E` — the
        // `&` prefix binds looser than the throws tail, wrapping the
        // sugar). Checked on the AST so the wording names the sugar, not a
        // generic nested-arg error. An explicit `&Result[T, E]` annotation
        // spells the generic by NAME and is untouched.
        let throws_ref = match &type_ann.0 {
            AstType::Result { ok, span, .. } if matches!(ok.as_ref(), AstType::Ref { .. }) => {
                Some(span)
            },
            AstType::Ref { inner, span, .. }
                if matches!(inner.as_ref(), AstType::Result { .. }) =>
            {
                Some(span)
            },
            _ => None,
        };
        if matches!(nk, Some(NodeKind::Function))
            && let Some(span) = throws_ref
        {
            ctx.accumulate(
                Diagnostic::error()
                    .with_code("E490")
                    .with_message("a throwing function cannot return a reference")
                    .with_labels(vec![Label::primary(span.file_id, span.range())
                        .with_message("`throws` wraps the return in `Result` — a reference \
                                       cannot live in an enum payload")]),
            );
            return Some(HirTy::Error(span.clone()));
        }

        let lowered = lower_ast_type(ctx, self.entity, self.root, &type_ann.0);

        // E481 carve-out (stage 1): a ref RETURN is legal for functions and
        // computed-property GETTERS (the Pointer-bridge `var value: &T`
        // shape — Field + Computed + Gettable, NOT Settable: a settable
        // property pairs with a setter, which is the call-as-place world of
        // stage 1.5). Subscripts stay rejected for the same reason;
        // init/deinit never return. The pointee subtree is still walked
        // (E487 nesting, E480 fn-type params, ...).
        // Stage 1.5: RefAccessor entities carry the SYNTHESIZED `&T` /
        // `&mutating T` return built by `spawn_ref_accessor` — always legal
        // (the user-facing rule is unchanged: a ref type never appears in a
        // subscript/property SOURCE signature).
        let is_computed_getter = matches!(nk, Some(NodeKind::Field))
            && ctx.get::<Computed>(self.entity).is_some()
            && ctx.get::<Gettable>(self.entity).is_some()
            && ctx.get::<Settable>(self.entity).is_none();
        if (matches!(nk, Some(NodeKind::Function | NodeKind::RefAccessor)) || is_computed_getter)
            && let HirTy::Ref {
                inner,
                mutating,
                span,
            } = lowered
        {
            let inner = reject_ref_types(ctx, *inner, RefPosition::Other, RefPolicy::AllowAggregate);
            return Some(HirTy::Ref {
                inner: Box::new(inner),
                mutating,
                span,
            });
        }

        // Stage 2d: an ASSOC-TYPE BINDING (`type Item = &T` in a struct/
        // enum/extension conformance) may be a bare ref or carry refs in
        // aggregate positions — ref-Item iterators hang off this. The carve
        // is restricted to TRIVIAL member aliases: those are eagerly
        // expanded at every named use (`let x: Foo.Item` re-applies the
        // use-site position rules, E482/E480 — nothing smuggles), while
        // non-trivial aliases flow as AliasUse through solver Reduce with
        // no position re-check, so they stay Strict. Protocol-parented
        // aliases (assoc DECLS and their defaults) also stay Strict.
        let parent_kind = ctx
            .parent_of(self.entity)
            .and_then(|p| ctx.get::<NodeKind>(p).cloned());
        if matches!(nk, Some(NodeKind::TypeAlias))
            && matches!(
                parent_kind,
                Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Extension)
            )
            && is_trivial_alias(ctx, self.entity)
        {
            return Some(reject_ref_types_allowing_top_ref(ctx, lowered));
        }

        // A TypeAnnotation on a callable is its return annotation; on a
        // field/enum-case it is the stored type. Classify the ref rejection
        // accordingly (stage 1 carved out only the Return position, above;
        // stage 2b accepts aggregate positions everywhere EXCEPT alias
        // RHS / other annotations, which stay Strict).
        let (pos, policy) = match nk {
            Some(
                NodeKind::Function | NodeKind::Initializer | NodeKind::Subscript | NodeKind::Deinit,
            ) => (RefPosition::Return, RefPolicy::AllowAggregate),
            Some(NodeKind::Field | NodeKind::EnumCase) => {
                (RefPosition::Field, RefPolicy::AllowAggregate)
            },
            Some(NodeKind::TypeParameter) => (RefPosition::GenericArg, RefPolicy::AllowAggregate),
            _ => (RefPosition::Other, RefPolicy::Strict),
        };
        Some(reject_ref_types(ctx, lowered, pos, policy))
    }
}

/// Query: lower a callable entity's declared return type.
///
/// Applies the language rule so every consumer gets the same answer:
/// if the callable has an explicit `-> T` (`TypeAnnotation`), lower it;
/// otherwise default to unit `()`. This replaces ad-hoc `unwrap_or` chains
/// that previously each picked their own fallback (fresh TyVar, unit, or
/// Error) at different call sites.
///
/// Initializers have no annotation and are treated as returning unit here.
/// Their "actual" return is the parent struct, but every consumer of init
/// calls (solve_member, emit_call_maybe_init) wires up the result via a
/// dedicated `self` slot, so the declared return is immaterial — making it
/// unit matches that convention and keeps the query total.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LowerCallableReturnType {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for LowerCallableReturnType {
    type Output = HirTy;

    fn describe(&self) -> String {
        format!("LowerCallableReturnType(entity={:?})", self.entity)
    }

    fn execute(&self, ctx: &QueryContext<'_>) -> HirTy {
        ctx.query(LowerTypeAnnotation {
            entity: self.entity,
            root: self.root,
        })
        .unwrap_or_else(|| HirTy::Tuple(Vec::new(), Span::synthetic(0)))
    }
}

/// A callable's reference-return convention (`-> &T` / `-> &mutating T`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RefReturn {
    pub mutating: bool,
}

/// Query: does this callable return a reference (the `ret_borrow` ABI)?
///
/// THE single front-end derivation of the return convention (the MIR twin is
/// `kestrel_mir::item::function::ret_convention`). Consumers — the solver's
/// ref-returning-function-as-value check, analyze's mutability classifier and
/// ambiguous-root rule, MIR signature lowering — ask this query; nobody
/// re-matches `HirTy::Ref` ad hoc. CONTRACT: this (keyed by the resolved
/// callee entity) is the ground truth for "is this call a place"; an
/// expression's recorded `&T` type is a sufficient-but-not-necessary signal
/// (solver ordering can pin a call-result var to the pointee type first).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CallableRefReturn {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for CallableRefReturn {
    type Output = Option<RefReturn>;

    fn describe(&self) -> String {
        format!("CallableRefReturn(entity={:?})", self.entity)
    }

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<RefReturn> {
        let ret = ctx.query(LowerCallableReturnType {
            entity: self.entity,
            root: self.root,
        });
        match ret {
            HirTy::Ref { mutating, .. } => Some(RefReturn { mutating }),
            _ => None,
        }
    }
}

/// A subscript's / computed property's place-accessor children (stage 1.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlaceAccessorInfo {
    /// The `ref { … }` child (read provider), if declared.
    pub ref_accessor: Option<Entity>,
    /// The `mutating ref { … }` child (write/RMW provider), if declared.
    pub mutating_ref_accessor: Option<Entity>,
}

/// Query: the place-accessor children of a Subscript/Field member.
///
/// THE single discovery source for provider routing — type inference (read
/// provider types the member expr `&T`), analyze (mutability classification,
/// decl checks), and mir-lower (per-operation callee routing) all ask this
/// query; nobody walks `children_of` for `NodeKind::RefAccessor` ad hoc.
/// Returns None when the member has no ref accessors (the get/set world).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PlaceAccessors {
    pub entity: Entity,
}

impl QueryFn for PlaceAccessors {
    type Output = Option<PlaceAccessorInfo>;

    fn describe(&self) -> String {
        format!("PlaceAccessors(entity={:?})", self.entity)
    }

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<PlaceAccessorInfo> {
        let mut info = PlaceAccessorInfo {
            ref_accessor: None,
            mutating_ref_accessor: None,
        };
        for &child in ctx.children_of(self.entity) {
            if matches!(ctx.get::<NodeKind>(child), Some(NodeKind::RefAccessor)) {
                if ctx.get::<MutatingAccessor>(child).is_some() {
                    info.mutating_ref_accessor = Some(child);
                } else {
                    info.ref_accessor = Some(child);
                }
            }
        }
        (info.ref_accessor.is_some() || info.mutating_ref_accessor.is_some()).then_some(info)
    }
}

/// Query: lower a declaration entity's Callable param types to HirTy.
///
/// Reads the `Callable` component and lowers each param's type annotation.
/// Returns a Vec indexed to match `Callable.params` — None for unannotated params.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LowerCallableTypes {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for LowerCallableTypes {
    type Output = Option<Vec<Option<HirTy>>>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Vec<Option<HirTy>>> {
        let callable = ctx.get::<Callable>(self.entity)?;
        // Enum payloads live in the Callable component but are STORAGE
        // slots, not parameters: stage 2b accepts refs there (Field
        // position), while real params keep E480 on bare refs — ref-bearing
        // AGGREGATE params (`o: Optional[&T]`) are legal either way.
        let pos = if matches!(ctx.get::<NodeKind>(self.entity), Some(NodeKind::EnumCase)) {
            RefPosition::Field
        } else {
            RefPosition::Param
        };
        Some(
            callable
                .params
                .iter()
                .map(|p| {
                    p.ty.as_ref().map(|ast_ty| {
                        let lowered = lower_ast_type(ctx, self.entity, self.root, ast_ty);
                        reject_ref_types(ctx, lowered, pos, RefPolicy::AllowAggregate)
                    })
                })
                .collect(),
        )
    }
}

/// Query: lower an extension target's type arguments to HirTy.
///
/// For `extend Box[lang.i64]`, returns `Some(vec![HirTy for i64])`.
/// For `extend Box` (no type args), returns `Some(vec![])`.
/// Returns None if the entity has no ExtensionTarget component.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LowerExtensionTargetTypeArgs {
    pub extension: Entity,
    pub root: Entity,
}

impl QueryFn for LowerExtensionTargetTypeArgs {
    type Output = Option<Vec<HirTy>>;

    fn describe(&self) -> String {
        format!("LowerExtensionTargetTypeArgs(ext={:?})", self.extension)
    }

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Vec<HirTy>> {
        let target = ctx.get::<ExtensionTarget>(self.extension)?;
        let AstType::Named { segments, .. } = &target.0 else {
            return Some(vec![]);
        };

        // Type args are on the last path segment (the type name)
        let Some(last_seg) = segments.last() else {
            return Some(vec![]);
        };

        if last_seg.type_args.is_empty() {
            return Some(vec![]);
        }

        // If the target's arity is known and the extension provides too many
        // args, truncate. The arity-mismatch analyzer (E453) reports the
        // excess; lowering "cannot find type 'X'" for the extras would be
        // redundant noise. Args beyond the expected count become HirTy::Error
        // without triggering a name-resolution diagnostic.
        let expected_arity = ctx
            .query(ExtensionTargetEntity {
                extension: self.extension,
                root: self.root,
            })
            .and_then(|target| ctx.get::<TypeParams>(target).map(|tp| tp.0.len()));

        // Lower each type arg in the extension's own scope (so type params like T are visible)
        let context = self.extension;
        let limit = expected_arity.unwrap_or(last_seg.type_args.len());
        let args = last_seg
            .type_args
            .iter()
            .enumerate()
            .map(|(i, ast_ty)| {
                if i < limit {
                    let lowered = lower_ast_type(ctx, context, self.root, ast_ty);
                    // Extension targets stay Strict: extending a
                    // ref-instantiation engages witness machinery (2d).
                    reject_ref_types(ctx, lowered, RefPosition::GenericArg, RefPolicy::Strict)
                } else {
                    HirTy::Error(ast_type_span(ast_ty))
                }
            })
            .collect();
        Some(args)
    }
}

fn ast_type_span(ty: &AstType) -> Span {
    match ty {
        AstType::Named { span, .. }
        | AstType::Tuple(_, span)
        | AstType::Function { span, .. }
        | AstType::Array(_, span)
        | AstType::Optional(_, span)
        | AstType::Dictionary(_, _, span)
        | AstType::Result { span, .. }
        | AstType::Unit(span)
        | AstType::Never(span)
        | AstType::Inferred(span)
        | AstType::Some { span, .. }
        | AstType::Ref { span, .. } => span.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast::PathSegment;
    use kestrel_ast_builder::{Name, Typed};
    use kestrel_hecs::World;

    fn span() -> Span {
        Span::synthetic(0)
    }

    fn seg(name: &str) -> PathSegment {
        PathSegment {
            name: name.into(),
            type_args: vec![],
            span: span(),
        }
    }

    /// An associated type reference (`Iter.Item` where `Item` is a TypeAlias
    /// child of the `Iter` protocol) must lower to `HirTy::AssocProjection`,
    /// preserving the base (`Iter`) so the solver can project it through a
    /// concrete receiver. Previously this arm returned `HirTy::AliasUse`
    /// dropping the base, which caused associated-type names to leak into
    /// diagnostics ("Array[Item]" instead of "Array[Int64]").
    #[test]
    fn associated_type_lowers_to_assoc_projection() {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        let iter_proto = world.spawn();
        world.set(iter_proto, NodeKind::Protocol);
        world.set(iter_proto, Name("TargetIterator".into()));
        world.set(iter_proto, Typed);
        world.set_parent(iter_proto, root);

        let item_alias = world.spawn();
        world.set(item_alias, NodeKind::TypeAlias);
        world.set(item_alias, Name("Item".into()));
        world.set(item_alias, Typed);
        world.set_parent(item_alias, iter_proto);

        let ctx = world.query_context();
        let ast_ty = AstType::Named {
            segments: vec![seg("TargetIterator"), seg("Item")],
            span: span(),
        };
        let lowered = lower_ast_type(&ctx, root, root, &ast_ty);
        match lowered {
            HirTy::AssocProjection { base, assoc, .. } => {
                assert_eq!(assoc, item_alias);
                match *base {
                    HirTy::Protocol { entity, .. } => assert_eq!(entity, iter_proto),
                    other => panic!("expected Protocol base, got {other:?}"),
                }
            },
            other => panic!("expected AssocProjection, got {other:?}"),
        }
    }

    /// Non-associated TypeAliases (parent is Module, not Protocol) still
    /// lower to `HirTy::AliasUse`. Guards against over-broadening the
    /// `parent_is_protocol` branch.
    #[test]
    fn module_level_alias_stays_alias_use() {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        let alias = world.spawn();
        world.set(alias, NodeKind::TypeAlias);
        world.set(alias, Name("Fd".into()));
        world.set(alias, Typed);
        // no TypeAnnotation → non-trivial; stays as AliasUse
        let tp = world.spawn();
        world.set(alias, kestrel_ast_builder::TypeParams(vec![tp]));
        world.set_parent(alias, root);

        let ctx = world.query_context();
        let ast_ty = AstType::Named {
            segments: vec![seg("Fd")],
            span: span(),
        };
        let lowered = lower_ast_type(&ctx, root, root, &ast_ty);
        matches!(lowered, HirTy::AliasUse { .. });
    }
}
