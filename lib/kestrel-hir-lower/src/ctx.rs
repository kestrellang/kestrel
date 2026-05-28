//! Lowering context: arenas, local scope stack, and helper methods.
//!
//! `LowerCtx` holds the mutable state for a single HIR lowering pass.
//! It owns the HIR arenas and a stack of local variable scopes.

use std::collections::HashMap;

use kestrel_ast::arena::Arena;
use kestrel_ast_builder::InitEffect;
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::body::*;
use kestrel_hir::res::{Local, LocalId};
use kestrel_span::Span;

/// Bridge from the AST's string-typed name field to `HirName`. The AST
/// builder stores `""` for member identifiers that the parser recovered
/// as missing (the `Missing[Identifier ""]` wrapper from the parser's
/// recovery primitive). Translating that to `HirName::Missing` here
/// gives inference a single, explicit signal to short-circuit instead
/// of cascading "name not found" diagnostics.
pub(crate) fn name_from_ast(name: String) -> HirName {
    if name.is_empty() {
        HirName::Missing
    } else {
        HirName::Name(name)
    }
}

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

    /// Statements that originated from guard desugaring.
    /// Populated during lowering, transferred to HirBody for analysis.
    pub guard_stmts: Vec<HirStmtId>,
    /// Original condition expressions from while-loop desugaring.
    /// Populated during lowering, used by condition type analyzer.
    pub while_conditions: Vec<HirExprId>,

    /// Stack of active loop labels (innermost last). `None` = unlabeled loop.
    /// Used to validate break/continue labels during lowering.
    pub loop_labels: Vec<Option<String>>,

    /// Maps LocalId → scope depth at creation (for closure capture detection).
    local_depths: HashMap<LocalId, usize>,
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
            guard_stmts: Vec::new(),
            while_conditions: Vec::new(),
            loop_labels: Vec::new(),
            local_depths: HashMap::new(),
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
        self.local_depths.insert(id, self.scopes.len());
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), id);
        }
        id
    }

    /// Current scope depth (for closure capture detection).
    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }

    /// Scope depth at which a local was created.
    pub fn local_scope_depth(&self, id: LocalId) -> Option<usize> {
        self.local_depths.get(&id).copied()
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

    // ===== Loop label tracking =====

    /// Push a loop label onto the stack when entering a loop.
    pub fn push_loop(&mut self, label: Option<&str>) {
        self.loop_labels.push(label.map(|l| l.to_string()));
    }

    /// Pop a loop label from the stack when exiting a loop.
    pub fn pop_loop(&mut self) {
        self.loop_labels.pop();
    }

    /// Check if we're inside any loop.
    pub fn in_loop(&self) -> bool {
        !self.loop_labels.is_empty()
    }

    /// Check if a label is in the active loop stack.
    pub fn has_loop_label(&self, label: &str) -> bool {
        self.loop_labels.iter().any(|l| l.as_deref() == Some(label))
    }

    // ===== Init effect helpers =====

    /// For failable/throwing inits, wrap a bare success return in `.Some(())` or `.Ok(())`.
    /// Returns `None` if the owner isn't an effectful init.
    pub fn wrap_init_success_value(&mut self, span: Span) -> Option<HirExprId> {
        let effect = self.ctx.get::<InitEffect>(self.owner)?;
        let unit_expr = self.alloc_expr(HirExpr::Tuple {
            elements: vec![],
            span: span.clone(),
        });
        let wrapper_name = match effect {
            InitEffect::Failable => "Some",
            InitEffect::Throwing => "Ok",
        };
        Some(self.alloc_expr(HirExpr::ImplicitMember {
            name: HirName::name(wrapper_name),
            args: Some(vec![HirCallArg {
                label: None,
                value: unit_expr,
            }]),
            span,
        }))
    }

}
