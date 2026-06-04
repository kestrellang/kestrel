//! Expression parser data types.
//!
//! Pure AST-lite structs and enums that describe the shape of a parsed
//! expression. No parsing or emission logic lives here — the parser in
//! `expr/mod.rs` produces these values and the emitter in `expr/emit.rs`
//! consumes them.

use kestrel_lexer::Token;
use kestrel_span::Span;

use crate::block::{BlockItem, CodeBlockData};
use crate::ty::TyVariant;

/// A path segment with optional type arguments
/// Syntax: Ident or Ident[T, U]
#[derive(Debug, Clone)]
pub struct PathSegmentData {
    /// The identifier name
    pub name: Span,
    /// Optional type arguments: [T, U]
    pub type_args: Option<TypeArgsData>,
}

/// Type arguments data for expressions
/// Syntax: [T, U, V] where each can be any type (path, tuple, function, array)
#[derive(Debug, Clone)]
pub struct TypeArgsData {
    /// Left bracket span
    pub lbracket: Span,
    /// The type arguments (full types, not just paths)
    pub args: Vec<TyVariant>,
    /// Right bracket span
    pub rbracket: Span,
}

/// A call argument with optional label
#[derive(Debug, Clone)]
pub struct CallArg {
    /// Optional label (identifier before colon)
    pub label: Option<Span>,
    /// The colon after the label (if labeled)
    pub colon: Option<Span>,
    /// The argument expression
    pub value: ExprVariant,
}

/// Internal enum to distinguish between expression variants during parsing
#[derive(Debug, Clone)]
pub enum ExprVariant {
    /// Unit expression: ()
    Unit(Span, Span), // (lparen, rparen)
    /// Integer literal: 42, 0xFF, 0b1010, 0o17
    Integer(Span),
    /// Float literal: 3.14, 1.0e10
    Float(Span),
    /// String literal: "hello" (no interpolation)
    String(Span),
    /// Interpolated string literal: "Hello \(name)!"
    /// Contains interpolation expressions. The parsing of parts is deferred to the semantic phase.
    InterpolatedString(Span),
    /// Raw string literal: """hello"""
    RawString(Span),
    /// Character literal: 'a', '\n', '\u{1F600}'
    Char(Span),
    /// Boolean literal: true, false
    Bool(Span),
    /// Null literal: null
    Null(Span),
    /// Array literal: [1, 2, 3]
    Array(Span, Vec<ExprVariant>, Vec<Span>, Span), // (lbracket, elements, commas, rbracket)
    /// Dictionary literal: ["key": value, ...]
    Dictionary {
        lbracket: Span,
        entries: Vec<(ExprVariant, Span, ExprVariant)>, // (key, colon, value)
        commas: Vec<Span>,
        rbracket: Span,
    },
    /// Tuple literal: (1, 2, 3)
    Tuple(Span, Vec<ExprVariant>, Vec<Span>, Span), // (lparen, elements, commas, rparen)
    /// Grouping expression: (expr)
    Grouping(Span, Box<ExprVariant>, Span), // (lparen, inner, rparen)
    /// Path expression: a.b.c or a[T].b[U].c (used for initial path parsing)
    Path {
        segments: Vec<PathSegmentData>,
        dots: Vec<Span>,
    },
    /// Member access expression: base.member or base.member[T].
    ///
    /// `member` is `None` when the parser recovered from a missing identifier
    /// after the dot (e.g. `foo.` at EOF or `foo.;`). The CST emitter wraps
    /// the synthesized identifier in a `SyntaxKind::Missing` node so downstream
    /// passes can detect the gap.
    MemberAccess {
        base: Box<ExprVariant>,
        dot: Span,
        member: Option<Span>,
        type_args: Option<TypeArgsData>,
    },
    /// Tuple index expression: tuple.0, tuple.1
    TupleIndex {
        base: Box<ExprVariant>,
        dot: Span,
        index: Span,
    },
    /// Unary prefix expression: -expr, !expr, not expr, +expr
    Unary(Token, Span, Box<ExprVariant>), // (operator_token, operator_span, operand)
    /// Postfix expression: expr!
    Postfix {
        operand: Box<ExprVariant>,
        operator: Token,
        operator_span: Span,
    },
    /// Binary expression: a + b (flat, no precedence applied yet)
    Binary {
        lhs: Box<ExprVariant>,
        operator: Token,
        operator_span: Span,
        rhs: Box<ExprVariant>,
    },
    /// Call expression: callee(args) or callee { closure }
    Call {
        callee: Box<ExprVariant>,
        /// Left paren (None for trailing-closure-only calls)
        lparen: Option<Span>,
        arguments: Vec<CallArg>,
        commas: Vec<Span>,
        /// Right paren (None for trailing-closure-only calls)
        rparen: Option<Span>,
    },
    /// Assignment expression: lhs = rhs
    Assignment {
        lhs: Box<ExprVariant>,
        equals: Span,
        rhs: Box<ExprVariant>,
    },
    /// Compound assignment expression: lhs += rhs, lhs -= rhs, etc.
    CompoundAssignment {
        lhs: Box<ExprVariant>,
        operator: Token,
        operator_span: Span,
        rhs: Box<ExprVariant>,
    },
    /// If expression: if condition { then } else { else }
    /// Also supports if-let: if let pattern = expr { then } else { else }
    /// And if-let chains: if let .Some(x) = a, let .Some(y) = b { ... }
    If {
        if_span: Span,
        /// List of conditions (at least one). Each is either:
        /// - A boolean expression
        /// - A let-binding (pattern + expression)
        conditions: Vec<IfCondition>,
        then_block: CodeBlockData,
        else_clause: Option<ElseClause>,
    },
    /// While expression: label: while condition { body }
    While {
        label: Option<LabelData>,
        while_span: Span,
        condition: Box<ExprVariant>,
        body: CodeBlockData,
    },
    /// While-let expression: label: while let pattern = expr { body }
    /// Supports chains: while let .Some(x) = a, let .Some(y) = b, x > 0 { }
    WhileLet {
        label: Option<LabelData>,
        while_span: Span,
        /// List of conditions (at least one let-binding, possibly followed by more let-bindings or bool conditions)
        conditions: Vec<IfCondition>,
        body: CodeBlockData,
    },
    /// Loop expression: label: loop { body }
    Loop {
        label: Option<LabelData>,
        loop_span: Span,
        body: CodeBlockData,
    },
    /// For expression: label: for pattern in iterable { body }
    For {
        label: Option<LabelData>,
        for_span: Span,
        pattern: crate::pattern::PatternVariant,
        in_span: Span,
        iterable: Box<ExprVariant>,
        body: CodeBlockData,
    },
    /// Break expression: break or break label
    Break {
        break_span: Span,
        label: Option<Span>,
    },
    /// Continue expression: continue or continue label
    Continue {
        continue_span: Span,
        label: Option<Span>,
    },
    /// Return expression: return or return expr
    Return {
        return_span: Span,
        value: Option<Box<ExprVariant>>,
    },
    /// Throw expression: throw expr
    ///
    /// `value` is `None` when the parser recovered from a missing expression
    /// after `throw` (e.g. `throw\n}` mid-edit). The CST emits an `ExprThrow`
    /// node with no expression child; the ast-builder turns the absent child
    /// into `AstExpr::Error`, and a parse-level diagnostic ("expected
    /// expression after `throw`") is emitted from `throw_parser`'s
    /// `.validate`.
    Throw {
        throw_span: Span,
        value: Option<Box<ExprVariant>>,
    },
    /// Try expression: try expr
    Try {
        try_span: Span,
        operand: Box<ExprVariant>,
    },
    /// Closure expression: { params in body } or { body }
    Closure {
        lbrace: Span,
        params: Option<ClosureParamsData>,
        in_span: Option<Span>,
        body: Vec<BlockItem>,
        rbrace: Span,
    },
    /// Implicit member access expression: .Case or .Case(args)
    /// Used for enum shorthand: let x: Direction = .north
    ImplicitMemberAccess {
        dot: Span,
        member: Span,
        /// Optional argument list for enum cases with associated values
        arguments: Option<ArgumentListData>,
    },
    /// Match expression: match scrutinee { pattern => expr, ... }
    Match {
        match_span: Span,
        scrutinee: Box<ExprVariant>,
        lbrace: Span,
        arms: Vec<MatchArm>,
        rbrace: Span,
    },
}

/// One entry in a match expression's arm list. Either a well-formed arm or a
/// recovered range covering tokens skipped after a malformed arm.
#[derive(Debug, Clone)]
pub enum MatchArm {
    Arm(MatchArmData),
    /// Tokens skipped by recovery up to (but not including) the next arm
    /// boundary (`,` or `}`). Emitted as a `SyntaxKind::Error` node so the
    /// tree round-trips with the source.
    Recovered(Span),
}

/// Argument list data for implicit member access
#[derive(Debug, Clone)]
pub struct ArgumentListData {
    pub lparen: Span,
    pub arguments: Vec<CallArg>,
    pub commas: Vec<Span>,
    pub rparen: Span,
}

/// Match arm data: pattern [if guard] => expression
#[derive(Debug, Clone)]
pub struct MatchArmData {
    pub pattern: crate::pattern::PatternVariant,
    pub guard: Option<MatchGuardData>,
    pub fat_arrow: Span,
    pub body: Box<ExprVariant>,
}

/// Match guard data: if condition
#[derive(Debug, Clone)]
pub struct MatchGuardData {
    pub if_span: Span,
    pub condition: Box<ExprVariant>,
}

/// Loop label data: label:
#[derive(Debug, Clone)]
pub struct LabelData {
    pub name: Span,
    pub colon: Span,
}

/// Closure parameter list data: (x, y: Int)
#[derive(Debug, Clone)]
pub struct ClosureParamsData {
    pub lparen: Span,
    pub params: Vec<ClosureParamData>,
    pub commas: Vec<Span>,
    pub rparen: Span,
}

/// Single closure parameter: pattern or pattern: Type
///
/// Supports destructuring patterns like `(a, b)` or `Point { x, y }`.
#[derive(Debug, Clone)]
pub struct ClosureParamData {
    /// Span of a leading `mutating` keyword (`Some` ⇒ by-reference param).
    pub mutating: Option<Span>,
    pub pattern: crate::pattern::PatternVariant,
    pub colon: Option<Span>,
    pub ty: Option<TyVariant>,
}

/// Else clause: either a block or another if expression
#[derive(Debug, Clone)]
pub enum ElseClause {
    /// else { block }
    Block {
        else_span: Span,
        block: CodeBlockData,
    },
    /// else if ...
    ElseIf {
        else_span: Span,
        if_expr: Box<ExprVariant>,
    },
}

/// A single condition in an if or if-let expression
#[derive(Debug, Clone)]
pub enum IfCondition {
    /// Boolean expression condition
    Expr(ExprVariant),
    /// Let binding condition: let pattern = expr
    Let {
        let_span: Span,
        pattern: crate::pattern::PatternVariant,
        equals_span: Span,
        value: ExprVariant,
    },
}
