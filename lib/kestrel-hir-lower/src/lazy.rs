//! The synthesized accessors of a `lazy var x: T = init` (see
//! `kestrel_ast_builder::Lazy`). Both read and write the hidden `$x: T?`
//! cache; neither has a written body.
//!
//! ```text
//! modifying get {                  set {
//!     match self.$x {                  self.$x = .Some(newValue)
//!         .Some(v) => v,           }
//!         .None => {
//!             let v = init;
//!             self.$x = .Some(v);
//!             v
//!         }
//!     }
//! }
//! ```

use kestrel_ast_builder::{Lazy, Name};
use kestrel_hir::LocalId;
use kestrel_hir::body::{
    HirBlock, HirCallArg, HirExpr, HirExprId, HirMatchArm, HirName, HirPat, HirPatArg, HirStmt,
    MatchSource,
};
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxNode;

use crate::ctx::LowerCtx;
use crate::syntax::ExprSrc;

/// The binding the getter's match introduces. Not a spellable identifier, so
/// it can never shadow a name the initializer uses.
const CACHED: &str = "$cached";

impl LowerCtx<'_> {
    /// The cache field's name and the receiver, when the owner is a
    /// well-formed lazy field (`field`) with a `self`.
    fn lazy_cache(&self, field: kestrel_hecs::Entity) -> Option<(String, LocalId)> {
        let cache = self.ctx.get::<Lazy>(field)?.cache?;
        let name = self.ctx.get::<Name>(cache)?.0.clone();
        Some((name, *self.params.first()?))
    }

    /// `self.$x`
    fn cache_place(&mut self, self_local: LocalId, cache: &str, span: &Span) -> HirExprId {
        let base = self.alloc_expr(HirExpr::Local(self_local, span.clone()));
        self.alloc_expr(HirExpr::Field {
            base,
            name: HirName::name(cache),
            span: span.clone(),
        })
    }

    /// `self.$x = .Some(value)` as a statement.
    fn fill_cache(
        &mut self,
        self_local: LocalId,
        cache: &str,
        value: HirExprId,
        span: &Span,
    ) -> kestrel_hir::body::HirStmtId {
        let target = self.cache_place(self_local, cache, span);
        let some = self.alloc_expr(HirExpr::ImplicitMember {
            owner: None,
            name: HirName::name("Some"),
            args: Some(vec![HirCallArg { label: None, value }]),
            span: span.clone(),
        });
        let assign = self.alloc_expr(HirExpr::Assign {
            target,
            value: some,
            span: span.clone(),
        });
        self.alloc_stmt(HirStmt::Expr {
            expr: assign,
            span: span.clone(),
        })
    }

    /// The lazy field's `modifying get`: return the cached value, or
    /// evaluate `init` once and cache it.
    pub(crate) fn lower_lazy_getter(&mut self, init: &SyntaxNode) -> HirBlock {
        let empty = HirBlock {
            stmts: Vec::new(),
            tail_expr: None,
        };
        let Some((cache, self_local)) = self.lazy_cache(self.owner) else {
            return empty;
        };
        let span = self.span(init);
        let scrutinee = self.cache_place(self_local, &cache, &span);

        // `.Some(v) => v`
        self.push_scope();
        let hit = self.define_local(CACHED, false, span.clone());
        let hit_pat = self.alloc_pat(HirPat::Binding {
            local: hit,
            by_ref: None,
            span: span.clone(),
        });
        let some_pat = self.alloc_pat(HirPat::ImplicitVariant {
            name: HirName::name("Some"),
            args: vec![HirPatArg {
                label: None,
                pattern: hit_pat,
            }],
            span: span.clone(),
        });
        let hit_body = self.alloc_expr(HirExpr::Local(hit, span.clone()));
        self.pop_scope();

        // `.None => { let v = init; self.$x = .Some(v); v }`
        let none_pat = self.alloc_pat(HirPat::ImplicitVariant {
            name: HirName::name("None"),
            args: Vec::new(),
            span: span.clone(),
        });
        self.push_scope();
        let computed = self.lower_block_stmts(&[], Some(&ExprSrc::Node(init.clone())));
        let mut stmts = computed.stmts;
        let value = computed
            .tail_expr
            .unwrap_or_else(|| self.alloc_expr(HirExpr::Error { span: span.clone() }));
        let fresh = self.define_local(CACHED, false, span.clone());
        stmts.push(self.alloc_stmt(HirStmt::Let {
            local: fresh,
            ty: None,
            value: Some(value),
            span: span.clone(),
        }));
        let stored = self.alloc_expr(HirExpr::Local(fresh, span.clone()));
        stmts.push(self.fill_cache(self_local, &cache, stored, &span));
        let tail = self.alloc_expr(HirExpr::Local(fresh, span.clone()));
        self.pop_scope();
        let miss_body = self.alloc_expr(HirExpr::Block {
            body: HirBlock {
                stmts,
                tail_expr: Some(tail),
            },
            span: span.clone(),
        });

        let lookup = self.alloc_expr(HirExpr::Match {
            scrutinee,
            arms: vec![
                HirMatchArm {
                    pattern: some_pat,
                    guard: None,
                    body: hit_body,
                },
                HirMatchArm {
                    pattern: none_pat,
                    guard: None,
                    body: miss_body,
                },
            ],
            source: MatchSource::IfLet,
            span,
        });
        HirBlock {
            stmts: Vec::new(),
            tail_expr: Some(lookup),
        }
    }

    /// The lazy field's `set`: `self.$x = .Some(newValue)`.
    pub(crate) fn lower_lazy_setter(&mut self, decl: &SyntaxNode) -> HirBlock {
        let field = self.ctx.parent_of(self.owner);
        let new_value = self.params.get(1).copied();
        let (Some((cache, self_local)), Some(new_value)) =
            (field.and_then(|f| self.lazy_cache(f)), new_value)
        else {
            return HirBlock {
                stmts: Vec::new(),
                tail_expr: None,
            };
        };
        let span = self.span(decl);
        let value = self.alloc_expr(HirExpr::Local(new_value, span.clone()));
        let stmt = self.fill_cache(self_local, &cache, value, &span);
        HirBlock {
            stmts: vec![stmt],
            tail_expr: None,
        }
    }
}
