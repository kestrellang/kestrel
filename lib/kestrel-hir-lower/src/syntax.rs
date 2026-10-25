//! Reading a body's CST: the shapes body lowering consumes.
//!
//! Lowering walks the typed views (`kestrel_syntax_tree::ast`) directly — there
//! is no intermediate body AST. This module holds the few decisions about
//! *syntax* that are more than one accessor deep, so the lowering code reads
//! as "what does this construct mean" rather than "where is its child":
//!
//! - which items of a block are statements and which expression is its value
//!   ([`block_syntax`], [`closure_body_syntax`]);
//! - a path's segments versus member accesses on a computed base
//!   ([`PathSyntax`]);
//! - `if`/`while`/`guard` condition lists ([`Cond`]);
//! - operator tokens ([`binary_op`], [`unary_op`], …);
//! - the implicit `it` of a header-less closure ([`implicit_it_reference`]).
//!
//! Missing syntax is never papered over with an empty name: an absent child is
//! an [`ExprSrc::Error`] / [`PatSrc::Error`] carrying the span to report, and
//! an absent member name is `None` (lowered to `HirName::Missing`).

use kestrel_ast::{AstType, BinaryOp, CompoundAssignOp, PostfixOp, UnaryOp};
use kestrel_ast_builder::ast_type::ast_type_from_cst;
use kestrel_span::Span;
use kestrel_syntax_tree::ast::{self, AstNode};
use kestrel_syntax_tree::utils::get_node_span;
use kestrel_syntax_tree::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

// ===== Node classification =====

/// An expression position: the `Expression` wrapper or a bare `Expr*` node.
pub(crate) fn is_expr_like(kind: SyntaxKind) -> bool {
    kind == SyntaxKind::Expression || ast::Expr::can_cast(kind)
}

/// A pattern position: the `Pattern` wrapper, a `Pat` node, or recovery's
/// `ErrorPattern`.
pub(crate) fn is_pat_like(kind: SyntaxKind) -> bool {
    matches!(kind, SyntaxKind::Pattern | SyntaxKind::ErrorPattern) || ast::Pat::can_cast(kind)
}

/// The expression children of `node`, in order.
pub(crate) fn expr_children(node: &SyntaxNode) -> impl Iterator<Item = SyntaxNode> {
    node.children().filter(|c| is_expr_like(c.kind()))
}

pub(crate) fn first_expr(node: &SyntaxNode) -> Option<SyntaxNode> {
    expr_children(node).next()
}

pub(crate) fn first_pat(node: &SyntaxNode) -> Option<SyntaxNode> {
    node.children().find(|c| is_pat_like(c.kind()))
}

/// Strip `Expression` wrappers. An empty wrapper is returned as is (it lowers
/// to an error spanning the wrapper).
pub(crate) fn unwrap_expr(node: &SyntaxNode) -> SyntaxNode {
    let mut current = node.clone();
    while current.kind() == SyntaxKind::Expression {
        match current.children().next() {
            Some(inner) => current = inner,
            None => break,
        }
    }
    current
}

/// Strip `Pattern` wrappers.
pub(crate) fn unwrap_pat(node: &SyntaxNode) -> SyntaxNode {
    let mut current = node.clone();
    while current.kind() == SyntaxKind::Pattern {
        match current.children().next() {
            Some(inner) => current = inner,
            None => break,
        }
    }
    current
}

/// The direct child token of `kind`.
pub(crate) fn token(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
    node.children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|t| t.kind() == kind)
}

pub(crate) fn has_token(node: &SyntaxNode, kind: SyntaxKind) -> bool {
    token(node, kind).is_some()
}

/// The first direct non-trivia, non-error token.
pub(crate) fn first_token(node: &SyntaxNode) -> Option<SyntaxToken> {
    node.children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|t| !t.kind().is_trivia() && t.kind() != SyntaxKind::Error)
}

/// Direct children without trivia (and without recovery's `Error` tokens).
fn significant_elements(node: &SyntaxNode) -> Vec<SyntaxElement> {
    node.children_with_tokens()
        .filter(|e| {
            e.as_token()
                .is_none_or(|t| !t.kind().is_trivia() && t.kind() != SyntaxKind::Error)
        })
        .collect()
}

pub(crate) fn token_span(token: &SyntaxToken, file_id: usize) -> Span {
    Span::new(file_id, token.text_range().into())
}

// ===== Sources: a node to lower, or the span of a missing one =====

/// An expression to lower: its node, or — when the syntax is absent — the
/// span the resulting error expression should carry.
#[derive(Clone)]
pub(crate) enum ExprSrc {
    Node(SyntaxNode),
    Error(Span),
}

impl ExprSrc {
    /// `node`, or an error at `span` when it is absent.
    pub(crate) fn or_error(node: Option<SyntaxNode>, span: &Span) -> Self {
        match node {
            Some(n) => ExprSrc::Node(n),
            None => ExprSrc::Error(span.clone()),
        }
    }

    pub(crate) fn node(&self) -> Option<&SyntaxNode> {
        match self {
            ExprSrc::Node(n) => Some(n),
            ExprSrc::Error(_) => None,
        }
    }

    /// The expression node itself, wrappers stripped.
    pub(crate) fn inner(&self) -> Option<SyntaxNode> {
        self.node().map(unwrap_expr)
    }
}

/// A pattern to lower.
#[derive(Clone)]
pub(crate) enum PatSrc {
    Node(SyntaxNode),
    Error(Span),
    /// `.Case(x)` with a bare identifier argument: a binding of the
    /// `EnumPatternArg`'s identifier, spanning the whole argument.
    ArgBinding(SyntaxNode),
}

impl PatSrc {
    pub(crate) fn or_error(node: Option<SyntaxNode>, span: &Span) -> Self {
        match node {
            Some(n) => PatSrc::Node(n),
            None => PatSrc::Error(span.clone()),
        }
    }

    /// The pattern this stands for, with `Pattern` wrappers and grouping
    /// parentheses (`(p)` — a one-element tuple pattern without a rest) removed.
    pub(crate) fn resolve(self, file_id: usize) -> PatSrc {
        let PatSrc::Node(node) = self else {
            return self;
        };
        let node = unwrap_pat(&node);
        if node.kind() == SyntaxKind::TuplePattern {
            let elements = tuple_pattern_elements(&node, file_id);
            if elements.len() == 1 && !elements[0].is_rest {
                return elements.into_iter().next().unwrap().pat.resolve(file_id);
            }
        }
        PatSrc::Node(node)
    }

    /// The kind of the resolved pattern node (`None` for an error or a
    /// synthesized binding).
    pub(crate) fn kind(&self) -> Option<SyntaxKind> {
        match self {
            PatSrc::Node(n) => Some(n.kind()),
            _ => None,
        }
    }
}

// ===== Blocks =====

/// A block's statements and its value expression.
pub(crate) struct BlockSyntax {
    pub stmts: Vec<StmtSyntax>,
    pub tail: Option<ExprSrc>,
}

impl BlockSyntax {
    pub(crate) fn empty() -> Self {
        BlockSyntax {
            stmts: Vec::new(),
            tail: None,
        }
    }

    /// The block of `node` (a `CodeBlock`), or an empty one.
    pub(crate) fn of(node: Option<SyntaxNode>, file_id: usize) -> Self {
        node.map_or_else(Self::empty, |n| block_syntax(&n, file_id))
    }
}

/// One statement of a block.
pub(crate) enum StmtSyntax {
    /// A `StatementKind` node (`VariableDeclaration`, `ExpressionStatement`,
    /// `GuardStatement`, `DeinitStatement`).
    Node(SyntaxNode),
    /// An expression standing as a statement without a `Statement` wrapper:
    /// a statement-like expression in a closure body that is not its last
    /// item. `span` is the statement's span.
    Expr { expr: ExprSrc, span: Span },
}

/// A `CodeBlock`: its statements, and the value — a trailing bare
/// `Expression`, or a final statement-like expression written without `;`.
pub(crate) fn block_syntax(block: &SyntaxNode, file_id: usize) -> BlockSyntax {
    let mut out = BlockSyntax::empty();
    let children: Vec<SyntaxNode> = block.children().collect();
    for (i, child) in children.iter().enumerate() {
        match child.kind() {
            SyntaxKind::Statement => {
                let Some(inner) = child.children().next() else {
                    continue;
                };
                let is_last = i + 1 == children.len();
                if is_last
                    && out.tail.is_none()
                    && inner.kind() == SyntaxKind::ExpressionStatement
                    && !has_token(&inner, SyntaxKind::Semicolon)
                {
                    let span = get_node_span(&inner, file_id);
                    out.tail = Some(ExprSrc::or_error(first_expr(&inner), &span));
                    continue;
                }
                out.stmts.push(StmtSyntax::Node(inner));
            },
            kind if is_expr_like(kind) => out.tail = Some(ExprSrc::Node(child.clone())),
            _ => {},
        }
    }
    out
}

/// A closure's items (the closure has no `CodeBlock`; its items sit directly
/// in `ExprClosure`). A statement-like expression in a closure stands without
/// a `Statement` wrapper, so an `Expression` is the value only when nothing
/// follows it; otherwise it is demoted to a statement.
pub(crate) fn closure_body_syntax(closure: &SyntaxNode, file_id: usize) -> BlockSyntax {
    let mut out = BlockSyntax::empty();
    let demote = |out: &mut BlockSyntax| {
        if let Some(prev) = out.tail.take() {
            let span = demoted_stmt_span(&prev, file_id);
            out.stmts.push(StmtSyntax::Expr { expr: prev, span });
        }
    };
    for child in closure.children() {
        match child.kind() {
            SyntaxKind::Statement => {
                demote(&mut out);
                if let Some(inner) = child.children().next() {
                    out.stmts.push(StmtSyntax::Node(inner));
                }
            },
            SyntaxKind::Expression => {
                demote(&mut out);
                out.tail = Some(ExprSrc::Node(child));
            },
            _ => {},
        }
    }
    out
}

/// The span of a demoted closure-body expression statement: the expression's
/// own span when it is malformed (so a diagnostic there has somewhere to
/// point), otherwise synthetic — the statement has no syntax of its own.
fn demoted_stmt_span(expr: &ExprSrc, file_id: usize) -> Span {
    match expr {
        ExprSrc::Error(span) => span.clone(),
        ExprSrc::Node(node) => {
            malformed_expr_span(node, file_id).unwrap_or_else(|| Span::synthetic(file_id))
        },
    }
}

/// `Some(span)` when `node` is syntactically unusable as an expression — an
/// empty wrapper, a non-expression node, an operator expression without a
/// recognised operator, or `()` grouping nothing — i.e. exactly the cases
/// that lower to an error expression before any name is looked up.
pub(crate) fn malformed_expr_span(node: &SyntaxNode, file_id: usize) -> Option<Span> {
    let inner = unwrap_expr(node);
    let span = get_node_span(&inner, file_id);
    let malformed = match inner.kind() {
        SyntaxKind::ExprGrouping => first_expr(&inner).is_none(),
        SyntaxKind::ExprUnary => unary_op(&inner).is_none(),
        SyntaxKind::ExprBinary => binary_op(&inner).is_none(),
        SyntaxKind::ExprCompoundAssignment => compound_assign_op(&inner).is_none(),
        SyntaxKind::ExprPostfix => postfix_op(&inner).is_none(),
        kind => !ast::Expr::can_cast(kind),
    };
    malformed.then_some(span)
}

// ===== Statements =====

/// A `let`/`var` declaration.
pub(crate) struct LetSyntax {
    pub is_mut: bool,
    pub pat: PatSrc,
    pub ty: Option<AstType>,
    pub value: Option<SyntaxNode>,
}

pub(crate) fn let_syntax(node: &SyntaxNode, span: &Span, file_id: usize) -> LetSyntax {
    // The initializer is the expression after `=` (a type annotation, if
    // any, sits before it).
    let mut seen_equals = false;
    let mut value = None;
    for element in node.children_with_tokens() {
        match element {
            SyntaxElement::Token(t) if t.kind() == SyntaxKind::Equals => seen_equals = true,
            SyntaxElement::Node(n) if seen_equals && is_expr_like(n.kind()) => {
                value = Some(n);
                break;
            },
            _ => {},
        }
    }
    LetSyntax {
        is_mut: has_token(node, SyntaxKind::Var),
        pat: PatSrc::or_error(first_pat(node), span),
        ty: node
            .children()
            .find(|c| c.kind().is_type())
            .and_then(|c| ast_type_from_cst(&c, file_id)),
        value,
    }
}

// ===== Conditions =====

/// One condition of an `if` / `while` / `guard`.
pub(crate) enum Cond {
    Expr(ExprSrc),
    /// `let pat = value`; `node` is the condition node.
    Let {
        pat: PatSrc,
        value: ExprSrc,
    },
}

impl Cond {
    pub(crate) fn is_let(&self) -> bool {
        matches!(self, Cond::Let { .. })
    }
}

fn let_condition(node: &SyntaxNode, file_id: usize) -> Cond {
    let span = get_node_span(node, file_id);
    Cond::Let {
        pat: PatSrc::or_error(first_pat(node), &span),
        value: ExprSrc::or_error(first_expr(node), &span),
    }
}

/// The conditions of an `ExprIf`: the `let` conditions and expressions
/// before its block.
pub(crate) fn if_conditions(node: &SyntaxNode, file_id: usize) -> Vec<Cond> {
    let mut conditions = Vec::new();
    for child in node.children() {
        match child.kind() {
            SyntaxKind::IfLetCondition => conditions.push(let_condition(&child, file_id)),
            SyntaxKind::Expression => conditions.push(Cond::Expr(ExprSrc::Node(child))),
            SyntaxKind::CodeBlock => break,
            _ => {},
        }
    }
    if conditions.is_empty()
        && let Some(bare) = node
            .children()
            .take_while(|c| c.kind() != SyntaxKind::CodeBlock)
            .find(|c| is_expr_like(c.kind()))
    {
        conditions.push(Cond::Expr(ExprSrc::Node(bare)));
    }
    conditions
}

/// The conditions of a `while let` or `guard`: `let` conditions of
/// `let_kind` and expressions, up to the block.
pub(crate) fn let_conditions(node: &SyntaxNode, let_kind: SyntaxKind, file_id: usize) -> Vec<Cond> {
    let mut conditions = Vec::new();
    for child in node.children() {
        if child.kind() == let_kind {
            conditions.push(let_condition(&child, file_id));
        } else if is_expr_like(child.kind()) {
            conditions.push(Cond::Expr(ExprSrc::Node(child)));
        } else if child.kind() == SyntaxKind::CodeBlock {
            break;
        }
    }
    conditions
}

/// The first `CodeBlock` child.
pub(crate) fn code_block(node: &SyntaxNode) -> Option<SyntaxNode> {
    node.children().find(|c| c.kind() == SyntaxKind::CodeBlock)
}

/// `label:` of a loop.
pub(crate) fn loop_label(node: &SyntaxNode) -> Option<String> {
    let label = node.children().find_map(ast::LoopLabel::cast)?;
    Some(label.identifier_token()?.text().to_string())
}

/// The label of `break label` / `continue label`.
pub(crate) fn jump_label(node: &SyntaxNode) -> Option<String> {
    token(node, SyntaxKind::Identifier).map(|t| t.text().to_string())
}

// ===== Paths and member access =====

/// One segment of a path: `name` or `name[T, U]`.
#[derive(Clone)]
pub(crate) struct PathSeg {
    pub name: String,
    pub type_args: Option<Vec<AstType>>,
    pub span: Span,
}

/// `.name[T]` applied to a computed base; `name` is `None` when the parser
/// recovered from `base.` with nothing after the dot.
#[derive(Clone)]
pub(crate) struct MemberSeg {
    pub name: Option<String>,
    /// The name's identifier token range.
    pub name_range: Option<rowan::TextRange>,
    pub type_args: Option<Vec<AstType>>,
}

/// Where an `ExprPath` starts.
#[derive(Clone)]
pub(crate) enum PathBase {
    /// `a.b[T].c` — every segment an identifier; scope decides later which
    /// prefix is a value and which segments are members.
    Segments(Vec<PathSeg>),
    /// `f().c` — a computed base (an `Expression` node).
    Expr(SyntaxNode),
}

/// An `ExprPath`: a base, then member accesses on it. A path spelled only
/// with identifiers has no members, except a trailing `.` with no name after
/// it, which is a member access whose name is missing.
#[derive(Clone)]
pub(crate) struct PathSyntax {
    pub base: PathBase,
    pub members: Vec<MemberSeg>,
}

impl PathSyntax {
    pub(crate) fn of(node: &SyntaxNode, file_id: usize) -> Self {
        let elements = significant_elements(node);
        let base_expr = match elements.first() {
            Some(SyntaxElement::Node(n)) if is_expr_like(n.kind()) => Some(n.clone()),
            _ => None,
        };
        match base_expr {
            Some(base) => PathSyntax {
                base: PathBase::Expr(base),
                members: member_chain(&elements[1..], file_id),
            },
            None => pure_path(&elements, file_id),
        }
    }

    /// The segments of a pure path (`a.b.c`) with no member accesses.
    pub(crate) fn as_segments(&self) -> Option<&[PathSeg]> {
        match (&self.base, self.members.is_empty()) {
            (PathBase::Segments(segs), true) => Some(segs),
            _ => None,
        }
    }
}

/// The segments of a path written only with identifiers.
fn pure_path(elements: &[SyntaxElement], file_id: usize) -> PathSyntax {
    let mut segments = Vec::new();
    let mut trailing_dot = false;
    for (i, element) in elements.iter().enumerate() {
        let Some(token) = element.as_token() else {
            continue;
        };
        match token.kind() {
            SyntaxKind::Identifier => {
                trailing_dot = false;
                segments.push(PathSeg {
                    name: token.text().to_string(),
                    type_args: type_args_at(elements, i + 1, file_id),
                    span: token_span(token, file_id),
                });
            },
            SyntaxKind::Dot => trailing_dot = true,
            _ => {},
        }
    }
    let members = if trailing_dot && !segments.is_empty() {
        vec![MemberSeg {
            name: None,
            name_range: None,
            type_args: None,
        }]
    } else {
        Vec::new()
    };
    PathSyntax {
        base: PathBase::Segments(segments),
        members,
    }
}

/// `.name[T]` accesses after a computed base.
fn member_chain(elements: &[SyntaxElement], file_id: usize) -> Vec<MemberSeg> {
    let mut members = Vec::new();
    let mut i = 0;
    while i < elements.len() {
        let is_dot = elements[i]
            .as_token()
            .is_some_and(|t| t.kind() == SyntaxKind::Dot);
        i += 1;
        if !is_dot {
            continue;
        }
        let name_token = elements
            .get(i)
            .and_then(SyntaxElement::as_token)
            .filter(|t| t.kind() == SyntaxKind::Identifier);
        let name = name_token.map(|t| t.text().to_string());
        let name_range = name_token.map(|t| t.text_range());
        let type_args = if name.is_some() {
            i += 1;
            let args = type_args_at(elements, i, file_id);
            if args.is_some() {
                i += 1;
            }
            args
        } else {
            None
        };
        members.push(MemberSeg {
            name,
            name_range,
            type_args,
        });
    }
    members
}

/// The type arguments when `elements[at]` is a `TypeArgumentList`.
fn type_args_at(elements: &[SyntaxElement], at: usize, file_id: usize) -> Option<Vec<AstType>> {
    let list = elements.get(at)?.as_node()?;
    (list.kind() == SyntaxKind::TypeArgumentList).then(|| type_args(list, file_id))
}

/// The types of a `TypeArgumentList`.
pub(crate) fn type_args(list: &SyntaxNode, file_id: usize) -> Vec<AstType> {
    list.children()
        .filter(|c| c.kind().is_type())
        .filter_map(|c| ast_type_from_cst(&c, file_id))
        .collect()
}

// ===== Calls =====

/// One argument: `label: value` or `value`.
pub(crate) struct ArgSyntax {
    pub label: Option<String>,
    pub value: ExprSrc,
}

/// The arguments of an `ArgumentList` (parenthesised and trailing closures,
/// in source order).
pub(crate) fn arguments(list: &SyntaxNode, file_id: usize) -> Vec<ArgSyntax> {
    list.children()
        .filter_map(ast::Argument::cast)
        .map(|arg| {
            let label = arg
                .colon_token()
                .and(arg.label())
                .map(|t| t.text().to_string());
            let span = get_node_span(arg.syntax(), file_id);
            ArgSyntax {
                label,
                value: ExprSrc::or_error(first_expr(arg.syntax()), &span),
            }
        })
        .collect()
}

// ===== Closures =====

/// One closure parameter.
pub(crate) struct ClosureParamSyntax {
    pub pat: PatSrc,
    pub ty: Option<AstType>,
    /// Declared `mutating` (by reference).
    pub is_mut: bool,
}

/// The parameter header of a closure, or `None` when it has none.
pub(crate) fn closure_params(
    closure: &SyntaxNode,
    file_id: usize,
) -> Option<Vec<ClosureParamSyntax>> {
    let header = closure
        .children()
        .find(|c| c.kind() == SyntaxKind::ClosureParams)?;
    Some(
        header
            .children()
            .filter(|c| c.kind() == SyntaxKind::ClosureParam)
            .map(|param| {
                let span = get_node_span(&param, file_id);
                ClosureParamSyntax {
                    pat: PatSrc::or_error(first_pat(&param), &span),
                    ty: param
                        .children()
                        .find(|c| c.kind().is_type())
                        .and_then(|c| ast_type_from_cst(&c, file_id)),
                    is_mut: has_token(&param, SyntaxKind::Mutating),
                }
            })
            .collect(),
    )
}

/// The first reference to the NAME `it` that a header-less closure owns, in
/// source order: a value path whose first segment is `it` (`it`, `it.count`,
/// `it.0`…). A member, argument label, or binding spelled `it` (`x.it`,
/// `f(it: 1)`, `let it = …`) is not a reference (audit H8: this used to be a
/// search for any `it` token).
///
/// `it` belongs to the innermost enclosing closure WITHOUT a parameter
/// header. So the walk stops at a nested header-less closure (`{ list.map {
/// it } }` — that `it` is the inner closure's) but looks through a nested
/// closure that declares its parameters (`{ ys.filter { (y) in y == it } }`
/// — that `it` is the outer closure's). Interpolation holes are ordinary
/// CST nodes, so `"\(it)"` counts.
pub(crate) fn implicit_it_reference(closure: &SyntaxNode) -> Option<SyntaxNode> {
    fn walk(node: &SyntaxNode) -> Option<SyntaxNode> {
        node.children().find_map(|child| match child.kind() {
            SyntaxKind::ClosureParams => None,
            SyntaxKind::ExprClosure
                if !child
                    .children()
                    .any(|c| c.kind() == SyntaxKind::ClosureParams) =>
            {
                None
            },
            _ => {
                let names_it = ast::ExprPath::cast(child.clone()).is_some_and(|path| {
                    path.expression().is_none()
                        && path.identifier_token().is_some_and(|t| t.text() == "it")
                });
                if names_it { Some(child) } else { walk(&child) }
            },
        })
    }
    walk(closure)
}

// ===== Patterns =====

/// One element of a tuple pattern.
pub(crate) struct TupleElem {
    pub pat: PatSrc,
    pub is_rest: bool,
}

/// The elements of a `TuplePattern`, with rest markers.
pub(crate) fn tuple_pattern_elements(node: &SyntaxNode, file_id: usize) -> Vec<TupleElem> {
    node.children()
        .filter(|c| c.kind() == SyntaxKind::TuplePatternElement || is_pat_like(c.kind()))
        .map(|c| {
            if c.kind() == SyntaxKind::TuplePatternElement {
                let span = get_node_span(&c, file_id);
                let is_rest = c.children().any(|p| p.kind() == SyntaxKind::RestPattern);
                TupleElem {
                    pat: first_pat(&c).map_or(PatSrc::Error(span), PatSrc::Node),
                    is_rest,
                }
            } else {
                TupleElem {
                    is_rest: c.kind() == SyntaxKind::RestPattern,
                    pat: PatSrc::Node(c),
                }
            }
        })
        .collect()
}

/// `label:` of an enum-pattern argument (an identifier followed by `:`).
pub(crate) fn enum_arg_label(arg: &SyntaxNode) -> Option<String> {
    let elements = significant_elements(arg);
    match (elements.first(), elements.get(1)) {
        (Some(SyntaxElement::Token(name)), Some(SyntaxElement::Token(colon)))
            if name.kind() == SyntaxKind::Identifier && colon.kind() == SyntaxKind::Colon =>
        {
            Some(name.text().to_string())
        },
        _ => None,
    }
}

// ===== Operators =====

pub(crate) fn unary_op(node: &SyntaxNode) -> Option<UnaryOp> {
    match unary_op_of(first_token(node)?.kind())? {
        // `&mutating expr`: the `mutating` keyword sits inside the node.
        UnaryOp::Borrow if has_token(node, SyntaxKind::Mutating) => Some(UnaryOp::BorrowMutating),
        op => Some(op),
    }
}

/// The unary operator a leading token spells (`&` is `Borrow`; the
/// `mutating` that makes it `BorrowMutating` is a second token).
pub(crate) fn unary_op_of(kind: SyntaxKind) -> Option<UnaryOp> {
    let op = match kind {
        SyntaxKind::Minus => UnaryOp::Neg,
        SyntaxKind::Not => UnaryOp::LogicalNot,
        SyntaxKind::Bang => UnaryOp::BitNot,
        SyntaxKind::Plus => UnaryOp::Pos,
        SyntaxKind::DotDotLess => UnaryOp::RangeUpTo,
        SyntaxKind::DotDotEquals => UnaryOp::RangeThrough,
        SyntaxKind::Ampersand => UnaryOp::Borrow,
        _ => return None,
    };
    Some(op)
}

pub(crate) fn postfix_op(node: &SyntaxNode) -> Option<PostfixOp> {
    node.children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find_map(|t| match t.kind() {
            SyntaxKind::Bang => Some(PostfixOp::Unwrap),
            SyntaxKind::DotDot => Some(PostfixOp::RangeFrom),
            _ => None,
        })
}

pub(crate) fn binary_op(node: &SyntaxNode) -> Option<BinaryOp> {
    node.children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find_map(|t| binary_op_of(t.kind()))
}

pub(crate) fn binary_op_of(kind: SyntaxKind) -> Option<BinaryOp> {
    let op = match kind {
        SyntaxKind::Plus => BinaryOp::Add,
        SyntaxKind::Minus => BinaryOp::Sub,
        SyntaxKind::Star => BinaryOp::Mul,
        SyntaxKind::Slash => BinaryOp::Div,
        SyntaxKind::Percent => BinaryOp::Rem,
        SyntaxKind::Ampersand => BinaryOp::BitAnd,
        SyntaxKind::Pipe => BinaryOp::BitOr,
        SyntaxKind::Caret => BinaryOp::BitXor,
        SyntaxKind::LessLess => BinaryOp::Shl,
        SyntaxKind::GreaterGreater => BinaryOp::Shr,
        SyntaxKind::EqualsEquals => BinaryOp::Eq,
        SyntaxKind::BangEquals => BinaryOp::Ne,
        SyntaxKind::Less => BinaryOp::Lt,
        SyntaxKind::Greater => BinaryOp::Gt,
        SyntaxKind::LessEquals => BinaryOp::Le,
        SyntaxKind::GreaterEquals => BinaryOp::Ge,
        SyntaxKind::And => BinaryOp::And,
        SyntaxKind::Or => BinaryOp::Or,
        SyntaxKind::QuestionQuestion => BinaryOp::Coalesce,
        SyntaxKind::DotDotEquals => BinaryOp::RangeInclusive,
        SyntaxKind::DotDotLess => BinaryOp::RangeExclusive,
        _ => return None,
    };
    Some(op)
}

pub(crate) fn compound_assign_op(node: &SyntaxNode) -> Option<CompoundAssignOp> {
    node.children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find_map(|t| compound_assign_op_of(t.kind()))
}

pub(crate) fn compound_assign_op_of(kind: SyntaxKind) -> Option<CompoundAssignOp> {
    let op = match kind {
        SyntaxKind::PlusEquals => CompoundAssignOp::AddAssign,
        SyntaxKind::MinusEquals => CompoundAssignOp::SubAssign,
        SyntaxKind::StarEquals => CompoundAssignOp::MulAssign,
        SyntaxKind::SlashEquals => CompoundAssignOp::DivAssign,
        SyntaxKind::PercentEquals => CompoundAssignOp::RemAssign,
        SyntaxKind::AmpersandEquals => CompoundAssignOp::BitAndAssign,
        SyntaxKind::PipeEquals => CompoundAssignOp::BitOrAssign,
        SyntaxKind::CaretEquals => CompoundAssignOp::BitXorAssign,
        SyntaxKind::LessLessEquals => CompoundAssignOp::ShlAssign,
        SyntaxKind::GreaterGreaterEquals => CompoundAssignOp::ShrAssign,
        _ => return None,
    };
    Some(op)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lex `text` and return its non-trivia `SyntaxKind`s.
    fn kinds_of(text: &str) -> Vec<SyntaxKind> {
        kestrel_lexer::lex(text, 0)
            .filter_map(|t| t.ok())
            .map(|t| SyntaxKind::from(t.value))
            .filter(|k| !k.is_trivia())
            .collect()
    }

    /// Every operator spelling must lex back to the token lowering maps to
    /// that same operator.
    ///
    /// Both directions are derived — the ops come from walking the proven-
    /// complete `SyntaxKind::ALL` through lowering's own maps, so there is no
    /// hand-written list to fall out of date. This is the check that was
    /// missing while `kestrel-hir-lower` carried a second spelling table that
    /// wrote `&&`, `||` and `...` for operators Kestrel spells `and`, `or` and
    /// `..=`; none of those three lex to the token they claim.
    #[test]
    fn operator_spellings_round_trip_through_the_lexer() {
        for &kind in SyntaxKind::ALL {
            if let Some(op) = binary_op_of(kind) {
                assert_eq!(
                    kinds_of(op.symbol()),
                    vec![kind],
                    "BinaryOp::{op:?} is spelled {:?}, which does not lex to \
                     the single token {kind:?} lowering maps to it",
                    op.symbol()
                );
            }
            if let Some(op) = compound_assign_op_of(kind) {
                assert_eq!(
                    kinds_of(op.symbol()),
                    vec![kind],
                    "CompoundAssignOp::{op:?} is spelled {:?}, which does not \
                     lex to {kind:?}",
                    op.symbol()
                );
            }
            if let Some(op) = unary_op_of(kind) {
                assert_eq!(
                    kinds_of(op.symbol()),
                    vec![kind],
                    "UnaryOp::{op:?} is spelled {:?}, which does not lex to {kind:?}",
                    op.symbol()
                );
            }
        }

        // `&mutating` is the one spelling that is deliberately two tokens —
        // `unary_op` promotes `Borrow` to `BorrowMutating` when it sees the
        // keyword inside the unary node.
        assert_eq!(
            kinds_of(UnaryOp::BorrowMutating.symbol()),
            vec![SyntaxKind::Ampersand, SyntaxKind::Mutating]
        );
    }
}
