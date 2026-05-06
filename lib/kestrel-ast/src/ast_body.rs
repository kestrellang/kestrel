//! AST body types: expressions, patterns, statements, and their arena container.
//!
//! These are unresolved — paths are just names, no symbol resolution, no types
//! embedded. Position-independent for incremental caching.

use kestrel_span::Span;

use crate::arena::{Arena, Idx};
use crate::ast_type::AstType;

// ===== Arena index type aliases =====

pub type ExprId = Idx<AstExpr>;
pub type PatId = Idx<AstPat>;
pub type StmtId = Idx<AstStmt>;

// ===== Top-level body container =====

/// A function/getter/default-value body: arenas of expressions, patterns,
/// and statements, plus the top-level statement list and optional tail expression.
#[derive(Clone, Debug)]
pub struct AstBody {
    pub exprs: Arena<AstExpr>,
    pub pats: Arena<AstPat>,
    pub stmts: Arena<AstStmt>,
    /// Top-level statements in the body.
    pub statements: Vec<StmtId>,
    /// Trailing expression (the block's value), if any.
    pub tail_expr: Option<ExprId>,
}

/// A nested code block (if/while/for/etc. bodies).
#[derive(Clone, Debug)]
pub struct AstBlock {
    pub stmts: Vec<StmtId>,
    pub tail_expr: Option<ExprId>,
}

// ===== Expressions =====

#[derive(Clone, Debug)]
pub enum AstExpr {
    Literal {
        kind: AstLiteral,
        span: Span,
    },
    InterpolatedString {
        parts: Vec<StringPart>,
        span: Span,
    },
    Array {
        elements: Vec<ExprId>,
        span: Span,
    },
    Dictionary {
        entries: Vec<DictEntry>,
        span: Span,
    },
    Tuple {
        elements: Vec<ExprId>,
        span: Span,
    },
    /// Simple path: `a`, `a.b.c` (no computed base expression)
    Path {
        segments: Vec<ExprPathSegment>,
        span: Span,
    },
    /// Member access on an expression: `expr.member`
    MemberAccess {
        base: ExprId,
        member: String,
        type_args: Option<Vec<AstType>>,
        span: Span,
    },
    /// Tuple field access: `expr.0`
    TupleIndex {
        base: ExprId,
        index: u32,
        span: Span,
    },
    /// Implicit member (enum shorthand): `.Case` or `.Case(args)`
    ImplicitMember {
        member: String,
        arguments: Option<Vec<CallArg>>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        operand: ExprId,
        span: Span,
    },
    Postfix {
        operand: ExprId,
        op: PostfixOp,
        span: Span,
    },
    Binary {
        lhs: ExprId,
        op: BinaryOp,
        rhs: ExprId,
        span: Span,
    },
    Assignment {
        lhs: ExprId,
        rhs: ExprId,
        span: Span,
    },
    CompoundAssignment {
        lhs: ExprId,
        op: CompoundAssignOp,
        rhs: ExprId,
        span: Span,
    },
    Call {
        callee: ExprId,
        arguments: Vec<CallArg>,
        span: Span,
    },
    If {
        conditions: Vec<IfCondition>,
        then_body: AstBlock,
        else_body: Option<ElseBody>,
        span: Span,
    },
    While {
        label: Option<String>,
        condition: ExprId,
        body: AstBlock,
        span: Span,
    },
    WhileLet {
        label: Option<String>,
        conditions: Vec<IfCondition>,
        body: AstBlock,
        span: Span,
    },
    Loop {
        label: Option<String>,
        body: AstBlock,
        span: Span,
    },
    For {
        label: Option<String>,
        pattern: PatId,
        iterable: ExprId,
        body: AstBlock,
        span: Span,
    },
    Break {
        label: Option<String>,
        span: Span,
    },
    Continue {
        label: Option<String>,
        span: Span,
    },
    Return {
        value: Option<ExprId>,
        span: Span,
    },
    Throw {
        value: ExprId,
        span: Span,
    },
    Try {
        operand: ExprId,
        span: Span,
    },
    Closure {
        params: Vec<ClosureParam>,
        body: AstBlock,
        span: Span,
    },
    Match {
        scrutinee: ExprId,
        arms: Vec<MatchArm>,
        span: Span,
    },
    /// Block expression: `{ stmts; tail_expr }`.
    /// Used when the parser treats a match arm body block as a closure.
    Block {
        body: AstBlock,
        span: Span,
    },
    /// Parenthesized expression: `(expr)`. Preserved so binary-operator
    /// flattening doesn't merge across user-written grouping.
    Paren {
        inner: ExprId,
        span: Span,
    },
    /// Malformed expression (parse error recovery)
    Error {
        span: Span,
    },
}

// ===== Statements =====

#[derive(Clone, Debug)]
pub enum AstStmt {
    Let {
        is_mut: bool,
        pattern: PatId,
        ty: Option<AstType>,
        value: Option<ExprId>,
        span: Span,
    },
    Expr {
        expr: ExprId,
        span: Span,
    },
    Guard {
        conditions: Vec<IfCondition>,
        else_body: AstBlock,
        span: Span,
    },
    Deinit {
        name: String,
        span: Span,
    },
}

// ===== Patterns =====

#[derive(Clone, Debug)]
pub enum AstPat {
    Wildcard {
        span: Span,
    },
    Binding {
        is_mut: bool,
        name: String,
        span: Span,
    },
    Tuple {
        prefix: Vec<PatId>,
        has_rest: bool,
        /// True if >1 rest pattern found (error case)
        multiple_rests: bool,
        suffix: Vec<PatId>,
        span: Span,
    },
    Literal {
        kind: LitPatKind,
        span: Span,
    },
    Range {
        start: Option<LitPatKind>,
        end: Option<LitPatKind>,
        inclusive: bool,
        span: Span,
    },
    Enum {
        case_name: String,
        args: Vec<EnumPatArg>,
        span: Span,
    },
    Struct {
        name: String,
        fields: Vec<StructPatField>,
        has_rest: bool,
        span: Span,
    },
    Array {
        prefix: Vec<PatId>,
        rest: Option<Option<String>>,
        suffix: Vec<PatId>,
        span: Span,
    },
    At {
        is_mut: bool,
        name: String,
        subpattern: PatId,
        span: Span,
    },
    Or {
        alternatives: Vec<PatId>,
        span: Span,
    },
    Rest {
        span: Span,
    },
    Error {
        span: Span,
    },
}

// ===== Supporting types =====

/// Literal value kinds for expressions.
#[derive(Clone, Debug)]
pub enum AstLiteral {
    Integer(String),
    Float(String),
    String(String),
    RawString(String),
    Char(String),
    Bool(bool),
    Null,
    Unit,
}

/// Parts of an interpolated string.
#[derive(Clone, Debug)]
pub enum StringPart {
    /// Literal text segment.
    Literal(String),
    /// `\(expr)` or `\(expr:format)` interpolation.
    Interpolation {
        expr: ExprId,
        format: Option<String>,
    },
}

/// A single argument in a call expression.
#[derive(Clone, Debug)]
pub struct CallArg {
    pub label: Option<String>,
    pub value: ExprId,
}

/// A key-value entry in a dictionary literal.
#[derive(Clone, Debug)]
pub struct DictEntry {
    pub key: ExprId,
    pub value: ExprId,
}

/// A segment in an expression path (e.g. `Foo[Int].bar`).
#[derive(Clone, Debug)]
pub struct ExprPathSegment {
    pub name: String,
    pub type_args: Option<Vec<AstType>>,
    pub span: Span,
}

/// Condition in if-let / while-let / guard-let chains.
#[derive(Clone, Debug)]
pub enum IfCondition {
    /// Plain boolean expression.
    Expr(ExprId),
    /// `let pattern = value` binding condition.
    Let { pattern: PatId, value: ExprId },
}

/// The else branch of an if expression.
#[derive(Clone, Debug)]
pub enum ElseBody {
    Block(AstBlock),
    ElseIf(ExprId),
}

/// A single arm in a match expression.
#[derive(Clone, Debug)]
pub struct MatchArm {
    pub pattern: PatId,
    pub guard: Option<ExprId>,
    pub body: ExprId,
}

/// A parameter in a closure expression.
#[derive(Clone, Debug)]
pub struct ClosureParam {
    pub pattern: PatId,
    pub ty: Option<AstType>,
}

// ===== Operators =====

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Neg,
    BitNot,
    LogicalNot,
    Pos,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PostfixOp {
    Unwrap,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Coalesce,
    RangeInclusive,
    RangeExclusive,
}

impl BinaryOp {
    /// Binding power for Pratt parsing (higher = tighter binding).
    pub fn precedence(&self) -> u8 {
        match self {
            BinaryOp::Or => 10,
            BinaryOp::Coalesce => 15,
            BinaryOp::And => 20,
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Gt
            | BinaryOp::Le
            | BinaryOp::Ge => 30,
            BinaryOp::RangeInclusive | BinaryOp::RangeExclusive => 40,
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::BitOr | BinaryOp::BitXor => 50,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem | BinaryOp::BitAnd => 60,
            BinaryOp::Shl | BinaryOp::Shr => 70,
        }
    }

    /// Whether this operator is right-associative (only Coalesce).
    pub fn is_right_assoc(&self) -> bool {
        matches!(self, BinaryOp::Coalesce)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CompoundAssignOp {
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
}

// ===== Pattern helpers =====

/// Literal value kinds for patterns (stored as source text).
#[derive(Clone, Debug)]
pub enum LitPatKind {
    Integer(String),
    Float(String),
    String(String),
    Bool(bool),
    Char(String),
}

/// A single argument in an enum pattern (e.g. `.Some(label: pat)`).
#[derive(Clone, Debug)]
pub struct EnumPatArg {
    pub label: Option<String>,
    pub pattern: PatId,
}

/// A single field in a struct pattern (e.g. `Point { x: a, y }`).
#[derive(Clone, Debug)]
pub struct StructPatField {
    pub field_name: String,
    pub pattern: Option<PatId>,
}
