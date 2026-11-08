//! Existential function tables: which protocol requirements can be called
//! through `any P` (existentials design §5).
//!
//! A requirement is **dispatchable** when a call through an existential can
//! reach the payload's witness with nothing but the receiver: it is an
//! instance member, not generic, and mentions `Self` only as the receiver.
//! A requirement mentioning an associated type is dispatchable only from an
//! existential that pins that type (`any Iterator[Item = Int64]`), so its
//! entry lists the types it needs and the caller compares.
//!
//! Both queries depend only on declarations, so editing a body never
//! invalidates them. Type inference reads them for diagnostics; MIR reads
//! [`ExistentialLayout`] for slot order. Nobody else recomputes either.

use std::sync::Arc;

use kestrel_ast_builder::{NodeKind, Static, TypeParams};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::HirTy;
use kestrel_hir_lower::{LowerCallableReturnType, LowerCallableTypes, LowerTypeAnnotation};
use kestrel_name_res::{ConformingProtocolInstantiations, ConformingProtocols, ProtocolMembers};

/// Can this requirement be called through an existential?
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Dispatch {
    /// Yes, given that the existential pins every associated type listed
    /// (empty for most requirements).
    Dispatchable { assocs: Vec<Entity> },
    /// A `static` member or an initializer: there is no receiver.
    Static,
    /// Generic requirements need a witness per instantiation.
    Generic,
    /// `Self` appears outside the receiver, so the caller can't supply it.
    UsesSelf,
}

impl Dispatch {
    /// Callable from an existential that pins `bound` associated types?
    pub fn allowed(&self, bound: &[Entity]) -> bool {
        matches!(self, Dispatch::Dispatchable { assocs } if assocs.iter().all(|a| bound.contains(a)))
    }
}

/// Query: [`Dispatch`] for one protocol requirement.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RequirementDispatch {
    pub requirement: Entity,
    pub root: Entity,
}

impl QueryFn for RequirementDispatch {
    type Output = Dispatch;

    fn execute(&self, ctx: &QueryContext<'_>) -> Dispatch {
        let r = self.requirement;
        if ctx.get::<Static>(r).is_some() || ctx.get::<NodeKind>(r) == Some(&NodeKind::Initializer)
        {
            return Dispatch::Static;
        }
        if ctx.get::<TypeParams>(r).is_some_and(|tp| !tp.0.is_empty()) {
            return Dispatch::Generic;
        }
        let mut assocs = Vec::new();
        for ty in signature_types(ctx, r, self.root) {
            if mentions_self(&ty, &mut assocs) {
                return Dispatch::UsesSelf;
            }
        }
        assocs.sort_by_key(|a| a.index());
        assocs.dedup();
        Dispatch::Dispatchable { assocs }
    }

    fn describe(&self) -> String {
        format!("RequirementDispatch({:?})", self.requirement)
    }
}

/// A requirement's parameter and result types (a property's type).
fn signature_types(ctx: &QueryContext<'_>, entity: Entity, root: Entity) -> Vec<HirTy> {
    if ctx.get::<NodeKind>(entity) == Some(&NodeKind::Field) {
        return ctx
            .query(LowerTypeAnnotation { entity, root })
            .into_iter()
            .collect();
    }
    let mut out: Vec<HirTy> = ctx
        .query(LowerCallableTypes { entity, root })
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .collect();
    out.push(ctx.query(LowerCallableReturnType { entity, root }));
    out
}

/// Does `ty` use `Self` other than through an associated type? Associated
/// types reached from `Self` (`Self.Item`) are collected into `assocs`.
fn mentions_self(ty: &HirTy, assocs: &mut Vec<Entity>) -> bool {
    match ty {
        HirTy::SelfType(..) => true,
        HirTy::AssocProjection { base, assoc, .. } => match base.as_ref() {
            HirTy::SelfType(..) => {
                assocs.push(*assoc);
                false
            },
            other => mentions_self(other, assocs),
        },
        HirTy::Struct { args, .. }
        | HirTy::Enum { args, .. }
        | HirTy::Protocol { args, .. }
        | HirTy::Tuple(args, _) => args.iter().any(|a| mentions_self(a, assocs)),
        HirTy::Alias { expanded, .. } => mentions_self(expanded, assocs),
        HirTy::Function { params, ret, .. } => {
            params.iter().any(|p| mentions_self(p, assocs)) || mentions_self(ret, assocs)
        },
        HirTy::Ref { inner, .. } => mentions_self(inner, assocs),
        HirTy::RawExistential {
            bounds, bindings, ..
        } => {
            bounds.iter().any(|b| mentions_self(b, assocs))
                || bindings.iter().any(|b| mentions_self(&b.ty, assocs))
        },
        HirTy::Opaque { bounds, .. } => bounds.iter().any(|b| mentions_self(b, assocs)),
        HirTy::Param(..)
        | HirTy::AbstractAssoc { .. }
        | HirTy::Never(_)
        | HirTy::Infer(_)
        | HirTy::Error(_) => false,
    }
}

/// The function-table shape of one protocol.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Layout {
    /// The protocol's own requirements, in declaration order, with whether
    /// each can be called through an existential. A table holds a slot for
    /// every `Dispatchable` entry, in this order; parents' requirements live
    /// in the parents' tables.
    pub requirements: Vec<(Entity, Dispatch)>,
    /// Directly refined protocols (one parent-table pointer each).
    pub parents: Vec<Entity>,
}

impl Layout {
    /// The dispatchable requirements: the table's slots, in order.
    pub fn slots(&self) -> impl Iterator<Item = Entity> + '_ {
        self.requirements
            .iter()
            .filter(|(_, d)| matches!(d, Dispatch::Dispatchable { .. }))
            .map(|(e, _)| *e)
    }
}

/// Query: the [`Layout`] of `protocol`'s function table.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExistentialLayout {
    pub protocol: Entity,
    pub root: Entity,
}

impl QueryFn for ExistentialLayout {
    type Output = Arc<Layout>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Arc<Layout> {
        let members = ctx.query(ProtocolMembers {
            protocol: self.protocol,
            root: self.root,
        });
        let requirements = members
            .iter()
            .filter(|m| m.extension.is_none() && m.declaring_protocol == self.protocol)
            .map(|m| {
                let dispatch = ctx.query(RequirementDispatch {
                    requirement: m.entity,
                    root: self.root,
                });
                (m.entity, dispatch)
            })
            .collect();
        let parents = ctx
            .query(ConformingProtocols {
                entity: self.protocol,
                root: self.root,
            })
            .iter()
            .copied()
            .filter(|&p| p != self.protocol)
            .collect();
        Arc::new(Layout {
            requirements,
            parents,
        })
    }

    fn describe(&self) -> String {
        format!("ExistentialLayout({:?})", self.protocol)
    }
}

/// Is `member` a protocol requirement (declared in a protocol body), as
/// opposed to an extension method? Extension methods reach an existential
/// through self-conformance, not the table.
pub fn is_requirement(ctx: &QueryContext<'_>, member: Entity) -> bool {
    ctx.parent_of(member)
        .is_some_and(|owner| ctx.get::<NodeKind>(owner) == Some(&NodeKind::Protocol))
}

/// What `any P: P` needs (existentials design §6), for one protocol and its
/// parents.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SelfConformance {
    /// The first requirement that can't go through the table, if any. The
    /// conformance fails when this is `Some`.
    pub blocker: Option<Entity>,
    /// Some requirement takes `mutating self`: the box must be
    /// `MutableIndirection`.
    pub needs_mutable: bool,
    /// Some requirement takes `consuming self`: the box must be
    /// `ConsumingIndirection`.
    pub needs_consuming: bool,
}

/// [`SelfConformance`] of `protocol` for an existential that pins the
/// associated types in `bound`.
pub fn self_conformance(
    ctx: &QueryContext<'_>,
    protocol: Entity,
    bound: &[Entity],
    root: Entity,
) -> SelfConformance {
    use kestrel_ast_builder::{Callable, ReceiverKind};
    let mut out = SelfConformance {
        blocker: None,
        needs_mutable: false,
        needs_consuming: false,
    };
    // Declared parents only: a protocol added by a blanket extension is
    // satisfied by that extension's members, not through the table.
    for p in bound_closure(ctx, &[protocol], root) {
        let layout = ctx.query(ExistentialLayout { protocol: p, root });
        for (requirement, dispatch) in &layout.requirements {
            if !dispatch.allowed(bound) {
                out.blocker.get_or_insert(*requirement);
            }
            match ctx
                .get::<Callable>(*requirement)
                .and_then(|c| c.receiver.clone())
            {
                Some(ReceiverKind::Mutating) => out.needs_mutable = true,
                Some(ReceiverKind::Consuming) => out.needs_consuming = true,
                _ => {},
            }
        }
    }
    out
}

/// The bounds and every protocol they refine, through declared `: Parent`
/// edges only. A protocol added by a blanket `extend Q: R` is not in it: an
/// existential gets `R` through `Q`, not by checking `R`'s requirements.
pub fn bound_closure(ctx: &QueryContext<'_>, bounds: &[Entity], root: Entity) -> Vec<Entity> {
    let mut out: Vec<Entity> = bounds.to_vec();
    let edges: Vec<(Entity, Entity)> = bounds
        .iter()
        .flat_map(|&b| {
            ctx.query(ConformingProtocolInstantiations { entity: b, root })
                .to_vec()
        })
        .filter(|(_, source, _)| ctx.get::<NodeKind>(*source) == Some(&NodeKind::Protocol))
        .map(|(proto, source, _)| (proto, source))
        .collect();
    // Fixpoint: a parent joins once the protocol declaring the edge has.
    loop {
        let before = out.len();
        for &(proto, source) in &edges {
            if out.contains(&source) && !out.contains(&proto) {
                out.push(proto);
            }
        }
        if out.len() == before {
            return out;
        }
    }
}

/// Every protocol an existential with these bounds conforms to through
/// some path: declared refinement or a blanket protocol extension.
pub fn reachable_protocols(ctx: &QueryContext<'_>, bounds: &[Entity], root: Entity) -> Vec<Entity> {
    let mut out: Vec<Entity> = bounds.to_vec();
    for &b in bounds {
        for &p in ctx.query(ConformingProtocols { entity: b, root }).iter() {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}
