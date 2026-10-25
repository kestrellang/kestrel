//! kestrel-hir-lower: body lowering, CST → HIR.
//!
//! Lowers a declaration's body straight from its syntax (the typed views of
//! `kestrel-syntax-tree`) into a resolved `HirBody`, on demand, and records a
//! [`BodySourceMap`] linking the two. Responsibilities:
//!
//! - Resolve paths to entities or locals via name resolution queries
//! - Desugar operators to protocol calls
//! - Desugar for-loops, while, try/throw to basic control flow
//! - Expand type sugar (Array, Optional, etc.) to Named types
//! - Allocate local variable slots for params, let bindings, pattern bindings

mod ctx;
mod desugar;
mod expr;
pub mod format_spec;
pub mod literal;
pub(crate) mod pat;
pub mod source_map;
mod stmt;
pub(crate) mod string_token;
pub(crate) mod syntax;
pub mod ty;

use std::sync::Arc;

use kestrel_ast_builder::{Callable, DefaultReferencesParam, FileId, Valued};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::body::{HirBody, HirExpr, HirMatchArm, HirStmt, MatchSource};
use kestrel_span::Span;
use kestrel_syntax_tree::ast::{self, AstNode};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode, SyntaxToken};

pub use source_map::{BodySourceMap, LocalSource};
pub use ty::{
    CallableRefReturn, LowerCallableReturnType, LowerCallableTypes, LowerExtensionTargetTypeArgs,
    LowerTypeAnnotation, PlaceAccessorInfo, PlaceAccessors, RefPolicy, RefPosition, RefReturn,
    lower_ast_type, reject_ref_types, reject_ref_types_allowing_top_ref,
};

use ctx::LowerCtx;
use syntax::{BlockSyntax, ExprSrc, block_syntax, first_expr, is_expr_like};

// ===== LowerBody query =====

/// Query: a declaration entity's body, lowered to HIR.
///
/// The body is the syntax the entity's `Valued` component points at. Its
/// diagnostics are filed by [`LowerBodyWithSourceMap`], which this projects.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LowerBody {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for LowerBody {
    // Arc-wrapped: HirBody is large and widely re-queried; memo cache hits
    // clone the Output, so share one allocation instead of deep-copying.
    type Output = Option<Arc<HirBody>>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Arc<HirBody>> {
        let lowered = ctx.query(LowerBodyWithSourceMap {
            entity: self.entity,
            root: self.root,
        })?;
        Some(lowered.body.clone())
    }
}

/// A lowered body and its source map. Both are `Send + Sync`: the map holds
/// syntax *pointers*, resolved against the file's tree on demand.
#[derive(Clone, Debug, Hash)]
pub struct LoweredBody {
    pub body: Arc<HirBody>,
    pub source_map: Arc<BodySourceMap>,
}

/// Query: lower an entity's body and record its [`BodySourceMap`].
///
/// Reads the body syntax (`Valued`, resolved against the file's
/// `FileSyntax`) and the `Callable` signature, creates local slots for the
/// receiver and parameters, and lowers every statement and expression.
/// `None` when the entity has no body.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LowerBodyWithSourceMap {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for LowerBodyWithSourceMap {
    type Output = Option<Arc<LoweredBody>>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Arc<LoweredBody>> {
        ctx.get::<Valued>(self.entity)?;
        let node = kestrel_ast_builder::syntax::valued_node(ctx, self.entity)?;
        let file = ctx.get::<FileId>(self.entity).map_or(self.entity, |f| f.0);
        let file_id = file.index();

        // A parameter default that names a sibling parameter is reported by
        // the analyzer (it carries `DefaultReferencesParam`); its body lowers
        // empty so inference adds no "undefined name" on top.
        let block = if ctx.get::<DefaultReferencesParam>(self.entity).is_some() {
            BlockSyntax::empty()
        } else {
            body_syntax(&node, file_id)
        };

        let mut lower = LowerCtx::new(ctx, self.root, self.entity, file_id);
        let param_desugar_stmts = lower.define_params();

        // Lower all top-level statements via lower_block_stmts so that
        // guard-let CPS transformation applies at the function body level.
        let body_block = lower.lower_block_stmts(&block.stmts, block.tail.as_ref());
        let mut statements: Vec<_> = param_desugar_stmts;
        statements.extend(body_block.stmts);

        // Lower tail expression.
        // For effectful inits, wrap the tail (or synthesize one) in .Some(()) / .Ok(())
        // so the implicit fall-through returns the success wrapper around unit.
        let tail_expr = if let Some(lowered) = body_block.tail_expr {
            if let Some(wrapped) = lower.wrap_init_success_value(Span::synthetic(0)) {
                // Effectful init: emit the original tail as a statement, return the wrapper
                let stmt = lower.alloc_stmt(HirStmt::Expr {
                    expr: lowered,
                    span: Span::synthetic(0),
                });
                statements.push(stmt);
                Some(wrapped)
            } else {
                Some(lowered)
            }
        } else {
            lower.wrap_init_success_value(Span::synthetic(0))
        };

        let body = HirBody {
            exprs: lower.exprs,
            pats: lower.pats,
            stmts: lower.stmts,
            locals: lower.locals,
            params: lower.params,
            statements,
            tail_expr,
            guard_stmts: lower.guard_stmts,
            while_conditions: lower.while_conditions,
        };
        Some(Arc::new(LoweredBody {
            body: Arc::new(body),
            source_map: Arc::new(lower.source_map),
        }))
    }
}

/// The statements and value of a body node: a `{ … }` block, a function's
/// `= expr` (`FunctionBody`), a parameter default's `= expr`
/// (`DefaultValue`), or a field initializer (a bare `Expression`).
fn body_syntax(node: &SyntaxNode, file_id: usize) -> BlockSyntax {
    match node.kind() {
        SyntaxKind::CodeBlock => block_syntax(node, file_id),
        SyntaxKind::FunctionBody => {
            match node.children().find(|c| c.kind() == SyntaxKind::CodeBlock) {
                Some(block) => block_syntax(&block, file_id),
                None => expr_body(first_expr(node)),
            }
        },
        SyntaxKind::DefaultValue => expr_body(first_expr(node)),
        kind if is_expr_like(kind) => expr_body(Some(node.clone())),
        _ => BlockSyntax::empty(),
    }
}

/// A body that is a single expression.
fn expr_body(expr: Option<SyntaxNode>) -> BlockSyntax {
    BlockSyntax {
        stmts: Vec::new(),
        tail: expr.map(ExprSrc::Node),
    }
}

impl LowerCtx<'_> {
    /// Locals for the receiver and parameters, in order. A destructured
    /// parameter gets a synthetic local plus a match that binds the pattern's
    /// variables in the body scope; those matches are returned as the body's
    /// leading statements.
    fn define_params(&mut self) -> Vec<kestrel_hir::body::HirStmtId> {
        let mut param_desugar_stmts = Vec::new();
        let Some(callable) = self.ctx.get::<Callable>(self.owner) else {
            return param_desugar_stmts;
        };
        // If method has a receiver, create `self` local
        if let Some(receiver) = &callable.receiver {
            let is_mut = matches!(
                receiver,
                kestrel_ast_builder::ReceiverKind::Mutating
                    | kestrel_ast_builder::ReceiverKind::Consuming
            );
            let self_local = self.define_local("self", is_mut, Span::synthetic(0));
            self.params.push(self_local);
        }

        let mut named = self.signature_param_names();
        for param in &callable.params {
            let local = self.define_local(&param.name, param.is_mut, Span::synthetic(0));
            self.params.push(local);
            // Record where a plainly-named parameter is spelled.
            if param.pattern.is_none()
                && let Some(slot) = named
                    .iter_mut()
                    .find(|n| n.as_ref().is_some_and(|(_, t)| t.text() == param.name))
                && let Some((binding, name)) = slot.take()
            {
                self.source_map.record_local(local, &binding, &name);
            }

            // Desugar destructured params: match _param_0 { (a, b) => () }
            if let Some(ref pattern) = param.pattern {
                let span = Span::synthetic(0);
                let hir_pat = self.lower_param_pattern(pattern, &span, param.is_mut);
                let param_ref = self.alloc_expr(HirExpr::Local(local, span.clone()));
                let unit = self.alloc_expr(HirExpr::Tuple {
                    elements: Vec::new(),
                    span: span.clone(),
                });
                let match_expr = self.alloc_expr(HirExpr::Match {
                    scrutinee: param_ref,
                    arms: vec![HirMatchArm {
                        pattern: hir_pat,
                        guard: None,
                        body: unit,
                    }],
                    source: MatchSource::ParamDestructure,
                    span: span.clone(),
                });
                let stmt = self.alloc_stmt(HirStmt::Expr {
                    expr: match_expr,
                    span: span.clone(),
                });
                param_desugar_stmts.push(stmt);
            }
        }
        param_desugar_stmts
    }

    /// The binding pattern and identifier of each plainly-named parameter in
    /// the owner's own signature, in order.
    ///
    /// Only a function or initializer binds its parameters in exactly one
    /// body. A subscript's index parameters are bound again by its setter and
    /// place accessors, and a rename through one body would miss the others,
    /// so those stay unrecorded (tools then refuse to rename them).
    fn signature_param_names(&self) -> Vec<Option<(SyntaxNode, SyntaxToken)>> {
        use kestrel_ast_builder::NodeKind;
        if !matches!(
            self.ctx.get::<NodeKind>(self.owner),
            Some(NodeKind::Function | NodeKind::Initializer)
        ) {
            return Vec::new();
        }
        let Some(list) = kestrel_ast_builder::syntax::cst_node(self.ctx, self.owner)
            .and_then(|decl| decl.children().find_map(ast::ParameterList::cast))
        else {
            return Vec::new();
        };
        list.parameters()
            .filter_map(|param| match param.pattern()?.pat()? {
                ast::Pat::BindingPattern(b) => {
                    Some(Some((b.syntax().clone(), b.identifier_token()?)))
                },
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast_builder::{Name, NodeKind, build_declarations};
    use kestrel_hecs::World;
    use kestrel_hir::body::*;

    /// Build `source` into a fresh world; return it with the root and the
    /// first function named `name`.
    fn setup(source: &str, name: &str) -> (World, Entity, Entity) {
        let mut world = World::new();
        world.begin_revision();
        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name(Name::ROOT.into()));
        let file = world.spawn();
        let tokens: Vec<_> = kestrel_lexer::lex(source, file.index())
            .filter_map(|r| r.ok())
            .collect();
        let result = kestrel_parser::parse_source_file_from_source(
            source,
            tokens.iter().map(|t| (t.value.clone(), t.span.clone())),
        );
        build_declarations(&mut world, file, &result.tree(), root, None);
        let func = world
            .iter_component::<Name>()
            .find(|(e, n)| n.0 == name && world.get::<NodeKind>(*e) == Some(&NodeKind::Function))
            .map(|(e, _)| e)
            .expect("function");
        (world, root, func)
    }

    fn lower(source: &str, name: &str) -> Arc<HirBody> {
        let (world, root, func) = setup(source, name);
        let ctx = world.query_context();
        ctx.query(LowerBody { entity: func, root })
            .expect("should produce HirBody")
    }

    #[test]
    fn lower_empty_body() {
        let hir = lower("func f() {}", "f");
        assert!(hir.statements.is_empty());
        assert!(hir.tail_expr.is_none());
        assert!(hir.params.is_empty());
    }

    #[test]
    fn lower_literal_tail_expr() {
        let hir = lower("func f() { 42 }", "f");
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
        let hir = lower("func f() { let x = 10; }", "f");
        assert_eq!(hir.statements.len(), 1);
        assert_eq!(hir.locals.len(), 1); // one local: x
        assert_eq!(hir.locals[hir.locals.iter().next().unwrap().0].name, "x");
    }

    #[test]
    fn lower_function_params() {
        let hir = lower("func add(a: Int, b: Int) {}", "add");
        assert_eq!(hir.params.len(), 2);
        assert_eq!(hir.locals[hir.params[0]].name, "a");
        assert_eq!(hir.locals[hir.params[1]].name, "b");
    }

    #[test]
    fn lower_method_with_self() {
        let hir = lower("struct S { func method() {} }", "method");
        // self + no explicit params = 1 param (self)
        assert_eq!(hir.params.len(), 1);
        assert_eq!(hir.locals[hir.params[0]].name, "self");
    }

    #[test]
    fn lower_if_expression() {
        let hir = lower("func f() { if true { 1 } else { 2 } }", "f");
        let tail = &hir.exprs[hir.tail_expr.unwrap()];
        assert!(matches!(tail, HirExpr::If { .. }));
    }

    #[test]
    fn lower_assignment() {
        let hir = lower("func f() { var x = 1; x = 2; }", "f");
        assert_eq!(hir.statements.len(), 2);
        // The second statement should be an Assign to a local
        match &hir.stmts[hir.statements[1]] {
            HirStmt::Expr { expr, .. } => {
                let HirExpr::Assign { target, .. } = &hir.exprs[*expr] else {
                    panic!("expected Assign");
                };
                assert!(matches!(hir.exprs[*target], HirExpr::Local(..)));
            },
            _ => panic!("expected Expr stmt"),
        }
    }

    #[test]
    fn lower_no_body_returns_none() {
        let (world, root, func) = setup("protocol P { func noBody() }", "noBody");
        let ctx = world.query_context();
        assert!(ctx.query(LowerBody { entity: func, root }).is_none());
    }

    /// A statement-like expression ending a block without `;` is the block's
    /// value, not a statement.
    #[test]
    fn trailing_if_is_the_block_value() {
        let hir = lower(
            "func f() -> Int { let a = 1; if true { a } else { 2 } }",
            "f",
        );
        assert_eq!(hir.statements.len(), 1);
        assert!(matches!(
            hir.exprs[hir.tail_expr.unwrap()],
            HirExpr::If { .. }
        ));
    }

    /// In a closure, a statement-like expression stands without a
    /// `Statement` wrapper; one that is not last is a statement, kept in
    /// source order with the statements around it.
    #[test]
    fn closure_items_keep_source_order() {
        let hir = lower(
            "func g(c: () -> Int) -> Int { c() }
             func f() -> Int { g({ while false {}; let x = 1; x }) }",
            "f",
        );
        let closure = hir
            .exprs
            .iter()
            .find_map(|(_, e)| match e {
                HirExpr::Closure { body, .. } => Some(body.clone()),
                _ => None,
            })
            .expect("closure");
        assert_eq!(closure.stmts.len(), 2);
        let first = &hir.stmts[closure.stmts[0]];
        let HirStmt::Expr { expr, .. } = first else {
            panic!("the loop comes first: {first:?}");
        };
        assert!(matches!(hir.exprs[*expr], HirExpr::Loop { .. }));
        assert!(matches!(hir.stmts[closure.stmts[1]], HirStmt::Let { .. }));
        assert!(matches!(
            hir.exprs[closure.tail_expr.unwrap()],
            HirExpr::Local(..)
        ));
    }

    /// `it` is a parameter only when a header-less closure refers to the
    /// *name* `it`; a member spelled `it` is not a reference.
    #[test]
    fn implicit_it_is_a_name_reference() {
        let param_names = |src: &'static str| {
            let hir = lower(src, "f");
            hir.exprs
                .iter()
                .filter_map(|(_, e)| match e {
                    HirExpr::Closure { params, .. } => Some(
                        params
                            .iter()
                            .map(|p| hir.locals[p.local].name.clone())
                            .collect::<Vec<_>>(),
                    ),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            param_names("func f() { let c = { it }; }"),
            vec![vec!["it"]]
        );
        assert_eq!(
            param_names("func f(p: Int) { let c = { p.it }; }"),
            vec![Vec::<String>::new()]
        );
        // An explicit header is transparent: the inner `it` is the outer
        // closure's.
        assert_eq!(
            param_names("func f() { let c = { { (y) in y + it } }; }"),
            vec![vec!["y".to_string()], vec!["it".to_string()]]
        );
    }

    /// A field initializer is a one-expression body.
    #[test]
    fn field_initializer_lowers_as_a_value() {
        let (world, root, _) = setup(
            "struct S { var x: Int = 42 }
func f() {}",
            "f",
        );
        let field = world
            .iter_component::<Name>()
            .find(|(_, n)| n.0 == "x")
            .map(|(e, _)| e)
            .unwrap();
        let ctx = world.query_context();
        let hir = ctx
            .query(LowerBody {
                entity: field,
                root,
            })
            .unwrap();
        assert!(hir.statements.is_empty());
        assert!(matches!(
            hir.exprs[hir.tail_expr.unwrap()],
            HirExpr::Literal {
                value: HirLiteral::Integer(42),
                ..
            }
        ));
    }

    /// Every body of a real stdlib file lowers.
    #[test]
    fn ordering_ks_bodies_lower() {
        let (world, root, _) = setup(
            include_str!("../../../lang/std/core/ordering.ks"),
            "reverse",
        );
        let ctx = world.query_context();
        let bodies: Vec<_> = world.iter_component::<Valued>().map(|(e, _)| e).collect();
        assert!(
            bodies.len() >= 6,
            "ordering.ks has 6 methods, got {}",
            bodies.len()
        );
        for body in bodies {
            assert!(ctx.query(LowerBody { entity: body, root }).is_some());
        }
    }
}
