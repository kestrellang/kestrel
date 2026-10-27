//! Sizedness: does a type have a size known at compile time? (existentials
//! design §2).
//!
//! Every type is **Sized** except the raw existential an `any P` box holds,
//! and a struct whose *last* stored field is an unsized type parameter (an
//! "unsized tail", like `RcBoxStorage[T]`) instantiated at an unsized type.
//! `Sized` is structural, like `Static`: computed, never declared. Every type
//! parameter carries an implicit `T: Sized`, relaxed with `where T: not Sized`;
//! an associated type is Sized unless declared `type X: not Sized`.
//!
//! The shape mirrors `staticness.rs` (a per-nominal classification with
//! gating positions, one per-instantiation rule shared by every layer), but
//! only the tail field can gate: every other stored field must be Sized
//! (E403 enforces that).

use kestrel_ast_builder::WhereConstraint;
use kestrel_ast_builder::{
    FieldClass, Intrinsic, NodeKind, TypeParams, WhereClause as AstWhereClause,
};
use kestrel_hecs::{CycleMode, Entity, QueryContext, QueryFn};
use kestrel_hir::{Builtin, HirTy};
use kestrel_hir_lower::LowerTypeAnnotation;
use kestrel_name_res::ResolveBuiltin;

use crate::{ExplicitlyNegatesProtocol, param_bound_contexts, resolve_type_entity};

// ===== Vocabulary =====

/// Per-nominal classification. There is no unconditional "unsized"
/// nominal: a struct is unsized only through its tail type argument.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Sizedness {
    Sized,
    /// Sized iff the type args at these positions (indices into the
    /// nominal's type params, sorted, deduped) are Sized. Non-empty only for
    /// a struct whose last stored field is (transitively) one of its own
    /// `not Sized` params.
    ConditionalOn(Vec<usize>),
}

/// Bound state of a type param: every param requires Sized by default;
/// `where T: not Sized` relaxes it — the param then accepts both sized and
/// unsized arguments (need-not, the `not Copyable` convention). A positive
/// `where T: Sized` in the asking context restores it, which is how an
/// `extend Box[T] where T: Sized` extension gets to use `T` by value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SizeRequirement {
    RequiresSized,
    MayBeUnsized,
}

// ===== Type-param requirement =====

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypeParamSizeRequirement {
    pub param: Entity,
    pub context: Entity,
    pub root: Entity,
}

impl QueryFn for TypeParamSizeRequirement {
    type Output = SizeRequirement;

    fn execute(&self, ctx: &QueryContext<'_>) -> SizeRequirement {
        // No builtin (stdlib-less fixture): nothing can spell a relaxation.
        let Some(sized) = ctx.query(ResolveBuiltin {
            builtin: Builtin::Sized,
            root: self.root,
        }) else {
            return SizeRequirement::RequiresSized;
        };
        // A compiler intrinsic's params (`lang.ptr[T]`, `lang.&`, `lang.*`
        // functions) work at the ABI level and accept an unsized argument —
        // the same reason the implicit-bound injection skips intrinsics.
        if ctx
            .parent_of(self.param)
            .is_some_and(|owner| ctx.get::<Intrinsic>(owner).is_some())
        {
            return SizeRequirement::MayBeUnsized;
        }
        let mut relaxed = false;
        for context in param_bound_contexts(ctx, self.param, self.context) {
            let Some(wc) = ctx.get::<AstWhereClause>(context) else {
                continue;
            };
            for constraint in &wc.0 {
                match constraint {
                    WhereConstraint::NegativeBound {
                        subject, protocol, ..
                    } => {
                        if resolve_type_entity(ctx, subject, context, self.root) == Some(self.param)
                            && resolve_type_entity(ctx, protocol, context, self.root) == Some(sized)
                        {
                            relaxed = true;
                        }
                    },
                    // A positive `T: Sized` anywhere in the chain wins: the
                    // asking context is inside a `where T: Sized` scope. So
                    // does a bitwise-copy bound (`T: Copyable`), which needs
                    // a compile-time size.
                    WhereConstraint::Bound {
                        subject, protocols, ..
                    } => {
                        if resolve_type_entity(ctx, subject, context, self.root) != Some(self.param)
                        {
                            continue;
                        }
                        let implies_sized = protocols.iter().any(|p| {
                            resolve_type_entity(ctx, p, context, self.root)
                                .is_some_and(|p| p == sized || implies_bitwise_copy(ctx, p, self.root))
                        });
                        if implies_sized {
                            return SizeRequirement::RequiresSized;
                        }
                    },
                    WhereConstraint::Equality { .. } => {},
                }
            }
        }
        if relaxed {
            SizeRequirement::MayBeUnsized
        } else {
            SizeRequirement::RequiresSized
        }
    }

    fn describe(&self) -> String {
        format!("TypeParamSizeRequirement({:?})", self.param)
    }
}

/// Does a `T: protocol` bound mean `T` is bitwise-copied, so needs a
/// compile-time size? True for `Copyable` and its refinements, except the
/// `Cloneable` family: a clone runs `clone()`, which an existential reaches
/// through its function table (`any[CowBox] P` with `P: Cloneable`).
fn implies_bitwise_copy(ctx: &QueryContext<'_>, protocol: Entity, root: Entity) -> bool {
    let refines = |builtin| {
        ctx.query(ResolveBuiltin { builtin, root }).is_some_and(|base| {
            ctx.query(crate::ProtocolRefines {
                protocol,
                base,
                root,
            })
        })
    };
    refines(Builtin::Copyable) && !refines(Builtin::Cloneable)
}

/// Is the associated type `assoc` declared `type X: not Sized`?
pub fn assoc_may_be_unsized(ctx: &QueryContext<'_>, assoc: Entity, root: Entity) -> bool {
    ctx.query(ResolveBuiltin {
        builtin: Builtin::Sized,
        root,
    })
    .is_some_and(|sized| {
        ctx.query(ExplicitlyNegatesProtocol {
            entity: assoc,
            protocol: sized,
            root,
        })
    })
}

// ===== The shared instance rule =====

/// Layer hooks for `instance_is_sized`: the HIR walker below and the
/// solver's TyVar layer.
pub trait SizedLayer {
    type Ty;
    fn nominal_sizedness(&self, entity: Entity) -> Sizedness;
    fn member_is_sized(&self, ty: &Self::Ty) -> bool;
}

/// THE per-instantiation sizedness rule for every layer: a Sized base wins;
/// a conditional base ANDs the args at its tail positions. A missing arg at
/// a gating position is unprovable ⇒ false.
pub fn instance_is_sized<L: SizedLayer>(layer: &L, entity: Entity, args: &[L::Ty]) -> bool {
    match layer.nominal_sizedness(entity) {
        Sizedness::Sized => true,
        Sizedness::ConditionalOn(positions) => positions
            .iter()
            .all(|&i| args.get(i).is_some_and(|arg| layer.member_is_sized(arg))),
    }
}

// ===== Nominal classification =====

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NominalSizedness {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for NominalSizedness {
    type Output = Sizedness;

    fn execute(&self, ctx: &QueryContext<'_>) -> Sizedness {
        nominal_sizedness_impl(ctx, self.entity, self.root)
    }

    /// A struct whose unsized tail reaches itself is an infinite type,
    /// reported elsewhere. Start at Sized and let the gating positions grow
    /// (bounded by the param count) to the fixpoint.
    fn cycle_mode(&self) -> CycleMode<Sizedness> {
        CycleMode::Fixpoint {
            initial: Sizedness::Sized,
        }
    }

    fn describe(&self) -> String {
        format!("NominalSizedness({:?})", self.entity)
    }
}

fn nominal_sizedness_impl(ctx: &QueryContext<'_>, entity: Entity, root: Entity) -> Sizedness {
    // Enums hold payloads by value and every payload must be Sized; only a
    // struct can have an unsized tail.
    if ctx.get::<NodeKind>(entity) != Some(&NodeKind::Struct) {
        return Sizedness::Sized;
    }
    let Some(tail) = last_stored_field_type(ctx, entity, root) else {
        return Sizedness::Sized;
    };
    let own_params: Vec<Entity> = ctx
        .get::<TypeParams>(entity)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    let mut positions = tail_positions(ctx, &tail, entity, &own_params, root);
    if positions.is_empty() {
        return Sizedness::Sized;
    }
    positions.sort_unstable();
    positions.dedup();
    Sizedness::ConditionalOn(positions)
}

/// `entity`'s last stored instance field, in declaration order — the one
/// field allowed to be an unsized tail.
pub fn last_stored_field(ctx: &QueryContext<'_>, entity: Entity) -> Option<Entity> {
    ctx.children_of(entity)
        .iter()
        .rev()
        .copied()
        .find(|&child| {
            ctx.get::<NodeKind>(child) == Some(&NodeKind::Field)
                && ctx
                    .get::<FieldClass>(child)
                    .is_some_and(FieldClass::is_stored_instance)
        })
}

/// The type of `entity`'s last stored instance field.
pub fn last_stored_field_type(
    ctx: &QueryContext<'_>,
    entity: Entity,
    root: Entity,
) -> Option<HirTy> {
    ctx.query(LowerTypeAnnotation {
        entity: last_stored_field(ctx, entity)?,
        root,
    })
}

/// Which of `owner`'s own `not Sized` params a tail type's size depends on.
fn tail_positions(
    ctx: &QueryContext<'_>,
    tail: &HirTy,
    owner: Entity,
    own_params: &[Entity],
    root: Entity,
) -> Vec<usize> {
    match tail {
        HirTy::Param(param, _) => {
            let relaxed = ctx.query(TypeParamSizeRequirement {
                param: *param,
                context: owner,
                root,
            }) == SizeRequirement::MayBeUnsized;
            match own_params.iter().position(|p| p == param) {
                Some(i) if relaxed => vec![i],
                _ => Vec::new(),
            }
        },
        HirTy::Struct { entity, args, .. } => match ctx.query(NominalSizedness {
            entity: *entity,
            root,
        }) {
            Sizedness::Sized => Vec::new(),
            Sizedness::ConditionalOn(gating) => gating
                .iter()
                .filter_map(|&i| args.get(i))
                .flat_map(|arg| tail_positions(ctx, arg, owner, own_params, root))
                .collect(),
        },
        _ => Vec::new(),
    }
}

// ===== The HIR walker =====

struct HirSizedWalk<'a, 'q> {
    ctx: &'a QueryContext<'q>,
    context: Entity,
    root: Entity,
}

impl SizedLayer for HirSizedWalk<'_, '_> {
    type Ty = HirTy;

    fn nominal_sizedness(&self, entity: Entity) -> Sizedness {
        self.ctx.query(NominalSizedness {
            entity,
            root: self.root,
        })
    }

    fn member_is_sized(&self, ty: &HirTy) -> bool {
        match ty {
            HirTy::RawExistential { .. } => false,
            HirTy::Struct { entity, args, .. } => instance_is_sized(self, *entity, args),
            HirTy::Param(param, _) => {
                self.ctx.query(TypeParamSizeRequirement {
                    param: *param,
                    context: self.context,
                    root: self.root,
                }) == SizeRequirement::RequiresSized
            },
            HirTy::AssocProjection { assoc, .. } => {
                !assoc_may_be_unsized(self.ctx, *assoc, self.root)
            },
            // Enums, tuples, functions, refs (two words even to an unsized
            // pointee), protocols, opaques, Self, aliases and recovery leaves.
            HirTy::Enum { .. }
            | HirTy::Protocol { .. }
            | HirTy::Tuple(..)
            | HirTy::Function { .. }
            | HirTy::AliasUse { .. }
            | HirTy::SelfType(..)
            | HirTy::Opaque { .. }
            | HirTy::Ref { .. }
            | HirTy::Never(_)
            | HirTy::Infer(_)
            | HirTy::Error(_) => true,
        }
    }
}

/// Is this HIR type provably Sized? `context` scopes type-param bound
/// lookups (the entity whose where-clauses govern params in `ty`).
pub fn hir_type_is_sized(
    ctx: &QueryContext<'_>,
    ty: &HirTy,
    context: Entity,
    root: Entity,
) -> bool {
    HirSizedWalk { ctx, context, root }.member_is_sized(ty)
}

// ===== By-value uses of a relaxed param =====

/// The first `not Sized` type param `ty` uses **by value**, judged from
/// `context`, if any — the one rule behind E403 (unsized param by value) and
/// E402 (a box field storing its payload by value). A use is by value unless
/// it sits behind a pointer: `&T`, or an argument at a `not Sized`
/// parameter (`Pointer[T]`, a box). A bare param in such a position is fine;
/// anything built around it (`Pointer[Optional[T]]`) is still checked.
pub fn relaxed_param_by_value(
    ctx: &QueryContext<'_>,
    ty: &HirTy,
    context: Entity,
    root: Entity,
) -> Option<Entity> {
    ByValueWalk {
        query: ctx,
        context,
        root,
    }
    .by_value(ty)
}

struct ByValueWalk<'a, 'q> {
    query: &'a QueryContext<'q>,
    context: Entity,
    root: Entity,
}

impl ByValueWalk<'_, '_> {
    fn relaxed(&self, param: Entity) -> bool {
        self.query.query(TypeParamSizeRequirement {
            param,
            context: self.context,
            root: self.root,
        }) == SizeRequirement::MayBeUnsized
    }

    fn top_level_param(&self, ty: &HirTy) -> bool {
        matches!(ty, HirTy::Param(..))
    }

    /// The first relaxed param `ty` uses by value, if any.
    fn by_value(&self, ty: &HirTy) -> Option<Entity> {
        match ty {
            HirTy::Param(p, _) => self.relaxed(*p).then_some(*p),
            // Behind a pointer: the pointee itself may be unsized.
            HirTy::Ref { inner, .. } => self.behind_pointer(inner),
            HirTy::Struct { entity, args, .. } | HirTy::Enum { entity, args, .. } => {
                self.nominal_args(*entity, args)
            },
            HirTy::Tuple(elems, _) => elems.iter().find_map(|e| self.by_value(e)),
            HirTy::Function { params, ret, .. } => params
                .iter()
                .chain(std::iter::once(ret.as_ref()))
                .find_map(|t| self.by_value(t)),
            HirTy::AliasUse { args, .. } | HirTy::Protocol { args, .. } => {
                args.iter().find_map(|a| self.by_value(a))
            },
            HirTy::RawExistential { bindings, .. } => {
                bindings.iter().find_map(|b| self.by_value(&b.ty))
            },
            HirTy::AssocProjection { .. }
            | HirTy::SelfType(..)
            | HirTy::Opaque { .. }
            | HirTy::Never(_)
            | HirTy::Infer(_)
            | HirTy::Error(_) => None,
        }
    }

    /// A position that may itself be unsized: a bare param is fine there,
    /// but anything built around it is still checked.
    fn behind_pointer(&self, ty: &HirTy) -> Option<Entity> {
        if self.top_level_param(ty) {
            return None;
        }
        self.by_value(ty)
    }

    /// A nominal's args: an arg at a `not Sized` parameter is behind a
    /// pointer (`Pointer[T]`, a box); every other arg is by value.
    fn nominal_args(&self, entity: Entity, args: &[HirTy]) -> Option<Entity> {
        let params: Vec<Entity> = self
            .query
            .get::<TypeParams>(entity)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        args.iter().enumerate().find_map(|(i, arg)| {
            let unsized_ok = params.get(i).is_some_and(|&p| {
                self.query.query(TypeParamSizeRequirement {
                    param: p,
                    context: entity,
                    root: self.root,
                }) == SizeRequirement::MayBeUnsized
            });
            if unsized_ok {
                self.behind_pointer(arg)
            } else {
                self.by_value(arg)
            }
        })
    }
}

// ===== Existential box validity (E402) =====

/// Why a type cannot be the `B` of `any[B] P` (existentials design §3).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BoxDefect {
    /// Not exactly one type parameter.
    ParamCount,
    /// The one type parameter is not declared `not Sized`.
    ParamNotRelaxed,
    /// No conformance to `ExistentialBox`.
    NotExistentialBox,
    /// A stored field holds the payload by value, so the box can't be
    /// widened to carry an unsized payload.
    FieldByValue { field: Entity, param: Entity },
}

/// Is `entity` a valid existential box? `None` when it is. Checked in the
/// order the defects are listed, so the first one a user hits is reported.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExistentialBoxDefect {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for ExistentialBoxDefect {
    type Output = Option<BoxDefect>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<BoxDefect> {
        let params = ctx
            .get::<TypeParams>(self.entity)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        let [param] = params.as_slice() else {
            return Some(BoxDefect::ParamCount);
        };
        let relaxed = ctx.query(TypeParamSizeRequirement {
            param: *param,
            context: self.entity,
            root: self.root,
        }) == SizeRequirement::MayBeUnsized;
        if !relaxed {
            return Some(BoxDefect::ParamNotRelaxed);
        }
        let conforms = ctx
            .query(ResolveBuiltin {
                builtin: Builtin::ExistentialBox,
                root: self.root,
            })
            .is_some_and(|protocol| {
                ctx.query(crate::DeclaresConformanceTo {
                    entity: self.entity,
                    protocol,
                    root: self.root,
                })
            });
        if !conforms {
            return Some(BoxDefect::NotExistentialBox);
        }
        // Every stored field must hold the payload only behind a pointer.
        // Unlike E403 there is no tail exemption: a box is a fixed-size
        // handle, so even a last-field `T` makes it unwidenable.
        ctx.children_of(self.entity)
            .iter()
            .copied()
            .filter(|&child| {
                ctx.get::<NodeKind>(child) == Some(&NodeKind::Field)
                    && ctx
                        .get::<FieldClass>(child)
                        .is_some_and(FieldClass::is_stored_instance)
            })
            .find_map(|field| {
                let ty = ctx.query(LowerTypeAnnotation {
                    entity: field,
                    root: self.root,
                })?;
                relaxed_param_by_value(ctx, &ty, self.entity, self.root)
                    .map(|param| BoxDefect::FieldByValue { field, param })
            })
    }

    fn describe(&self) -> String {
        format!("ExistentialBoxDefect({:?})", self.entity)
    }
}
