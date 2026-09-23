//! Where-clause entailment: "is constraint C provable from context Γ?"
//!
//! Used by callers (currently the conformance-completeness analyzer) that
//! need to know whether the where clauses on some declaration (e.g. a
//! protocol extension) are satisfied by the where clauses in scope at a
//! conformance site, without spinning up the full inference solver.
//!
//! This is the lightweight static-analysis cousin of `solve_conforms` in
//! `solver.rs`. Both must agree about what conformance means; here we
//! compose:
//!
//! - Direct context match — Γ contains `(p, P_ctx)` with `p == constraint.param`
//! - Refinement transitivity — `expand_protocol_closure([P_ctx])` contains
//!   `constraint.protocol`. Picks up protocol inheritance and
//!   extension-added conformances (e.g. `extend Equatable: Equal[Self]`).
//! - Param-declared bounds — `param_owner_where_clauses(constraint.param)`
//!   returns the bounds written on the param's enclosing decl. Mirrors
//!   `collect_param_protocol_bounds` in `resolve.rs`.
//!
//! Conservative on `TypeEquality` / `DirectEquality`: structural match
//! against context. Generalize when a real test demands.

use kestrel_hecs::{Entity, QueryContext};
use kestrel_name_res::expand_protocol_closure;

use crate::resolve::WhereClause;
use crate::where_clauses::param_owner_where_clauses;

/// True iff `constraint` is provable from `context` (plus any bounds
/// written on the constraint's subject param's owner decl).
pub fn constraint_entailed_by(
    qctx: &QueryContext<'_>,
    root: Entity,
    constraint: &WhereClause,
    context: &[WhereClause],
) -> bool {
    match constraint {
        // A projection or `Self`-rooted subject has no `as_param()`, so it
        // falls to `false` exactly as the old `ProjectionBound` variant did.
        // This is the one reader with *no* owner and *no* receiver — there is
        // nothing here that could name what `Self` denotes (D8 called this out
        // by name), so it cannot be entailed and the caller falls back.
        // TODO(G17 stage 3a): entailment for projection subjects.
        WhereClause::Bound {
            subject, protocol, ..
        } => match subject.as_param() {
            Some(param) => bound_entailed(qctx, root, param, *protocol, context),
            None => false,
        },
        // TypeEquality / DirectEquality carry HirTy on the RHS, which has
        // no structural equality. Reject conservatively until a real
        // caller demands proper handling — matches prior behavior in
        // `conformance_completeness::extension_where_clauses_satisfied`.
        WhereClause::TypeEquality { .. } | WhereClause::DirectEquality { .. } => false,
    }
}

/// No `Copyable` / `Cloneable` special case is needed here, and none should be
/// added. Unlike `conformance::type_satisfies`, this function never touches
/// `ConformingProtocols` — it does plain `Entity` containment over the
/// protocols named by `WhereClause::Bound` nodes, and `expand_protocol_closure`
/// seeds its output with the input set (`conformances.rs`). So a context clause
/// `where T: Copyable` matches a target clause `where T: Copyable` by protocol-
/// entity identity; the question "does `T` *declare* Copyable?" — the one
/// `ConformingProtocols` answers wrongly for structural builtins — is never
/// asked. That makes this path sound by construction, not incidentally.
/// Pasting a `type_satisfies`-style "skip the copy builtins" guard in here
/// would reopen the G13 hole: a `where T: Copyable` bound would stop gating.
fn bound_entailed(
    qctx: &QueryContext<'_>,
    root: Entity,
    param: Entity,
    protocol: Entity,
    context: &[WhereClause],
) -> bool {
    // 1. Direct or refinement-transitive match in context.
    let context_protocols: Vec<Entity> = context
        .iter()
        .filter_map(|c| match c {
            WhereClause::Bound {
                subject,
                protocol: cprot,
                ..
            } if subject.as_param() == Some(param) => Some(*cprot),
            _ => None,
        })
        .collect();
    if !context_protocols.is_empty()
        && expand_protocol_closure(qctx, root, context_protocols).contains(&protocol)
    {
        return true;
    }

    // 2. Bounds declared on the param's own enclosing decl (e.g. a struct
    //    or extension that wrote `where T: P`). Mirrors the resolver's
    //    `collect_param_protocol_bounds`. The clause lives on the param's
    //    OWNER, not on the `TypeParameter` entity; querying the param itself
    //    always came back empty and left this tier dead (G15 / A16).
    let param_bounds = param_owner_where_clauses(qctx, param, root);
    let param_protocols: Vec<Entity> = param_bounds
        .iter()
        .filter_map(|c| match c {
            WhereClause::Bound {
                subject,
                protocol: cprot,
                ..
            } if subject.as_param() == Some(param) => Some(*cprot),
            _ => None,
        })
        .collect();
    if !param_protocols.is_empty()
        && expand_protocol_closure(qctx, root, param_protocols).contains(&protocol)
    {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::WhereSubject;
    use kestrel_ast::{AstType, PathSegment};
    use kestrel_ast_builder::{
        ConformanceItem, Conformances, ExtensionTarget, Name, NodeKind, Typed, Vis,
    };
    use kestrel_hecs::World;
    use kestrel_span::Span;

    fn span() -> Span {
        Span::synthetic(0)
    }

    fn named(name: &str) -> AstType {
        AstType::Named {
            segments: vec![PathSegment {
                name: name.into(),
                type_args: vec![],
                span: span(),
            }],
            span: span(),
        }
    }

    fn fake_syntax() -> kestrel_syntax_tree::SyntaxNode {
        let mut b = kestrel_syntax_tree::GreenNodeBuilder::new();
        b.start_node(kestrel_syntax_tree::SyntaxKind::Root.into());
        b.finish_node();
        kestrel_syntax_tree::SyntaxNode::new_root(b.finish())
    }

    fn spawn_module(world: &mut World, parent: Option<Entity>, name: &str) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::Module);
        world.set(e, Name(name.into()));
        if let Some(p) = parent {
            world.set_parent(e, p);
        }
        e
    }

    fn spawn_protocol(world: &mut World, parent: Entity, name: &str) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::Protocol);
        world.set(e, Name(name.into()));
        world.set(e, Vis::Public);
        world.set(e, Typed);
        world.set_parent(e, parent);
        e
    }

    fn spawn_type_param(world: &mut World, parent: Entity, name: &str) -> Entity {
        let e = world.spawn();
        world.set(e, NodeKind::TypeParameter);
        world.set(e, Name(name.into()));
        world.set_parent(e, parent);
        e
    }

    #[test]
    fn direct_match_in_context() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, Name::ROOT);
        let p = spawn_protocol(&mut world, root, "P");
        let owner = spawn_module(&mut world, Some(root), "Owner");
        let t = spawn_type_param(&mut world, owner, "T");

        let context = vec![WhereClause::Bound {
            subject: WhereSubject::Param(t),
            protocol: p,
            protocol_type_args: vec![],
        }];
        let constraint = WhereClause::Bound {
            subject: WhereSubject::Param(t),
            protocol: p,
            protocol_type_args: vec![],
        };
        let ctx = world.query_context();
        assert!(constraint_entailed_by(&ctx, root, &constraint, &context));
    }

    #[test]
    fn refinement_transitivity_via_extension_added_conformance() {
        // Q: P  via `extend P: Q`. Context says T: P. Constraint asks T: Q.
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, Name::ROOT);
        let p = spawn_protocol(&mut world, root, "P");
        let q = spawn_protocol(&mut world, root, "Q");

        // extend P: Q
        let ext = world.spawn();
        world.set(ext, NodeKind::Extension);
        world.set(ext, ExtensionTarget(named("P")));
        world.set(
            ext,
            Conformances(vec![ConformanceItem::Positive(named("Q"), fake_syntax())]),
        );
        world.set_parent(ext, root);

        let owner = spawn_module(&mut world, Some(root), "Owner");
        let t = spawn_type_param(&mut world, owner, "T");

        let context = vec![WhereClause::Bound {
            subject: WhereSubject::Param(t),
            protocol: p,
            protocol_type_args: vec![],
        }];
        let constraint = WhereClause::Bound {
            subject: WhereSubject::Param(t),
            protocol: q,
            protocol_type_args: vec![],
        };
        let ctx = world.query_context();
        assert!(
            constraint_entailed_by(&ctx, root, &constraint, &context),
            "T: Q should hold via T: P + extend P: Q"
        );
    }

    #[test]
    fn unsatisfiable_when_no_path() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, Name::ROOT);
        let p = spawn_protocol(&mut world, root, "P");
        let q = spawn_protocol(&mut world, root, "Q");
        let owner = spawn_module(&mut world, Some(root), "Owner");
        let t = spawn_type_param(&mut world, owner, "T");

        let context = vec![WhereClause::Bound {
            subject: WhereSubject::Param(t),
            protocol: p,
            protocol_type_args: vec![],
        }];
        let constraint = WhereClause::Bound {
            subject: WhereSubject::Param(t),
            protocol: q,
            protocol_type_args: vec![],
        };
        let ctx = world.query_context();
        assert!(!constraint_entailed_by(&ctx, root, &constraint, &context));
    }

    /// Owner `Owner[T, U] where T: P` — the clause lives on the owner, the way
    /// the AST builder files every where clause.
    fn spawn_owner_with_clause(
        world: &mut World,
        root: Entity,
        subject: &str,
        protocol: &str,
    ) -> (Entity, Entity) {
        let owner = world.spawn();
        world.set(owner, NodeKind::Struct);
        world.set(owner, Name("Owner".into()));
        world.set(owner, Vis::Public);
        world.set_parent(owner, root);
        let t = spawn_type_param(world, owner, "T");
        let u = spawn_type_param(world, owner, "U");
        world.set(owner, kestrel_ast_builder::TypeParams(vec![t, u]));
        world.set(
            owner,
            kestrel_ast_builder::WhereClause(vec![kestrel_ast_builder::WhereConstraint::Bound {
                subject: named(subject),
                protocols: vec![named(protocol)],
                node: fake_syntax(),
            }]),
        );
        (t, u)
    }

    /// G15 / A16: the param-declared tier. A bound written on the param's
    /// OWNER entails the constraint with an empty context. Before the fix the
    /// tier queried `WhereClausesOf` on the `TypeParameter` itself, which is
    /// always empty, so this could never hold.
    #[test]
    fn bound_declared_on_param_owner_is_entailed() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, Name::ROOT);
        let p = spawn_protocol(&mut world, root, "P");
        let (t, _) = spawn_owner_with_clause(&mut world, root, "T", "P");

        let constraint = WhereClause::Bound {
            subject: WhereSubject::Param(t),
            protocol: p,
            protocol_type_args: vec![],
        };
        let ctx = world.query_context();
        assert!(constraint_entailed_by(&ctx, root, &constraint, &[]));
    }

    /// Control for the above: the owner's `T: P` says nothing about its
    /// sibling `U`, so the owner hop must not grant it.
    #[test]
    fn bound_on_sibling_param_is_not_entailed() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, Name::ROOT);
        let p = spawn_protocol(&mut world, root, "P");
        let (_, u) = spawn_owner_with_clause(&mut world, root, "T", "P");

        let constraint = WhereClause::Bound {
            subject: WhereSubject::Param(u),
            protocol: p,
            protocol_type_args: vec![],
        };
        let ctx = world.query_context();
        assert!(!constraint_entailed_by(&ctx, root, &constraint, &[]));
    }

    #[test]
    fn wrong_param_does_not_match() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, Name::ROOT);
        let p = spawn_protocol(&mut world, root, "P");
        let owner = spawn_module(&mut world, Some(root), "Owner");
        let t = spawn_type_param(&mut world, owner, "T");
        let u = spawn_type_param(&mut world, owner, "U");

        let context = vec![WhereClause::Bound {
            subject: WhereSubject::Param(t),
            protocol: p,
            protocol_type_args: vec![],
        }];
        let constraint = WhereClause::Bound {
            subject: WhereSubject::Param(u),
            protocol: p,
            protocol_type_args: vec![],
        };
        let ctx = world.query_context();
        assert!(!constraint_entailed_by(&ctx, root, &constraint, &context));
    }
}
