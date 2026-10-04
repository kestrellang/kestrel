//! `BodySourceMap`: the link between a lowered body's HIR ids and its syntax.
//!
//! HIR keeps its own `Span`s for diagnostics; the source map is for tools
//! that start from a *position* (the LSP) or from an id and want the syntax
//! back. It is produced by the same walk that lowers the body
//! ([`crate::LowerBodyWithSourceMap`]), so the two can never disagree.
//!
//! Every direction is explicit:
//!
//! | from | to | |
//! |------|----|---|
//! | `HirExprId` / `HirPatId` / `HirStmtId` | `SyntaxNodePtr` | the node it was lowered from |
//! | `SyntaxNodePtr` | id | the id the node lowered *to* (a desugaring's outermost node) |
//! | `LocalId` | [`LocalSource`] | the binding and its identifier token |
//! | identifier position | `LocalId` | a declaration site |
//! | identifier position | `HirExprId` | a use site (`HirExpr::Local`) |
//!
//! Synthesized ids (desugaring temporaries, `self`, an implicit `it`) have no
//! entry. Ranges are positions in the body's file.

use kestrel_hir::body::{HirExprId, HirPatId, HirStmtId};
use kestrel_hir::res::LocalId;
use kestrel_syntax_tree::{SyntaxNode, SyntaxNodePtr, SyntaxToken};
use rowan::{TextRange, TextSize};

/// Where a local is declared: the binding node and its name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LocalSource {
    /// The node that binds the name (`BindingPattern`, `AtPattern`,
    /// `Parameter`, `ArrayPatternRest`, `EnumPatternArg`, …).
    pub binding: SyntaxNodePtr,
    /// The identifier token's range — the bytes a rename rewrites.
    pub name: TextRange,
}

/// See the module docs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BodySourceMap {
    expr_syntax: Vec<Option<SyntaxNodePtr>>,
    pat_syntax: Vec<Option<SyntaxNodePtr>>,
    stmt_syntax: Vec<Option<SyntaxNodePtr>>,
    local_sources: Vec<Option<LocalSource>>,
    node_exprs: Vec<(SyntaxNodePtr, HirExprId)>,
    node_pats: Vec<(SyntaxNodePtr, HirPatId)>,
    node_stmts: Vec<(SyntaxNodePtr, HirStmtId)>,
    local_names: Vec<(TextRange, LocalId)>,
    local_refs: Vec<(TextRange, HirExprId)>,
}

/// `vec[idx] = value`, growing the vector as needed.
fn set_at<T: Clone>(vec: &mut Vec<Option<T>>, idx: usize, value: T) {
    if vec.len() <= idx {
        vec.resize(idx + 1, None);
    }
    vec[idx] = Some(value);
}

fn get_at<T>(vec: &[Option<T>], idx: usize) -> Option<&T> {
    vec.get(idx).and_then(Option::as_ref)
}

/// The entry for `ptr` in a node map.
fn find_node<T: Copy>(map: &[(SyntaxNodePtr, T)], ptr: &SyntaxNodePtr) -> Option<T> {
    map.iter().find(|(p, _)| p == ptr).map(|(_, id)| *id)
}

/// The entry whose range contains `offset` (ranges never overlap: each is
/// one identifier token).
fn find_range<T: Copy>(map: &[(TextRange, T)], offset: TextSize) -> Option<T> {
    map.iter()
        .find(|(range, _)| range.contains_inclusive(offset))
        .map(|(_, id)| *id)
}

impl BodySourceMap {
    // ===== Lookups =====

    /// The node `expr` was lowered from.
    pub fn expr_syntax(&self, expr: HirExprId) -> Option<SyntaxNodePtr> {
        get_at(&self.expr_syntax, expr.raw() as usize).copied()
    }

    /// The expression `node` lowered to.
    pub fn node_expr(&self, node: &SyntaxNode) -> Option<HirExprId> {
        find_node(&self.node_exprs, &SyntaxNodePtr::new(node))
    }

    pub fn pat_syntax(&self, pat: HirPatId) -> Option<SyntaxNodePtr> {
        get_at(&self.pat_syntax, pat.raw() as usize).copied()
    }

    pub fn node_pat(&self, node: &SyntaxNode) -> Option<HirPatId> {
        find_node(&self.node_pats, &SyntaxNodePtr::new(node))
    }

    pub fn stmt_syntax(&self, stmt: HirStmtId) -> Option<SyntaxNodePtr> {
        get_at(&self.stmt_syntax, stmt.raw() as usize).copied()
    }

    pub fn node_stmt(&self, node: &SyntaxNode) -> Option<HirStmtId> {
        find_node(&self.node_stmts, &SyntaxNodePtr::new(node))
    }

    /// Where `local` is declared; `None` for a local the source does not
    /// spell (`self`, an implicit `it`, desugaring temporaries, destructured
    /// parameters' synthetic names).
    pub fn local_source(&self, local: LocalId) -> Option<&LocalSource> {
        get_at(&self.local_sources, local.raw() as usize)
    }

    /// The local whose declaring identifier contains `offset`.
    pub fn local_declared_at(&self, offset: TextSize) -> Option<LocalId> {
        find_range(&self.local_names, offset)
    }

    /// The `HirExpr::Local` whose identifier contains `offset` (a use of a
    /// local, by name).
    pub fn local_ref_at(&self, offset: TextSize) -> Option<HirExprId> {
        find_range(&self.local_refs, offset)
    }

    /// Every use of a local, by name: `(identifier range, HirExpr::Local id)`.
    pub fn local_refs(&self) -> impl Iterator<Item = (TextRange, HirExprId)> + '_ {
        self.local_refs.iter().copied()
    }

    /// The smallest lowered expression node containing `offset`, walking out
    /// from the token there. `root` is the body's file tree.
    pub fn expr_at(&self, root: &SyntaxNode, offset: TextSize) -> Option<HirExprId> {
        let token = token_at(root, offset)?;
        token.parent_ancestors().find_map(|n| self.node_expr(&n))
    }

    // ===== Recording (lowering only) =====

    pub(crate) fn record_expr(&mut self, node: &SyntaxNode, expr: HirExprId) {
        let ptr = SyntaxNodePtr::new(node);
        // The innermost node wins for id → syntax: a grouping `(e)` lowers
        // to `e`'s id, which keeps pointing at `e`.
        if self.expr_syntax(expr).is_none() {
            set_at(&mut self.expr_syntax, expr.raw() as usize, ptr);
        }
        self.node_exprs.push((ptr, expr));
    }

    pub(crate) fn record_pat(&mut self, node: &SyntaxNode, pat: HirPatId) {
        let ptr = SyntaxNodePtr::new(node);
        if self.pat_syntax(pat).is_none() {
            set_at(&mut self.pat_syntax, pat.raw() as usize, ptr);
        }
        self.node_pats.push((ptr, pat));
    }

    pub(crate) fn record_stmt(&mut self, node: &SyntaxNode, stmt: HirStmtId) {
        let ptr = SyntaxNodePtr::new(node);
        set_at(&mut self.stmt_syntax, stmt.raw() as usize, ptr);
        self.node_stmts.push((ptr, stmt));
    }

    pub(crate) fn record_local(
        &mut self,
        local: LocalId,
        binding: &SyntaxNode,
        name: &SyntaxToken,
    ) {
        let source = LocalSource {
            binding: SyntaxNodePtr::new(binding),
            name: name.text_range(),
        };
        self.local_names.push((source.name, local));
        set_at(&mut self.local_sources, local.raw() as usize, source);
    }

    pub(crate) fn record_local_ref(&mut self, name: TextRange, expr: HirExprId) {
        self.local_refs.push((name, expr));
    }
}

/// The token at `offset`, preferring an identifier when `offset` sits
/// between two tokens (the cursor just after a name).
pub fn token_at(root: &SyntaxNode, offset: TextSize) -> Option<SyntaxToken> {
    if offset > root.text_range().end() {
        return None;
    }
    match root.token_at_offset(offset) {
        rowan::TokenAtOffset::None => None,
        rowan::TokenAtOffset::Single(t) => Some(t),
        rowan::TokenAtOffset::Between(left, right) => {
            if right.kind() == kestrel_syntax_tree::SyntaxKind::Identifier
                || left.kind().is_trivia()
            {
                Some(right)
            } else {
                Some(left)
            }
        },
    }
}
