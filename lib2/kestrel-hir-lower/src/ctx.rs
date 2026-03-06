//! Lowering context: arenas, local scope stack, and helper methods.
//!
//! `LowerCtx` holds the mutable state for a single HIR lowering pass.
//! It owns the HIR arenas and a stack of local variable scopes.

use std::collections::HashMap;

use kestrel_ast::arena::Arena;
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::body::*;
use kestrel_hir::res::{Local, LocalId};
use kestrel_span2::Span;

/// Mutable context for lowering a single function/getter/setter body.
pub(crate) struct LowerCtx<'a> {
    pub ctx: &'a QueryContext<'a>,
    pub root: Entity,
    /// The function/init/getter entity whose body we're lowering
    pub owner: Entity,

    // HIR arenas being built
    pub exprs: Arena<HirExpr>,
    pub pats: Arena<HirPat>,
    pub stmts: Arena<HirStmt>,
    pub locals: Arena<Local>,

    /// Collected param local IDs (in declaration order)
    pub params: Vec<LocalId>,

    /// Local scope stack (innermost last)
    scopes: Vec<HashMap<String, LocalId>>,
}

impl<'a> LowerCtx<'a> {
    pub fn new(ctx: &'a QueryContext<'a>, root: Entity, owner: Entity) -> Self {
        Self {
            ctx,
            root,
            owner,
            exprs: Arena::new(),
            pats: Arena::new(),
            stmts: Arena::new(),
            locals: Arena::new(),
            params: Vec::new(),
            scopes: vec![HashMap::new()], // start with one scope for params
        }
    }

    // ===== Scope management =====

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Allocate a local variable slot and insert into current scope.
    pub fn define_local(&mut self, name: &str, is_mut: bool, span: Span) -> LocalId {
        let id = self.locals.alloc(Local {
            name: name.to_string(),
            is_mut,
            span,
        });
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), id);
        }
        id
    }

    /// Look up a local variable by name, walking scopes from innermost out.
    pub fn lookup_local(&self, name: &str) -> Option<LocalId> {
        for scope in self.scopes.iter().rev() {
            if let Some(&id) = scope.get(name) {
                return Some(id);
            }
        }
        None
    }

    // ===== Arena allocation =====

    pub fn alloc_expr(&mut self, expr: HirExpr) -> HirExprId {
        self.exprs.alloc(expr)
    }

    pub fn alloc_pat(&mut self, pat: HirPat) -> HirPatId {
        self.pats.alloc(pat)
    }

    pub fn alloc_stmt(&mut self, stmt: HirStmt) -> HirStmtId {
        self.stmts.alloc(stmt)
    }

}
