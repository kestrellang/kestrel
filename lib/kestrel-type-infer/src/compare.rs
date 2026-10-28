//! Declaration type comparison helpers.
//!
//! This module compares already-lowered `HirTy` values under a small
//! environment. It is intentionally narrower than full body inference: callers
//! provide the conformance-specific substitutions for `Self` and associated
//! type bindings, and comparison reduces aliases/projections into `ResolvedTy`.

use crate::collections::HashMap;

use kestrel_ast_builder::Name;
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::ty::HirTy;

use crate::result::ResolvedTy;

#[derive(Clone, Debug, Default)]
pub struct TypeCompareEnv {
    pub self_ty: Option<ResolvedTy>,
    pub assoc_bindings: Vec<AssocBinding>,
    /// Direct substitutions for `HirTy::Param(entity, _)` during comparison.
    /// Used to align method-level type parameters between a protocol
    /// requirement and its impl: the protocol's `func make[U] -> U` and the
    /// impl's `func make[U] -> U` have *different* `U` entities, so without
    /// substitution the returns compare unequal. Seeded into both sides'
    /// NormalizeState so alias expansion's own save/restore still works.
    pub param_subs: Vec<(Entity, ResolvedTy)>,
}

#[derive(Clone, Debug)]
pub struct AssocBinding {
    pub assoc: Entity,
    pub name: String,
    pub ty: HirTy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeCompareResult {
    Equal,
    NotEqual {
        expected: ResolvedTy,
        actual: ResolvedTy,
    },
    Unknown,
}

impl TypeCompareResult {
    pub fn is_equal_or_unknown(&self) -> bool {
        matches!(self, Self::Equal | Self::Unknown)
    }
}

#[derive(Clone, Default)]
struct NormalizeState {
    param_subs: HashMap<Entity, ResolvedTy>,
    assoc_stack: Vec<Entity>,
}

/// A fully written type (an `is T` target) as a `ResolvedTy`, aliases and
/// projections reduced. Same reduction as [`compare_hir_types`].
pub fn resolve_hir_type(qctx: &QueryContext<'_>, ty: &HirTy) -> ResolvedTy {
    let env = TypeCompareEnv::default();
    normalize_hir_type(qctx, ty, &env, &mut NormalizeState::default())
}

pub fn compare_hir_types(
    qctx: &QueryContext<'_>,
    expected: &HirTy,
    actual: &HirTy,
    env: &TypeCompareEnv,
) -> TypeCompareResult {
    let seed_state = || {
        let mut state = NormalizeState::default();
        for (entity, ty) in &env.param_subs {
            state.param_subs.insert(*entity, ty.clone());
        }
        state
    };
    let mut expected_state = seed_state();
    let mut actual_state = seed_state();
    let expected = normalize_hir_type(qctx, expected, env, &mut expected_state);
    let actual = normalize_hir_type(qctx, actual, env, &mut actual_state);

    if contains_error(&expected) || contains_error(&actual) {
        TypeCompareResult::Unknown
    } else if expected == actual {
        TypeCompareResult::Equal
    } else {
        TypeCompareResult::NotEqual { expected, actual }
    }
}

fn normalize_hir_type(
    qctx: &QueryContext<'_>,
    ty: &HirTy,
    env: &TypeCompareEnv,
    state: &mut NormalizeState,
) -> ResolvedTy {
    match ty {
        HirTy::Struct { entity, args, .. }
        | HirTy::Enum { entity, args, .. }
        | HirTy::Protocol { entity, args, .. } => {
            if let Some(sub) = state.param_subs.get(entity) {
                return sub.clone();
            }
            ResolvedTy::Named {
                entity: *entity,
                args: args
                    .iter()
                    .map(|arg| normalize_hir_type(qctx, arg, env, state))
                    .collect(),
            }
        },
        // Opaque types don't have a single entity; treat as Error for now
        HirTy::Opaque { .. } => ResolvedTy::Error,
        // Aliases arrive expanded; a bounded one compares as its expansion.
        HirTy::Alias { expanded, .. } => normalize_hir_type(qctx, expanded, env, state),
        HirTy::RawExistential {
            bounds, bindings, ..
        } => ResolvedTy::RawExistential {
            bounds: bounds
                .iter()
                .filter_map(|b| match b {
                    HirTy::Protocol { entity, args, .. } => Some((
                        *entity,
                        args.iter()
                            .map(|a| normalize_hir_type(qctx, a, env, state))
                            .collect(),
                    )),
                    _ => None,
                })
                .collect(),
            bindings: bindings
                .iter()
                .map(|b| (b.assoc, normalize_hir_type(qctx, &b.ty, env, state)))
                .collect(),
        },
        HirTy::AbstractAssoc { entity, .. } => {
            if let Some(sub) = state.param_subs.get(entity) {
                return sub.clone();
            }
            if let Some(binding) = binding_for_assoc(env, qctx, *entity) {
                if state.assoc_stack.contains(entity) {
                    return ResolvedTy::Error;
                }
                state.assoc_stack.push(*entity);
                let normalized = normalize_hir_type(qctx, &binding.ty, env, state);
                state.assoc_stack.pop();
                return normalized;
            }
            ResolvedTy::Named {
                entity: *entity,
                args: Vec::new(),
            }
        },
        HirTy::Param(entity, _) => state
            .param_subs
            .get(entity)
            .cloned()
            .unwrap_or(ResolvedTy::Param { entity: *entity }),
        HirTy::SelfType(_, _) => env.self_ty.clone().unwrap_or_else(|| match ty {
            HirTy::SelfType(entity, _) => ResolvedTy::SelfType { entity: *entity },
            _ => unreachable!(),
        }),
        HirTy::AssocProjection { assoc, .. } => {
            if state.assoc_stack.contains(assoc) {
                return ResolvedTy::Error;
            }
            if let Some(binding) = binding_for_assoc(env, qctx, *assoc) {
                state.assoc_stack.push(*assoc);
                let normalized = normalize_hir_type(qctx, &binding.ty, env, state);
                state.assoc_stack.pop();
                normalized
            } else {
                ResolvedTy::Error
            }
        },
        HirTy::Tuple(elems, _) => ResolvedTy::Tuple(
            elems
                .iter()
                .map(|elem| normalize_hir_type(qctx, elem, env, state))
                .collect(),
        ),
        // The closure KIND rides through normalization, so requirement/witness
        // signature equality is kind-exact: a `func f(cb: escaping () -> ())`
        // requirement is not witnessed by a `func f(cb: () -> ())` impl (their
        // environment representations differ), exactly as `&` never matches
        // `&mutating` below.
        HirTy::Function {
            kind,
            params,
            param_conventions,
            ret,
            ..
        } => ResolvedTy::Function {
            kind: *kind,
            params: params
                .iter()
                .map(|param| normalize_hir_type(qctx, param, env, state))
                .collect(),
            conventions: param_conventions.clone(),
            ret: Box::new(normalize_hir_type(qctx, ret, env, state)),
        },
        HirTy::Never(_) => ResolvedTy::Never,
        HirTy::Infer(_) | HirTy::Error(_) => ResolvedTy::Error,
        // Faithful Ref normalization (stage 2d): conformance signature
        // checking compares ref returns (`-> &Self.Item` vs an impl's
        // `-> &T`) and ref-bearing aggregates. The derived equality then
        // enforces exact shape AND mutability — a `-> T` impl never
        // witnesses a `-> &T` requirement (the ABIs differ: raw pointer
        // vs owned value) and `&` never matches `&mutating`.
        HirTy::Ref {
            inner, mutating, ..
        } => ResolvedTy::Ref {
            pointee: Box::new(normalize_hir_type(qctx, inner, env, state)),
            mutating: *mutating,
        },
    }
}

fn binding_for_assoc<'a>(
    env: &'a TypeCompareEnv,
    qctx: &QueryContext<'_>,
    assoc: Entity,
) -> Option<&'a AssocBinding> {
    env.assoc_bindings
        .iter()
        .find(|binding| binding.assoc == assoc)
        .or_else(|| {
            let name = qctx.get::<Name>(assoc)?;
            env.assoc_bindings
                .iter()
                .find(|binding| binding.name == name.0)
        })
}

fn contains_error(ty: &ResolvedTy) -> bool {
    match ty {
        ResolvedTy::Error => true,
        ResolvedTy::Named { args, .. } | ResolvedTy::Tuple(args) => args.iter().any(contains_error),
        ResolvedTy::Function { params, ret, .. } => {
            params.iter().any(contains_error) || contains_error(ret)
        },
        ResolvedTy::AssocProjection { base, .. } => contains_error(base),
        ResolvedTy::Ref { pointee, .. } => contains_error(pointee),
        ResolvedTy::Opaque {
            bounds,
            origin_args,
            ..
        } => {
            bounds
                .iter()
                .any(|(_, args)| args.iter().any(contains_error))
                || origin_args.iter().any(contains_error)
        },
        ResolvedTy::RawExistential { bounds, bindings } => {
            bounds
                .iter()
                .any(|(_, args)| args.iter().any(contains_error))
                || bindings.iter().any(|(_, t)| contains_error(t))
        },
        ResolvedTy::Param { .. } | ResolvedTy::SelfType { .. } | ResolvedTy::Never => false,
    }
}
