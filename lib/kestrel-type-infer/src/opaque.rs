//! The concrete type behind an opaque `some P` return, with every nested
//! opaque type expanded — the one place that follows opaque types through
//! each other, so the query framework sees (and recovers from) a cycle.
//!
//! `func g() -> some P { W(inner: h()) }` + `func h() -> some P { W(inner:
//! g()) }` describe an infinite type that unification never sees (each body
//! only names the other's opaque type). Expanding `g` asks for `h`, which
//! asks for `g`: the cycle falls back to [`OpaqueConcrete::Cyclic`] for every
//! member, and the analyzer reports it (E408).

use kestrel_ast_builder::TypeParams;
use kestrel_hecs::{CycleMode, Entity, QueryContext, QueryFn};

use crate::InferBody;
use crate::result::ResolvedTy;

/// The expanded concrete type of an opaque origin.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum OpaqueConcrete {
    /// No opaque type left inside; written in the origin's own type params
    /// followed by its containers' ([`origin_type_params`]).
    Resolved(ResolvedTy),
    /// The body never produced a concrete type (an inference error that was
    /// already reported).
    Unresolved,
    /// The concrete type contains itself through other opaque types.
    Cyclic,
}

/// [`OpaqueConcrete`] of the function `origin` returning `some P`.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct OpaqueConcreteTy {
    pub origin: Entity,
    pub root: Entity,
}

impl QueryFn for OpaqueConcreteTy {
    type Output = OpaqueConcrete;

    fn execute(&self, ctx: &QueryContext<'_>) -> OpaqueConcrete {
        let body = ctx.query(InferBody {
            entity: self.origin,
            root: self.root,
        });
        let Some(concrete) = body.as_ref().and_then(|b| b.opaque_concrete_type.as_ref()) else {
            return OpaqueConcrete::Unresolved;
        };
        match expand(ctx, concrete, self.root) {
            Ok(ty) => OpaqueConcrete::Resolved(ty),
            Err(failure) => failure,
        }
    }

    /// A cycle is an infinite type: every member answers `Cyclic`.
    fn cycle_mode(&self) -> CycleMode<OpaqueConcrete> {
        CycleMode::Fallback {
            value: OpaqueConcrete::Cyclic,
        }
    }

    fn describe(&self) -> String {
        format!("OpaqueConcreteTy({:?})", self.origin)
    }
}

/// The params an opaque's `origin_args` are parallel to: the origin's own
/// type params, then those of each enclosing container. A method's `some P`
/// underlier can mention the enclosing type's params
/// (`Factory[T].makeOne() -> some Counter { self.proto }`, underlier `T`).
pub fn origin_type_params(ctx: &QueryContext<'_>, origin: Entity) -> Vec<Entity> {
    let mut params = Vec::new();
    let mut cur = Some(origin);
    while let Some(entity) = cur {
        if let Some(tp) = ctx.get::<TypeParams>(entity) {
            params.extend(tp.0.iter().copied());
        }
        cur = ctx.parent_of(entity);
    }
    params
}

/// Replace every nested opaque type in `ty` with its expanded concrete type,
/// instantiated at that opaque's `origin_args`.
fn expand(
    ctx: &QueryContext<'_>,
    ty: &ResolvedTy,
    root: Entity,
) -> Result<ResolvedTy, OpaqueConcrete> {
    let all = |tys: &[ResolvedTy]| -> Result<Vec<ResolvedTy>, OpaqueConcrete> {
        tys.iter().map(|t| expand(ctx, t, root)).collect()
    };
    Ok(match ty {
        ResolvedTy::Opaque {
            origin,
            origin_args,
            ..
        } => {
            let args = all(origin_args)?;
            match ctx.query(OpaqueConcreteTy {
                origin: *origin,
                root,
            }) {
                OpaqueConcrete::Resolved(concrete) => {
                    substitute_resolved_ty(&concrete, &origin_type_params(ctx, *origin), &args)
                },
                failure => return Err(failure),
            }
        },
        ResolvedTy::Named { entity, args } => ResolvedTy::Named {
            entity: *entity,
            args: all(args)?,
        },
        ResolvedTy::Tuple(elems) => ResolvedTy::Tuple(all(elems)?),
        ResolvedTy::Function {
            kind,
            params,
            conventions,
            ret,
        } => ResolvedTy::Function {
            kind: *kind,
            params: all(params)?,
            conventions: conventions.clone(),
            ret: Box::new(expand(ctx, ret, root)?),
        },
        ResolvedTy::AssocProjection { base, assoc } => ResolvedTy::AssocProjection {
            base: Box::new(expand(ctx, base, root)?),
            assoc: *assoc,
        },
        ResolvedTy::Ref { pointee, mutating } => ResolvedTy::Ref {
            pointee: Box::new(expand(ctx, pointee, root)?),
            mutating: *mutating,
        },
        ResolvedTy::RawExistential { bounds, bindings } => ResolvedTy::RawExistential {
            bounds: bounds
                .iter()
                .map(|(p, args)| Ok((*p, all(args)?)))
                .collect::<Result<_, OpaqueConcrete>>()?,
            bindings: bindings
                .iter()
                .map(|(a, t)| Ok((*a, expand(ctx, t, root)?)))
                .collect::<Result<_, OpaqueConcrete>>()?,
        },
        ResolvedTy::Param { .. }
        | ResolvedTy::SelfType { .. }
        | ResolvedTy::Never
        | ResolvedTy::Error => ty.clone(),
    })
}

/// Substitute type params in a ResolvedTy (for opaque return type resolution).
///
/// **Exhaustive on purpose — do not add a `_ =>` arm.** This is the second
/// substitution kernel in the tree (`kestrel_mir::substitute` is the first, over
/// `MirTy`, and is exhaustive for the same reason). It used to end in
/// `_ => ty.clone()`, which silently stopped recursing at `Ref` and `Opaque`
/// even though both carry nested `ResolvedTy`s, and guaranteed that any future
/// variant would be skipped rather than be a compile error (fragility audit
/// F32). Leaf variants are listed by name so adding one is a decision.
pub fn substitute_resolved_ty(
    ty: &ResolvedTy,
    type_params: &[Entity],
    args: &[ResolvedTy],
) -> ResolvedTy {
    match ty {
        ResolvedTy::Param { entity } => {
            for (i, tp) in type_params.iter().enumerate() {
                if tp == entity
                    && let Some(arg) = args.get(i)
                {
                    return arg.clone();
                }
            }
            ty.clone()
        },
        ResolvedTy::Named {
            entity,
            args: ty_args,
        } => ResolvedTy::Named {
            entity: *entity,
            args: ty_args
                .iter()
                .map(|a| substitute_resolved_ty(a, type_params, args))
                .collect(),
        },
        ResolvedTy::Tuple(elems) => ResolvedTy::Tuple(
            elems
                .iter()
                .map(|e| substitute_resolved_ty(e, type_params, args))
                .collect(),
        ),
        ResolvedTy::Function {
            kind,
            params: fn_params,
            conventions,
            ret,
        } => ResolvedTy::Function {
            kind: *kind,
            params: fn_params
                .iter()
                .map(|p| substitute_resolved_ty(p, type_params, args))
                .collect(),
            conventions: conventions.clone(),
            ret: Box::new(substitute_resolved_ty(ret, type_params, args)),
        },
        ResolvedTy::AssocProjection { base, assoc } => ResolvedTy::AssocProjection {
            base: Box::new(substitute_resolved_ty(base, type_params, args)),
            assoc: *assoc,
        },
        ResolvedTy::Ref { pointee, mutating } => ResolvedTy::Ref {
            pointee: Box::new(substitute_resolved_ty(pointee, type_params, args)),
            mutating: *mutating,
        },
        ResolvedTy::Opaque {
            origin,
            bounds,
            origin_args,
            index,
            not_copyable,
        } => ResolvedTy::Opaque {
            origin: *origin,
            // A bound's own arguments can mention the params being substituted
            // (`some Collection[T]`), as can the args the opaque was produced
            // with. The bound protocol entity itself is not a type.
            bounds: bounds
                .iter()
                .map(|(protocol, bound_args)| {
                    (
                        *protocol,
                        bound_args
                            .iter()
                            .map(|a| substitute_resolved_ty(a, type_params, args))
                            .collect(),
                    )
                })
                .collect(),
            origin_args: origin_args
                .iter()
                .map(|a| substitute_resolved_ty(a, type_params, args))
                .collect(),
            index: *index,
            not_copyable: *not_copyable,
        },
        ResolvedTy::RawExistential { bounds, bindings } => ResolvedTy::RawExistential {
            bounds: bounds
                .iter()
                .map(|(protocol, bound_args)| {
                    (
                        *protocol,
                        bound_args
                            .iter()
                            .map(|a| substitute_resolved_ty(a, type_params, args))
                            .collect(),
                    )
                })
                .collect(),
            bindings: bindings
                .iter()
                .map(|(assoc, t)| (*assoc, substitute_resolved_ty(t, type_params, args)))
                .collect(),
        },
        // Leaves: no nested types to walk into.
        ResolvedTy::SelfType { .. } | ResolvedTy::Never | ResolvedTy::Error => ty.clone(),
    }
}
