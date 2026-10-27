//! Which enum cases reach their own enum **by value** — the single source of
//! truth for both "recursive enum requires `indirect`" (E429) and which cases
//! of an `indirect enum` are boxed (MIR `enum_lower`, copy semantics).
//!
//! The walk follows a case's resolved payload types through everything stored
//! inline: tuples, the stored instance fields of structs and the case payloads
//! of enums, with each nominal's type arguments substituted (so
//! `case Cons(Int64, Optional[List])` reaches `List` through `Optional.Some`).
//! It stops at everything held behind a pointer or not stored at all: type
//! params, functions, existentials, opaque types, references, and builtin
//! storage like `lang.ptr[T]` (which has no stored fields).
//!
//! Whether to walk *through* other `indirect` enums is the caller's choice
//! ([`RecursiveCases::through_indirect`]):
//! - **E429** (non-indirect enums) stops at them: any cycle through an
//!   indirect enum's case also makes that case reach its own enum, so that
//!   case is boxed and breaks the cycle.
//! - **Boxing** (indirect enums) walks through them by value. Otherwise
//!   `indirect enum A { case X(B) }` + `indirect enum B { case Y(A) }` boxes
//!   neither case and the layout is infinite; walking through boxes both (one
//!   extra allocation, always sound).

use std::collections::HashSet;

use kestrel_ast_builder::{FieldClass, IsIndirect, NodeKind, TypeParams};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::HirTy;
use kestrel_hir_lower::{LowerCallableTypes, LowerTypeAnnotation};

/// Bound on nested nominal expansions along one path. Regular types never get
/// near it; it only cuts off polymorphic recursion through *other* types
/// (`struct S[T] { var s: Wrapper[S[[T]]] }`), which is an infinite type and
/// reported elsewhere.
const MAX_DEPTH: usize = 64;

/// One case of an enum whose payload reaches the enum itself by value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RecursiveCase {
    /// The `EnumCase` entity.
    pub case: Entity,
    /// A payload type names the enum directly (`case Node(Tree, Tree)`), as
    /// opposed to through another type.
    pub direct: bool,
    /// Every reference back to the enum passes its own type params unchanged
    /// (`List[T]` inside `List[T]`). `false` is polymorphic recursion
    /// (`case Nest(Nested[[T]])`): boxing it would need an unbounded set of
    /// instantiations.
    pub regular: bool,
}

/// The cases of `entity` (an enum) that reach `entity` by value, in
/// declaration order. Empty for a non-recursive enum or a non-enum.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RecursiveCases {
    pub entity: Entity,
    pub root: Entity,
    /// Walk into other `indirect` enums by value (boxing) or stop at them
    /// (E429). See the module docs.
    pub through_indirect: bool,
}

impl QueryFn for RecursiveCases {
    type Output = Vec<RecursiveCase>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<RecursiveCase> {
        let own_params: Vec<Entity> = ctx
            .get::<TypeParams>(self.entity)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        let mut out = Vec::new();
        for &case in ctx.children_of(self.entity) {
            if ctx.get::<NodeKind>(case) != Some(&NodeKind::EnumCase) {
                continue;
            }
            let payload = case_payload_types(ctx, case, self.root);
            let mut walk = Walk {
                ctx,
                root: self.root,
                target: self.entity,
                own_params: &own_params,
                through_indirect: self.through_indirect,
                reached: false,
                regular: true,
                visited: HashSet::new(),
            };
            for ty in &payload {
                walk.ty(ty, 0);
            }
            if walk.reached {
                out.push(RecursiveCase {
                    case,
                    direct: payload.iter().any(|ty| names_directly(ty, self.entity)),
                    regular: walk.regular,
                });
            }
        }
        out
    }

    fn describe(&self) -> String {
        format!("RecursiveCases({:?})", self.entity)
    }
}

/// The cases of an `indirect enum` stored behind a box: those that reach the
/// enum by value. Empty for a non-indirect enum — the single answer MIR
/// lowering and copy semantics share.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct IndirectCases {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for IndirectCases {
    type Output = Vec<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<Entity> {
        if !ctx.has::<IsIndirect>(self.entity) {
            return Vec::new();
        }
        ctx.query(RecursiveCases {
            entity: self.entity,
            root: self.root,
            through_indirect: true,
        })
        .into_iter()
        .map(|c| c.case)
        .collect()
    }

    fn describe(&self) -> String {
        format!("IndirectCases({:?})", self.entity)
    }
}

/// A case's declared payload types (unresolvable ones dropped).
pub fn case_payload_types(ctx: &QueryContext<'_>, case: Entity, root: Entity) -> Vec<HirTy> {
    ctx.query(LowerCallableTypes { entity: case, root })
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .collect()
}

/// Does `ty` name `target` itself, or as a top-level tuple element?
fn names_directly(ty: &HirTy, target: Entity) -> bool {
    match ty {
        HirTy::Enum { entity, .. } => *entity == target,
        HirTy::Tuple(elems, _) => elems.iter().any(|e| names_directly(e, target)),
        _ => false,
    }
}

struct Walk<'a, 'q> {
    ctx: &'a QueryContext<'q>,
    root: Entity,
    target: Entity,
    own_params: &'a [Entity],
    through_indirect: bool,
    reached: bool,
    regular: bool,
    /// Nominal instances already expanded, keyed by entity + canonical args.
    visited: HashSet<(Entity, String)>,
}

impl Walk<'_, '_> {
    fn ty(&mut self, ty: &HirTy, depth: usize) {
        match ty {
            HirTy::Enum { entity, args, .. } if *entity == self.target => {
                self.reached = true;
                if !self.is_own_params(args) {
                    self.regular = false;
                }
            },
            HirTy::Struct { entity, args, .. } => self.nominal(*entity, args, depth),
            HirTy::Enum { entity, args, .. } => {
                if self.through_indirect || !self.ctx.has::<IsIndirect>(*entity) {
                    self.nominal(*entity, args, depth);
                }
            },
            HirTy::AliasUse { entity, args, .. } => {
                if let Some(rhs) = self.ctx.query(LowerTypeAnnotation {
                    entity: *entity,
                    root: self.root,
                }) {
                    let rhs = substitute(&rhs, &params_of(self.ctx, *entity), args);
                    self.ty(&rhs, depth + 1);
                }
            },
            HirTy::Tuple(elems, _) => {
                for elem in elems {
                    self.ty(elem, depth);
                }
            },
            // Behind a pointer, or no storage of a known type.
            HirTy::Protocol { .. }
            | HirTy::Function { .. }
            | HirTy::Param(..)
            | HirTy::SelfType(..)
            | HirTy::AssocProjection { .. }
            | HirTy::Opaque { .. }
            | HirTy::RawExistential { .. }
            | HirTy::Never(_)
            | HirTy::Infer(_)
            | HirTy::Error(_)
            | HirTy::Ref { .. } => {},
        }
    }

    /// Expand a struct's stored fields or an enum's case payloads, with
    /// `args` substituted for the nominal's own params.
    fn nominal(&mut self, entity: Entity, args: &[HirTy], depth: usize) {
        if depth >= MAX_DEPTH || !self.visited.insert((entity, canonical_args(args))) {
            return;
        }
        let params = params_of(self.ctx, entity);
        for member in stored_member_types(self.ctx, entity, self.root) {
            let member = substitute(&member, &params, args);
            self.ty(&member, depth + 1);
        }
    }

    fn is_own_params(&self, args: &[HirTy]) -> bool {
        args.len() == self.own_params.len()
            && args
                .iter()
                .zip(self.own_params)
                .all(|(arg, p)| matches!(arg, HirTy::Param(e, _) if e == p))
    }
}

/// Types stored inline in an instance of `entity`: a struct's stored instance
/// fields, or every enum case's payload.
fn stored_member_types(ctx: &QueryContext<'_>, entity: Entity, root: Entity) -> Vec<HirTy> {
    let mut out = Vec::new();
    for &child in ctx.children_of(entity) {
        match ctx.get::<NodeKind>(child) {
            Some(NodeKind::Field)
                if ctx
                    .get::<FieldClass>(child)
                    .is_some_and(FieldClass::is_stored_instance) =>
            {
                out.extend(ctx.query(LowerTypeAnnotation {
                    entity: child,
                    root,
                }));
            },
            Some(NodeKind::EnumCase) => out.extend(case_payload_types(ctx, child, root)),
            _ => {},
        }
    }
    out
}

fn params_of(ctx: &QueryContext<'_>, entity: Entity) -> Vec<Entity> {
    ctx.get::<TypeParams>(entity)
        .map(|tp| tp.0.clone())
        .unwrap_or_default()
}

/// Replace each `Param(params[i])` in `ty` with `args[i]`.
fn substitute(ty: &HirTy, params: &[Entity], args: &[HirTy]) -> HirTy {
    if params.is_empty() || args.is_empty() {
        return ty.clone();
    }
    let sub = |t: &HirTy| substitute(t, params, args);
    let subs = |ts: &[HirTy]| ts.iter().map(sub).collect::<Vec<_>>();
    match ty {
        HirTy::Param(e, _) => match params.iter().position(|p| p == e) {
            Some(i) if i < args.len() => args[i].clone(),
            _ => ty.clone(),
        },
        HirTy::Struct {
            entity,
            args: a,
            span,
        } => HirTy::Struct {
            entity: *entity,
            args: subs(a),
            span: span.clone(),
        },
        HirTy::Enum {
            entity,
            args: a,
            span,
        } => HirTy::Enum {
            entity: *entity,
            args: subs(a),
            span: span.clone(),
        },
        HirTy::AliasUse {
            entity,
            args: a,
            span,
        } => HirTy::AliasUse {
            entity: *entity,
            args: subs(a),
            span: span.clone(),
        },
        HirTy::Tuple(elems, span) => HirTy::Tuple(subs(elems), span.clone()),
        // Nothing below these is stored inline, so the walk never looks in.
        _ => ty.clone(),
    }
}

/// A span-free rendering of type args, for the visited set.
fn canonical_args(args: &[HirTy]) -> String {
    fn canon(ty: &HirTy, out: &mut String) {
        match ty {
            HirTy::Struct { entity, args, .. }
            | HirTy::Enum { entity, args, .. }
            | HirTy::AliasUse { entity, args, .. } => {
                out.push_str(&format!("{entity:?}["));
                for a in args {
                    canon(a, out);
                    out.push(',');
                }
                out.push(']');
            },
            HirTy::Tuple(elems, _) => {
                out.push('(');
                for e in elems {
                    canon(e, out);
                    out.push(',');
                }
                out.push(')');
            },
            HirTy::Param(e, _) => out.push_str(&format!("P{e:?}")),
            _ => out.push('_'),
        }
    }
    let mut out = String::new();
    for a in args {
        canon(a, &mut out);
        out.push(';');
    }
    out
}
