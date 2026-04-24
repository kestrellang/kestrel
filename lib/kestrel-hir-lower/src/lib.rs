//! kestrel-hir-lower: AST → HIR lowering for the ECS-based compiler pipeline.
//!
//! Converts unresolved `AstBody` (arena-based AST) into resolved `HirBody`
//! (partially-resolved HIR). Responsibilities:
//!
//! - Resolve paths to entities or locals via name resolution queries
//! - Desugar operators to protocol calls
//! - Desugar for-loops, while, try/throw to basic control flow
//! - Expand type sugar (Array, Optional, etc.) to Named types
//! - Allocate local variable slots for params, let bindings, pattern bindings

mod ctx;
mod desugar;
mod expr;
pub mod literal;
pub(crate) mod pat;
mod stmt;
pub mod ty;

use kestrel_ast_builder::{Body, Callable};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::body::{HirBody, HirExpr, HirMatchArm, HirStmt, MatchSource};
use kestrel_span::Span;

pub use ty::{
    LowerCallableReturnType, LowerCallableTypes, LowerExtensionTargetTypeArgs, LowerTypeAnnotation,
    lower_ast_type,
};

use ctx::LowerCtx;

// ===== LowerBody query =====

/// Query: lower a declaration entity's AST body into HIR.
///
/// Reads the `Body(AstBody)` and `Callable` components from the entity,
/// creates local variable slots for parameters, and lowers all
/// statements/expressions into HIR form.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LowerBody {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for LowerBody {
    type Output = Option<HirBody>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<HirBody> {
        // Read the AST body component
        let body_component = ctx.get::<Body>(self.entity)?;
        let ast_body = &body_component.0;

        let mut lower = LowerCtx::new(ctx, self.root, self.entity);

        // Create locals for function parameters
        let mut param_desugar_stmts = Vec::new();
        if let Some(callable) = ctx.get::<Callable>(self.entity) {
            // If method has a receiver, create `self` local
            if let Some(receiver) = &callable.receiver {
                let is_mut = matches!(
                    receiver,
                    kestrel_ast_builder::ReceiverKind::Mutating
                        | kestrel_ast_builder::ReceiverKind::Consuming
                );
                let self_local = lower.define_local("self", is_mut, Span::synthetic(0));
                lower.params.push(self_local);
            }

            // Create locals for each parameter. For destructured params,
            // create a synthetic local and prepend a match-based destructure
            // that binds the pattern's variables in the body scope.
            for param in &callable.params {
                let local = lower.define_local(&param.name, param.is_mut, Span::synthetic(0));
                lower.params.push(local);

                // Desugar destructured params: match _param_0 { (a, b) => () }
                if let Some(ref pattern) = param.pattern {
                    let span = Span::synthetic(0);
                    let hir_pat = lower.lower_param_pattern(pattern, &span, param.is_mut);
                    let param_ref = lower.alloc_expr(HirExpr::Local(local, span.clone()));
                    let unit = lower.alloc_expr(HirExpr::Tuple {
                        elements: Vec::new(),
                        span: span.clone(),
                    });
                    let match_expr = lower.alloc_expr(HirExpr::Match {
                        scrutinee: param_ref,
                        arms: vec![HirMatchArm {
                            pattern: hir_pat,
                            guard: None,
                            body: unit,
                        }],
                        source: MatchSource::ParamDestructure,
                        span: span.clone(),
                    });
                    let stmt = lower.alloc_stmt(HirStmt::Expr {
                        expr: match_expr,
                        span: span.clone(),
                    });
                    param_desugar_stmts.push(stmt);
                }
            }
        }

        // Lower all top-level statements, prepending param destructure stmts
        let mut statements: Vec<_> = param_desugar_stmts;
        statements.extend(
            ast_body
                .statements
                .iter()
                .map(|&id| lower.lower_stmt(ast_body, id)),
        );

        // Lower tail expression
        let tail_expr = ast_body.tail_expr.map(|id| lower.lower_expr(ast_body, id));

        Some(HirBody {
            exprs: lower.exprs,
            pats: lower.pats,
            stmts: lower.stmts,
            locals: lower.locals,
            params: lower.params,
            statements,
            tail_expr,
            guard_let_stmts: lower.guard_let_stmts,
            while_conditions: lower.while_conditions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast::arena::Arena;
    use kestrel_ast::ast_body::*;
    use kestrel_ast_builder::{Name, NodeKind};
    use kestrel_hecs::World;
    use kestrel_hir::body::*;

    /// Helper: create a minimal world with a root module and a function entity.
    fn setup_with_body(ast_body: kestrel_ast::AstBody) -> (World, Entity, Entity) {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        let module = world.spawn();
        world.set(module, NodeKind::Module);
        world.set(module, Name("TestMod".into()));
        world.set_parent(module, root);

        let func = world.spawn();
        world.set(func, NodeKind::Function);
        world.set(func, Name("test_func".into()));
        world.set(func, Body(ast_body));
        world.set_parent(func, module);

        (world, root, func)
    }

    #[test]
    fn lower_empty_body() {
        let ast_body = kestrel_ast::AstBody {
            exprs: Arena::new(),
            pats: Arena::new(),
            stmts: Arena::new(),
            statements: Vec::new(),
            tail_expr: None,
        };

        let (world, root, func) = setup_with_body(ast_body);
        let ctx = world.query_context();
        let result = ctx.query(LowerBody { entity: func, root });

        let hir = result.expect("should produce HirBody");
        assert!(hir.statements.is_empty());
        assert!(hir.tail_expr.is_none());
        assert!(hir.params.is_empty());
    }

    #[test]
    fn lower_literal_tail_expr() {
        let mut exprs = Arena::new();
        let lit = exprs.alloc(AstExpr::Literal {
            kind: AstLiteral::Integer("42".into()),
            span: Span::synthetic(0),
        });

        let ast_body = kestrel_ast::AstBody {
            exprs,
            pats: Arena::new(),
            stmts: Arena::new(),
            statements: Vec::new(),
            tail_expr: Some(lit),
        };

        let (world, root, func) = setup_with_body(ast_body);
        let ctx = world.query_context();
        let hir = ctx.query(LowerBody { entity: func, root }).unwrap();

        assert!(hir.tail_expr.is_some());
        let expr = &hir.exprs[hir.tail_expr.unwrap()];
        assert!(matches!(
            expr,
            HirExpr::Literal {
                value: HirLiteral::Integer(42),
                ..
            }
        ));
    }

    #[test]
    fn lower_let_binding() {
        let mut exprs = Arena::new();
        let mut pats = Arena::new();
        let mut stmts = Arena::new();

        let value = exprs.alloc(AstExpr::Literal {
            kind: AstLiteral::Integer("10".into()),
            span: Span::synthetic(0),
        });

        let pat = pats.alloc(AstPat::Binding {
            is_mut: false,
            name: "x".into(),
            span: Span::synthetic(0),
        });

        let stmt = stmts.alloc(AstStmt::Let {
            is_mut: false,
            pattern: pat,
            ty: None,
            value: Some(value),
            span: Span::synthetic(0),
        });

        let ast_body = kestrel_ast::AstBody {
            exprs,
            pats,
            stmts,
            statements: vec![stmt],
            tail_expr: None,
        };

        let (world, root, func) = setup_with_body(ast_body);
        let ctx = world.query_context();
        let hir = ctx.query(LowerBody { entity: func, root }).unwrap();

        assert_eq!(hir.statements.len(), 1);
        assert_eq!(hir.locals.len(), 1); // one local: x
        assert_eq!(hir.locals[hir.locals.iter().next().unwrap().0].name, "x");
    }

    #[test]
    fn lower_function_params() {
        let ast_body = kestrel_ast::AstBody {
            exprs: Arena::new(),
            pats: Arena::new(),
            stmts: Arena::new(),
            statements: Vec::new(),
            tail_expr: None,
        };

        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        let func = world.spawn();
        world.set(func, NodeKind::Function);
        world.set(func, Name("add".into()));
        world.set(func, Body(ast_body));
        world.set(
            func,
            Callable {
                params: vec![
                    kestrel_ast_builder::AstParam {
                        label: None,
                        name: "a".into(),
                        ty: None,
                        default_entity: None,
                        pattern: None,
                        is_mut: false,
                        is_consuming: false,
                    },
                    kestrel_ast_builder::AstParam {
                        label: None,
                        name: "b".into(),
                        ty: None,
                        default_entity: None,
                        pattern: None,
                        is_mut: false,
                        is_consuming: false,
                    },
                ],
                receiver: None,
            },
        );
        world.set_parent(func, root);

        let ctx = world.query_context();
        let hir = ctx.query(LowerBody { entity: func, root }).unwrap();

        assert_eq!(hir.params.len(), 2);
        assert_eq!(hir.locals[hir.params[0]].name, "a");
        assert_eq!(hir.locals[hir.params[1]].name, "b");
    }

    #[test]
    fn lower_method_with_self() {
        let ast_body = kestrel_ast::AstBody {
            exprs: Arena::new(),
            pats: Arena::new(),
            stmts: Arena::new(),
            statements: Vec::new(),
            tail_expr: None,
        };

        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        let func = world.spawn();
        world.set(func, NodeKind::Function);
        world.set(func, Name("method".into()));
        world.set(func, Body(ast_body));
        world.set(
            func,
            Callable {
                params: Vec::new(),
                receiver: Some(kestrel_ast_builder::ReceiverKind::Borrowing),
            },
        );
        world.set_parent(func, root);

        let ctx = world.query_context();
        let hir = ctx.query(LowerBody { entity: func, root }).unwrap();

        // self + no explicit params = 1 param (self)
        assert_eq!(hir.params.len(), 1);
        assert_eq!(hir.locals[hir.params[0]].name, "self");
    }

    #[test]
    fn lower_if_expression() {
        let mut exprs = Arena::new();

        let cond = exprs.alloc(AstExpr::Literal {
            kind: AstLiteral::Bool(true),
            span: Span::synthetic(0),
        });
        let then_val = exprs.alloc(AstExpr::Literal {
            kind: AstLiteral::Integer("1".into()),
            span: Span::synthetic(0),
        });
        let else_val = exprs.alloc(AstExpr::Literal {
            kind: AstLiteral::Integer("2".into()),
            span: Span::synthetic(0),
        });

        let if_expr = exprs.alloc(AstExpr::If {
            conditions: vec![IfCondition::Expr(cond)],
            then_body: AstBlock {
                stmts: Vec::new(),
                tail_expr: Some(then_val),
            },
            else_body: Some(ElseBody::Block(AstBlock {
                stmts: Vec::new(),
                tail_expr: Some(else_val),
            })),
            span: Span::synthetic(0),
        });

        let ast_body = kestrel_ast::AstBody {
            exprs,
            pats: Arena::new(),
            stmts: Arena::new(),
            statements: Vec::new(),
            tail_expr: Some(if_expr),
        };

        let (world, root, func) = setup_with_body(ast_body);
        let ctx = world.query_context();
        let hir = ctx.query(LowerBody { entity: func, root }).unwrap();

        let tail = &hir.exprs[hir.tail_expr.unwrap()];
        assert!(matches!(tail, HirExpr::If { .. }));
    }

    #[test]
    fn lower_assignment() {
        let mut exprs = Arena::new();
        let mut pats = Arena::new();
        let mut stmts = Arena::new();

        // let x = 1
        let init_val = exprs.alloc(AstExpr::Literal {
            kind: AstLiteral::Integer("1".into()),
            span: Span::synthetic(0),
        });
        let pat = pats.alloc(AstPat::Binding {
            is_mut: true,
            name: "x".into(),
            span: Span::synthetic(0),
        });
        let let_stmt = stmts.alloc(AstStmt::Let {
            is_mut: true,
            pattern: pat,
            ty: None,
            value: Some(init_val),
            span: Span::synthetic(0),
        });

        // x = 2
        let lhs = exprs.alloc(AstExpr::Path {
            segments: vec![ExprPathSegment {
                name: "x".into(),
                type_args: None,
                span: Span::synthetic(0),
            }],
            span: Span::synthetic(0),
        });
        let rhs = exprs.alloc(AstExpr::Literal {
            kind: AstLiteral::Integer("2".into()),
            span: Span::synthetic(0),
        });
        let assign = exprs.alloc(AstExpr::Assignment {
            lhs,
            rhs,
            span: Span::synthetic(0),
        });
        let assign_stmt = stmts.alloc(AstStmt::Expr {
            expr: assign,
            span: Span::synthetic(0),
        });

        let ast_body = kestrel_ast::AstBody {
            exprs,
            pats,
            stmts,
            statements: vec![let_stmt, assign_stmt],
            tail_expr: None,
        };

        let (world, root, func) = setup_with_body(ast_body);
        let ctx = world.query_context();
        let hir = ctx.query(LowerBody { entity: func, root }).unwrap();

        assert_eq!(hir.statements.len(), 2);
        // The second statement should be an Assign to a local
        let stmt = &hir.stmts[hir.statements[1]];
        match stmt {
            HirStmt::Expr { expr, .. } => {
                let e = &hir.exprs[*expr];
                assert!(matches!(e, HirExpr::Assign { .. }));
            },
            _ => panic!("expected Expr stmt"),
        }
    }

    #[test]
    fn lower_no_body_returns_none() {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name("<root>".into()));

        // Entity without Body component
        let func = world.spawn();
        world.set(func, NodeKind::Function);
        world.set(func, Name("no_body".into()));
        world.set_parent(func, root);

        let ctx = world.query_context();
        let result = ctx.query(LowerBody { entity: func, root });
        assert!(result.is_none());
    }
}
