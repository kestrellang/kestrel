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
//! - Param-declared bounds — `WhereClausesOf(constraint.param)` returns
//!   bounds attached to the param's enclosing decl. Mirrors
//!   `collect_param_protocol_bounds` in `solver.rs`.
//!
//! Conservative on `TypeEquality` / `DirectEquality`: structural match
//! against context. Generalize when a real test demands.

use kestrel_hecs::{Entity, QueryContext};
use kestrel_name_res::expand_protocol_closure;

use crate::resolve::WhereClause;
use crate::where_clauses::WhereClausesOf;

/// True iff `constraint` is provable from `context` (plus any bounds
/// attached to the constraint's subject param via `WhereClausesOf`).
pub fn constraint_entailed_by(
    qctx: &QueryContext<'_>,
    root: Entity,
    constraint: &WhereClause,
    context: &[WhereClause],
) -> bool {
    match constraint {
        WhereClause::Bound {
            param, protocol, ..
        } => bound_entailed(qctx, root, *param, *protocol, context),
        // TypeEquality / DirectEquality carry HirTy on the RHS, which has
        // no structural equality. Reject conservatively until a real
        // caller demands proper handling — matches prior behavior in
        // `conformance_completeness::extension_where_clauses_satisfied`.
        WhereClause::TypeEquality { .. } | WhereClause::DirectEquality { .. } => false,
    }
}

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
                param: cp,
                protocol: cprot,
                ..
            } if *cp == param => Some(*cprot),
            _ => None,
        })
        .collect();
    if !context_protocols.is_empty()
        && expand_protocol_closure(qctx, root, context_protocols).contains(&protocol)
    {
        return true;
    }

    // 2. Bounds declared on the param's own enclosing decl (e.g. a struct
    //    or extension that wrote `where T: P`). Mirrors the solver's
    //    `collect_param_protocol_bounds`.
    let param_bounds = qctx.query(WhereClausesOf {
        entity: param,
        root,
    });
    let param_protocols: Vec<Entity> = param_bounds
        .iter()
        .filter_map(|c| match c {
            WhereClause::Bound {
                param: cp,
                protocol: cprot,
                ..
            } if *cp == param => Some(*cprot),
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
        let root = spawn_module(&mut world, None, "<root>");
        let p = spawn_protocol(&mut world, root, "P");
        let owner = spawn_module(&mut world, Some(root), "Owner");
        let t = spawn_type_param(&mut world, owner, "T");

        let context = vec![WhereClause::Bound {
            param: t,
            protocol: p,
            protocol_type_args: vec![],
        }];
        let constraint = WhereClause::Bound {
            param: t,
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
        let root = spawn_module(&mut world, None, "<root>");
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
            param: t,
            protocol: p,
            protocol_type_args: vec![],
        }];
        let constraint = WhereClause::Bound {
            param: t,
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
        let root = spawn_module(&mut world, None, "<root>");
        let p = spawn_protocol(&mut world, root, "P");
        let q = spawn_protocol(&mut world, root, "Q");
        let owner = spawn_module(&mut world, Some(root), "Owner");
        let t = spawn_type_param(&mut world, owner, "T");

        let context = vec![WhereClause::Bound {
            param: t,
            protocol: p,
            protocol_type_args: vec![],
        }];
        let constraint = WhereClause::Bound {
            param: t,
            protocol: q,
            protocol_type_args: vec![],
        };
        let ctx = world.query_context();
        assert!(!constraint_entailed_by(&ctx, root, &constraint, &context));
    }

    #[test]
    fn wrong_param_does_not_match() {
        let mut world = World::new();
        world.begin_revision();
        let root = spawn_module(&mut world, None, "<root>");
        let p = spawn_protocol(&mut world, root, "P");
        let owner = spawn_module(&mut world, Some(root), "Owner");
        let t = spawn_type_param(&mut world, owner, "T");
        let u = spawn_type_param(&mut world, owner, "U");

        let context = vec![WhereClause::Bound {
            param: t,
            protocol: p,
            protocol_type_args: vec![],
        }];
        let constraint = WhereClause::Bound {
            param: u,
            protocol: p,
            protocol_type_args: vec![],
        };
        let ctx = world.query_context();
        assert!(!constraint_entailed_by(&ctx, root, &constraint, &context));
    }
}
