//! Lowering context: arenas, local scope stack, and helper methods.
//!
//! `LowerCtx` holds the mutable state for a single HIR lowering pass.
//! It owns the HIR arenas and a stack of local variable scopes.

use std::collections::HashMap;

use kestrel_ast::arena::Arena;
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

    /// Statements that originated from guard-let desugaring.
    /// Populated during lowering, transferred to HirBody for analysis.
    pub guard_let_stmts: Vec<HirStmtId>,
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
            guard_let_stmts: Vec::new(),
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

    // ===== Capture detection =====

    /// Walk a closure body and collect locals defined before `entry_depth`.
    pub fn collect_captures(&self, block: &HirBlock, entry_depth: usize) -> Vec<LocalId> {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        let mut captures = Vec::new();
        self.walk_block_for_captures(block, entry_depth, &mut seen, &mut captures);
        captures
    }

    fn walk_block_for_captures(
        &self,
        block: &HirBlock,
        entry_depth: usize,
        seen: &mut std::collections::HashSet<LocalId>,
        captures: &mut Vec<LocalId>,
    ) {
        for &stmt_id in &block.stmts {
            self.walk_stmt_for_captures(stmt_id, entry_depth, seen, captures);
        }
        if let Some(tail) = block.tail_expr {
            self.walk_expr_for_captures(tail, entry_depth, seen, captures);
        }
    }

    fn walk_stmt_for_captures(
        &self,
        id: HirStmtId,
        entry_depth: usize,
        seen: &mut std::collections::HashSet<LocalId>,
        captures: &mut Vec<LocalId>,
    ) {
        match &self.stmts[id] {
            HirStmt::Expr { expr, .. } => {
                self.walk_expr_for_captures(*expr, entry_depth, seen, captures);
            },
            HirStmt::Let { value, .. } => {
                if let Some(v) = value {
                    self.walk_expr_for_captures(*v, entry_depth, seen, captures);
                }
            },
            HirStmt::Deinit { .. } => {},
        }
    }

    fn walk_expr_for_captures(
        &self,
        id: HirExprId,
        entry_depth: usize,
        seen: &mut std::collections::HashSet<LocalId>,
        captures: &mut Vec<LocalId>,
    ) {
        match &self.exprs[id] {
            HirExpr::Local(local_id, _) => {
                if seen.insert(*local_id) {
                    if let Some(depth) = self.local_scope_depth(*local_id) {
                        if depth <= entry_depth {
                            captures.push(*local_id);
                        }
                    }
                }
            },
            // Skip nested closures — they compute their own captures
            HirExpr::Closure { .. } => {},
            // Recurse into all other expression kinds
            _ => self.walk_expr_children(id, entry_depth, seen, captures),
        }
    }

    /// Walk child expressions of an expr for capture detection.
    fn walk_expr_children(
        &self,
        id: HirExprId,
        entry_depth: usize,
        seen: &mut std::collections::HashSet<LocalId>,
        captures: &mut Vec<LocalId>,
    ) {
        match &self.exprs[id] {
            HirExpr::Literal { .. }
            | HirExpr::Local(..)
            | HirExpr::Def(..)
            | HirExpr::OverloadSet { .. }
            | HirExpr::Error { .. }
            | HirExpr::Break { .. }
            | HirExpr::Continue { .. } => {},

            HirExpr::Call { callee, args, .. } => {
                self.walk_expr_for_captures(*callee, entry_depth, seen, captures);
                for arg in args {
                    self.walk_expr_for_captures(arg.value, entry_depth, seen, captures);
                }
            },
            HirExpr::MethodCall { receiver, args, .. } => {
                self.walk_expr_for_captures(*receiver, entry_depth, seen, captures);
                for arg in args {
                    self.walk_expr_for_captures(arg.value, entry_depth, seen, captures);
                }
            },
            HirExpr::ProtocolCall { receiver, args, .. } => {
                self.walk_expr_for_captures(*receiver, entry_depth, seen, captures);
                for arg in args {
                    self.walk_expr_for_captures(arg.value, entry_depth, seen, captures);
                }
            },
            HirExpr::Field { base, .. } | HirExpr::TupleIndex { base, .. } => {
                self.walk_expr_for_captures(*base, entry_depth, seen, captures);
            },
            HirExpr::ImplicitMember { args, .. } => {
                if let Some(args) = args {
                    for arg in args {
                        self.walk_expr_for_captures(arg.value, entry_depth, seen, captures);
                    }
                }
            },
            HirExpr::Assign { target, value, .. } => {
                self.walk_expr_for_captures(*target, entry_depth, seen, captures);
                self.walk_expr_for_captures(*value, entry_depth, seen, captures);
            },
            HirExpr::Tuple { elements, .. } => {
                for &e in elements {
                    self.walk_expr_for_captures(e, entry_depth, seen, captures);
                }
            },
            HirExpr::Array { elements, .. } => {
                for &e in elements {
                    self.walk_expr_for_captures(e, entry_depth, seen, captures);
                }
            },
            HirExpr::Dict { entries, .. } => {
                for entry in entries {
                    self.walk_expr_for_captures(entry.key, entry_depth, seen, captures);
                    self.walk_expr_for_captures(entry.value, entry_depth, seen, captures);
                }
            },
            HirExpr::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                self.walk_expr_for_captures(*condition, entry_depth, seen, captures);
                self.walk_block_for_captures(then_body, entry_depth, seen, captures);
                if let Some(else_b) = else_body {
                    self.walk_block_for_captures(else_b, entry_depth, seen, captures);
                }
            },
            HirExpr::Match {
                scrutinee, arms, ..
            } => {
                self.walk_expr_for_captures(*scrutinee, entry_depth, seen, captures);
                for arm in arms {
                    if let Some(g) = arm.guard {
                        self.walk_expr_for_captures(g, entry_depth, seen, captures);
                    }
                    self.walk_expr_for_captures(arm.body, entry_depth, seen, captures);
                }
            },
            HirExpr::Loop { body, .. } => {
                self.walk_block_for_captures(body, entry_depth, seen, captures);
            },
            HirExpr::Block { body, .. } => {
                self.walk_block_for_captures(body, entry_depth, seen, captures);
            },
            HirExpr::Return { value, .. } => {
                if let Some(v) = value {
                    self.walk_expr_for_captures(*v, entry_depth, seen, captures);
                }
            },
            HirExpr::Closure { .. } => {}, // already handled
        }
    }
}
