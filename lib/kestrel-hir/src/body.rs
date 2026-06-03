//! HIR body types: expressions, patterns, statements, and their arena container.
//!
//! The HIR is a desugared, partially-resolved representation where:
//! - All syntactic sugar is expanded (operators → protocol calls, for → loop, etc.)
//! - Scope-resolvable names are resolved to entities/locals
//! - Type-dependent names (methods, fields) remain as strings for type inference
//! - Types are resolved to entities (not just names)

use kestrel_ast::arena::{Arena, Idx};
use kestrel_ast::{BinaryOp, CompoundAssignOp, PostfixOp, UnaryOp};
use kestrel_hecs::Entity;
use kestrel_span::Span;

use crate::builtin::Builtin;
use crate::res::{Local, LocalId};
use crate::ty::HirTy;

// ===== Arena index type aliases =====

pub type HirExprId = Idx<HirExpr>;
pub type HirPatId = Idx<HirPat>;
pub type HirStmtId = Idx<HirStmt>;

// ===== Top-level body container =====

/// A function/getter/setter body after HIR lowering: arenas of desugared
/// expressions, patterns, and statements, plus a locals table.
#[derive(Clone, Debug, Hash)]
pub struct HirBody {
    pub exprs: Arena<HirExpr>,
    pub pats: Arena<HirPat>,
    pub stmts: Arena<HirStmt>,
    pub locals: Arena<Local>,
    /// Function parameters in declaration order
    pub params: Vec<LocalId>,
    /// Top-level statements in the body
    pub statements: Vec<HirStmtId>,
    /// Trailing expression (the block's value), if any
    pub tail_expr: Option<HirExprId>,
    /// Statements that originated from guard desugaring.
    /// Used by the guard divergence analyzer to check that the else block diverges.
    pub guard_stmts: Vec<HirStmtId>,
    /// Original condition expressions from while-loop desugaring.
    /// Used by the condition type analyzer to check that while conditions are Bool.
    pub while_conditions: Vec<HirExprId>,
}

/// Where a `HirExpr::Match` came from. Drives diagnostic phrasing and lets
/// analyzers skip desugared matches (for-loop bodies, if-let wildcards) that
/// would otherwise produce false-positive unreachable / irrefutable warnings.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum MatchSource {
    /// `match x { ... }` written by the user.
    UserMatch,
    /// Desugared from `if let p = v { ... } else { ... }`.
    IfLet,
    /// Desugared from `while let p = v { ... }`.
    WhileLet,
    /// Desugared from `guard <condition> else { ... }` (bool-only, no pattern bindings).
    Guard,
    /// CPS-desugared from `guard let p = v else { ... }`: pattern arm = continuation,
    /// wildcard arm = else body (must diverge).
    GuardLet,
    /// Desugared from `for p in iter { ... }` (the Option match on iterator.next()).
    ForLoop,
    /// Desugared from `let <pattern> = expr;`.
    LetDestructure,
    /// Desugared from a destructured fn / method / closure parameter
    /// (`func f((a, b): (I, I))` or `{ ((a, b): (I, I)) in ... }`).
    /// Distinct from `LetDestructure` so inference can skip the cascading
    /// tuple-arity equate when `param_pattern` will emit E111 for the same site.
    ParamDestructure,
    /// Desugared from `try expr` (Continue/Break matching on ControlFlow).
    TryOp,
}

impl MatchSource {
    /// True if this match is a desugared construct whose arms are synthetic
    /// (analyzers should skip exhaustiveness/redundancy checks).
    pub fn is_desugared(self) -> bool {
        !matches!(self, MatchSource::UserMatch)
    }
}

/// Identifies the user-facing language construct that produced a `HirExpr::Sugar`
/// wrapper's `inner` subtree. Lets post-typing analyzers fire user-language
/// diagnostics anchored on the surface keyword without re-deriving the source
/// from protocol identity, and serves as the anchor for cascade suppression
/// (inference emits a kind-specific primary `Conforms` whose failure poisons
/// the receiver TyVar so synthesized inner Member/ImplicitMember errors absorb).
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum SugarKind {
    /// `for pat in iter { ... }` — `inner` is the outer `Block`
    /// (`let $iter = iter.iter(); loop { match $iter.next() { … } }`).
    ForLoop,
    /// `try expr` — `inner` is the `Match { source: TryOp, … }` whose
    /// scrutinee is `ProtocolCall(operand, Tryable, "tryExtract")`.
    Try,
    /// `lhs += rhs` (and other `CompoundAssignOp`s) — `inner` is the
    /// `ProtocolCall(lhs, AddAssign, "addAssign", [rhs])`, or `HirExpr::Error`
    /// if the AST-level place check rejected the LHS at desugar time.
    CompoundAssign,
    /// `"hello \(name)!"` — `inner` is the Block containing the
    /// DefaultStringInterpolation init/append/build sequence.
    StringInterpolation,
}

/// A nested code block (if/loop/match arm bodies, desugared blocks).
#[derive(Clone, Debug, Hash)]
pub struct HirBlock {
    pub stmts: Vec<HirStmtId>,
    pub tail_expr: Option<HirExprId>,
}

/// A name read from a name-bearing HIR position (field accesses, method
/// calls, implicit-member, struct-pat fields, …).
///
/// `Missing` means the parser recovered from an absent identifier (cursor
/// mid-edit, `foo.` with no member yet). Inference must short-circuit to
/// `ResolvedTy::Error` on `Missing` instead of emitting a "name not found"
/// cascade — the parser already reported the gap.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum HirName {
    Name(String),
    Missing,
}

impl HirName {
    /// Convenience constructor for the non-missing case. Use at desugar
    /// sites that hard-code names (operator method names like `"add"`,
    /// for-loop method names like `"next"`) — those are never missing
    /// because the parser doesn't synthesize them.
    pub fn name(s: impl Into<String>) -> Self {
        HirName::Name(s.into())
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            HirName::Name(s) => Some(s),
            HirName::Missing => None,
        }
    }

    /// `&str` view that collapses `Missing` to `""`. Use at boundaries where
    /// a string is structurally required — e.g. building MIR call witness
    /// keys — and you've already verified inference will short-circuit
    /// `Missing` to `Error` upstream so the empty string is unreachable in
    /// practice. Prefer `as_str()` whenever the consumer can branch on the
    /// `Option`.
    pub fn as_str_or_empty(&self) -> &str {
        self.as_str().unwrap_or("")
    }

    pub fn is_missing(&self) -> bool {
        matches!(self, HirName::Missing)
    }
}

impl std::fmt::Display for HirName {
    /// Renders `Missing` as `""`. Same caveats as `as_str_or_empty`: only
    /// used in diagnostic / debug paths that won't fire for missing names
    /// because inference's `HirName::Missing` short-circuit poisons the
    /// expression to `ResolvedTy::Error` first.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str_or_empty())
    }
}

// ===== Expressions (20 variants) =====

/// Desugared expression. All operators, for-loops, while-loops, try/throw,
/// and string interpolation have been lowered into calls and control flow.
#[derive(Clone, Debug, Hash)]
pub enum HirExpr {
    // === Values ===
    Literal {
        value: HirLiteral,
        span: Span,
    },
    Tuple {
        elements: Vec<HirExprId>,
        span: Span,
    },
    Array {
        elements: Vec<HirExprId>,
        span: Span,
    },
    Dict {
        entries: Vec<HirDictEntry>,
        span: Span,
    },
    Closure {
        params: Vec<HirClosureParam>,
        body: HirBlock,
        span: Span,
    },

    // === Resolved references ===
    /// Local variable, resolved by name resolution
    Local(LocalId, Span),
    /// Function, enum case, type, etc. — resolved by name resolution.
    /// Optional type args for explicit generic instantiation (e.g., `Pointer[UInt8]`).
    Def(Entity, Vec<crate::ty::HirTy>, Span),
    /// Multiple overloaded function entities sharing the same name.
    /// Resolved by type inference at the call site via OverloadedCall constraint.
    OverloadSet {
        candidates: Vec<Entity>,
        type_args: Vec<crate::ty::HirTy>,
        span: Span,
    },

    // === Access (member name resolved by type inference) ===
    Field {
        base: HirExprId,
        name: HirName,
        span: Span,
    },
    TupleIndex {
        base: HirExprId,
        index: u32,
        span: Span,
    },
    /// `.Case` or `.Case(args)` — resolved by type inference based on expected type
    ImplicitMember {
        name: HirName,
        args: Option<Vec<HirCallArg>>,
        span: Span,
    },

    // === Calls ===
    /// Direct function/constructor call: `foo(x)`, `Point(x: 1, y: 2)`
    Call {
        callee: HirExprId,
        args: Vec<HirCallArg>,
        span: Span,
    },
    /// User-written method call: `x.foo()` or `x.map[Int](...)`.
    /// Method resolved by type inference.
    MethodCall {
        receiver: HirExprId,
        method: HirName,
        type_args: Option<Vec<HirTy>>,
        args: Vec<HirCallArg>,
        span: Span,
    },
    /// Protocol method call (from desugared operators, for-loops, try, etc.).
    /// Protocol entity is resolved by name resolution. Type inference generates
    /// a conformance constraint: receiver must conform to `protocol`.
    ProtocolCall {
        receiver: HirExprId,
        protocol: Entity,
        method: HirName,
        type_args: Option<Vec<HirTy>>,
        args: Vec<HirCallArg>,
        span: Span,
    },

    // === Control flow ===
    If {
        condition: HirExprId,
        then_body: HirBlock,
        else_body: Option<HirBlock>,
        span: Span,
    },
    Loop {
        label: Option<String>,
        body: HirBlock,
        span: Span,
    },
    Match {
        scrutinee: HirExprId,
        arms: Vec<HirMatchArm>,
        source: MatchSource,
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
        value: Option<HirExprId>,
        span: Span,
    },

    // === Other ===
    Assign {
        target: HirExprId,
        value: HirExprId,
        span: Span,
    },
    /// Block expression: `{ stmts; tail_expr }`.
    /// Used when a match arm body contains statements before its result expression.
    Block {
        body: HirBlock,
        span: Span,
    },
    /// Malformed expression (error recovery)
    Error {
        span: Span,
    },

    /// Transparent wrapper marking that `inner` was synthesized from
    /// desugaring a user-language construct identified by `kind`.
    /// Type-of(Sugar) == type-of(inner); `span` is the user-typed span
    /// (covers the keyword like `for`/`try` or operator like `+=`).
    /// All consumers of `HirExpr` must recurse into `inner` transparently
    /// — see `SugarKind` for the per-kind cascade-suppression contract.
    Sugar {
        kind: SugarKind,
        inner: HirExprId,
        span: Span,
    },
}

// ===== Statements (3 variants) =====

/// HIR statement. Guard is desugared into if + diverging block.
#[derive(Clone, Debug, Hash)]
pub enum HirStmt {
    Let {
        local: LocalId,
        ty: Option<HirTy>,
        value: Option<HirExprId>,
        span: Span,
    },
    Expr {
        expr: HirExprId,
        span: Span,
    },
    /// Destructor registration: `deinit name` binds a cleanup action to a local.
    /// `local` is resolved during HIR lowering; `None` means the name did not
    /// resolve to any in-scope local (diagnostic is emitted at lowering time).
    Deinit {
        name: HirName,
        local: Option<LocalId>,
        span: Span,
    },
    // Guard desugared: if !condition { else_body } where else_body diverges
}

// ===== Patterns (10 variants) =====

/// HIR pattern. `Rest` patterns are absorbed during lowering.
#[derive(Clone, Debug, Hash)]
pub enum HirPat {
    Wildcard {
        span: Span,
    },
    /// Binding resolved to a local slot
    Binding {
        local: LocalId,
        span: Span,
    },
    Tuple {
        prefix: Vec<HirPatId>,
        has_rest: bool,
        suffix: Vec<HirPatId>,
        span: Span,
    },
    Literal {
        value: HirLiteral,
        span: Span,
    },
    Range {
        start: Option<HirLiteral>,
        end: Option<HirLiteral>,
        inclusive: bool,
        span: Span,
    },
    /// Fully qualified variant: resolved by name resolution
    Variant {
        entity: Entity,
        args: Vec<HirPatArg>,
        span: Span,
    },
    /// Implicit variant (`.Case`): resolved by type inference
    ImplicitVariant {
        name: HirName,
        args: Vec<HirPatArg>,
        span: Span,
    },
    Struct {
        entity: Entity,
        fields: Vec<HirStructPatField>,
        has_rest: bool,
        span: Span,
    },
    Array {
        prefix: Vec<HirPatId>,
        /// Rest pattern:
        /// - `None` — no rest (`[a, b]`)
        /// - `Some(None)` — bare rest (`[a, .., b]`)
        /// - `Some(Some(local))` — named rest (`[a, ..name, b]`), bound to `Slice[T]`
        rest: Option<Option<LocalId>>,
        suffix: Vec<HirPatId>,
        span: Span,
    },
    Or {
        alternatives: Vec<HirPatId>,
        span: Span,
    },
    /// `name @ subpattern` — binds the whole matched value while also matching a subpattern.
    At {
        binding: LocalId,
        subpattern: HirPatId,
        span: Span,
    },
    /// Error recovery
    Error {
        span: Span,
    },
}

// ===== Supporting types =====

/// Parsed literal value. Unlike `AstLiteral` which stores source text,
/// HIR literals have been parsed into their concrete types.
#[derive(Clone, Debug, PartialEq)]
pub enum HirLiteral {
    Integer(i64),
    Float(f64),
    /// Decoded string value plus any escape-sequence errors discovered during
    /// lowering. Errors are data on the node — a separate analyzer turns them
    /// into diagnostics. Codegen consumes `value` directly.
    String {
        value: String,
        escape_errors: Vec<EscapeError>,
    },
    /// Unicode scalar value (must be a valid `char`, i.e. `<= 0x10FFFF` and not a surrogate)
    Char(u32),
    Bool(bool),
    Null,
}

/// An escape-sequence error discovered while decoding a string literal.
/// Stored on `HirLiteral::String` and surfaced by the string-escape analyzer.
#[derive(Clone, Debug, PartialEq)]
pub struct EscapeError {
    pub span: Span,
    pub kind: EscapeErrorKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EscapeErrorKind {
    /// Unknown backslash escape (e.g. `\q`) or malformed `\xNN`.
    InvalidEscape { sequence: String },
    /// `\xNN` with value > 0x7F — strings only allow 7-bit ASCII via `\x`.
    AsciiEscapeOutOfRange { value: u8 },
    /// Trailing `\` at end of string.
    IncompleteEscape,
    /// `\u{...}` malformed in some way; `reason` distinguishes.
    InvalidUnicodeEscape {
        value: String,
        reason: UnicodeEscapeErrorReason,
    },
    /// A line in a multi-line string body has less indentation than the
    /// closing `"""` delimiter.
    MultilineUnderIndented,
    /// Multi-line string opener `"""` must be followed immediately by a
    /// newline.
    MultilineMissingLeadingNewline,
    /// Multi-line string closer `"""` must be on its own line (only
    /// whitespace before it on that line).
    MultilineMissingTrailingNewline,
    /// String literal has no closing delimiter.
    UnterminatedString,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UnicodeEscapeErrorReason {
    MissingOpenBrace,
    MissingCloseBrace,
    EmptyBraces,
    TooManyDigits,
    InvalidHexDigit,
    OutOfRange,
}

/// Manual Hash because f64 doesn't implement Hash.
/// We hash the bit representation which is deterministic.
/// String escape errors are derived from `value` + source spans, so we hash
/// `value` only — two literals with the same decoded value hash equal even
/// if their error lists differ in span detail.
impl std::hash::Hash for HirLiteral {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            HirLiteral::Integer(v) => v.hash(state),
            HirLiteral::Float(v) => v.to_bits().hash(state),
            HirLiteral::String { value, .. } => value.hash(state),
            HirLiteral::Char(v) => v.hash(state),
            HirLiteral::Bool(v) => v.hash(state),
            HirLiteral::Null => {},
        }
    }
}

/// A single argument in a call expression.
#[derive(Clone, Debug, Hash)]
pub struct HirCallArg {
    pub label: Option<String>,
    pub value: HirExprId,
}

/// A key-value entry in a dictionary literal.
#[derive(Clone, Debug, Hash)]
pub struct HirDictEntry {
    pub key: HirExprId,
    pub value: HirExprId,
}

/// A single arm in a match expression.
#[derive(Clone, Debug, Hash)]
pub struct HirMatchArm {
    pub pattern: HirPatId,
    pub guard: Option<HirExprId>,
    pub body: HirExprId,
}

/// A parameter in a closure expression.
///
/// Destructured closure params (`{ (a, b) in … }`) are desugared in hir-lower
/// into a synthetic `local` plus a prepended `match` statement. The original
/// pattern is still recorded in `pattern` so analyzers can validate it
/// without re-reading the AST.
#[derive(Clone, Debug, Hash)]
pub struct HirClosureParam {
    pub local: LocalId,
    pub ty: Option<HirTy>,
    /// Present for destructured params (tuple/struct). `None` for simple
    /// bindings and wildcards (the `local` already captures those).
    pub pattern: Option<HirPatId>,
    /// `true` when the param was written `mutating` (by-reference). Inference
    /// may additionally treat a param as `MutBorrow` based on the expected
    /// type even when this is `false`.
    pub is_mut: bool,
}

/// A single argument in an enum/variant pattern.
#[derive(Clone, Debug, Hash)]
pub struct HirPatArg {
    pub label: Option<String>,
    pub pattern: HirPatId,
}

/// A single field in a struct pattern.
#[derive(Clone, Debug, Hash)]
pub struct HirStructPatField {
    pub field_name: HirName,
    pub pattern: Option<HirPatId>,
}

// ===== Operator desugaring tables =====
//
// Each entry maps an operator to its protocol name + method name.
// The protocol entity is resolved from the DefMap during HIR lowering.
// Adding a new operator = one table entry, no new HIR variants needed.

/// (operator, protocol_name, method_name, arg_label)
///
/// Label is `None` for single-name params (no external label in Kestrel),
/// `Some("by")` for shift ops, `Some("to")` for range ops.
pub const BINARY_OP_PROTOCOLS: &[(BinaryOp, Builtin, &str, Option<&str>)] = &[
    (BinaryOp::Add, Builtin::AddOperatorProtocol, "add", None),
    (
        BinaryOp::Sub,
        Builtin::SubtractOperatorProtocol,
        "subtract",
        None,
    ),
    (
        BinaryOp::Mul,
        Builtin::MultiplyOperatorProtocol,
        "multiply",
        None,
    ),
    (
        BinaryOp::Div,
        Builtin::DivideOperatorProtocol,
        "divide",
        None,
    ),
    (
        BinaryOp::Rem,
        Builtin::ModuloOperatorProtocol,
        "modulo",
        None,
    ),
    (
        BinaryOp::Eq,
        Builtin::EqualsOperatorProtocol,
        "equal",
        Some("to"),
    ),
    (
        BinaryOp::Ne,
        Builtin::NotEqualsOperatorProtocol,
        "notEqual",
        Some("to"),
    ),
    (
        BinaryOp::Lt,
        Builtin::LessThanOperatorProtocol,
        "lessThan",
        None,
    ),
    (
        BinaryOp::Gt,
        Builtin::GreaterThanOperatorProtocol,
        "greaterThan",
        None,
    ),
    (
        BinaryOp::Le,
        Builtin::LessOrEqualOperatorProtocol,
        "lessThanOrEqual",
        None,
    ),
    (
        BinaryOp::Ge,
        Builtin::GreaterOrEqualOperatorProtocol,
        "greaterThanOrEqual",
        None,
    ),
    (
        BinaryOp::BitAnd,
        Builtin::BitwiseAndOperatorProtocol,
        "bitwiseAnd",
        None,
    ),
    (
        BinaryOp::BitOr,
        Builtin::BitwiseOrOperatorProtocol,
        "bitwiseOr",
        None,
    ),
    (
        BinaryOp::BitXor,
        Builtin::BitwiseXorOperatorProtocol,
        "bitwiseXor",
        None,
    ),
    (
        BinaryOp::Shl,
        Builtin::ShiftLeftOperatorProtocol,
        "shiftLeft",
        Some("by"),
    ),
    (
        BinaryOp::Shr,
        Builtin::ShiftRightOperatorProtocol,
        "shiftRight",
        Some("by"),
    ),
    (
        BinaryOp::RangeInclusive,
        Builtin::InclusiveRangeOperatorProtocol,
        "inclusiveRange",
        Some("to"),
    ),
    (
        BinaryOp::RangeExclusive,
        Builtin::ExclusiveRangeOperatorProtocol,
        "exclusiveRange",
        Some("to"),
    ),
];

/// Short-circuit operators: right operand is wrapped in a closure.
/// `logicalAnd(other:)` and `logicalOr(other:)` are single-name params (no label).
/// `coalesce(default:)` is also single-name (no label).
pub const SHORT_CIRCUIT_OP_PROTOCOLS: &[(BinaryOp, Builtin, &str, Option<&str>)] = &[
    (
        BinaryOp::And,
        Builtin::LogicalAndOperatorProtocol,
        "logicalAnd",
        None,
    ),
    (
        BinaryOp::Or,
        Builtin::LogicalOrOperatorProtocol,
        "logicalOr",
        None,
    ),
    (
        BinaryOp::Coalesce,
        Builtin::CoalesceOperatorProtocol,
        "coalesce",
        None,
    ),
];

/// (operator, protocol_builtin, method_name)
pub const UNARY_OP_PROTOCOLS: &[(UnaryOp, Builtin, &str)] = &[
    (UnaryOp::Neg, Builtin::NegateOperatorProtocol, "negate"),
    (
        UnaryOp::BitNot,
        Builtin::BitwiseNotOperatorProtocol,
        "bitwiseNot",
    ),
    (
        UnaryOp::LogicalNot,
        Builtin::LogicalNotOperatorProtocol,
        "logicalNot",
    ),
    (
        UnaryOp::RangeUpTo,
        Builtin::RangeUpToOperatorProtocol,
        "rangeUpTo",
    ),
    (
        UnaryOp::RangeThrough,
        Builtin::RangeThroughOperatorProtocol,
        "rangeThrough",
    ),
];

/// (operator, protocol_builtin, method_name)
pub const POSTFIX_OP_PROTOCOLS: &[(PostfixOp, Builtin, &str)] = &[
    (
        PostfixOp::RangeFrom,
        Builtin::RangeFromOperatorProtocol,
        "rangeFrom",
    ),
    (
        PostfixOp::Unwrap,
        Builtin::ForceUnwrapOperatorProtocol,
        "forceUnwrap",
    ),
];

/// (operator, protocol_builtin, method_name, arg_label)
///
/// Most compound assign methods use single-name params (no label).
/// Only shift-assign ops have a `"by"` label.
pub const COMPOUND_ASSIGN_PROTOCOLS: &[(CompoundAssignOp, Builtin, &str, Option<&str>)] = &[
    (
        CompoundAssignOp::AddAssign,
        Builtin::AddAssignProtocol,
        "addAssign",
        None,
    ),
    (
        CompoundAssignOp::SubAssign,
        Builtin::SubtractAssignProtocol,
        "subtractAssign",
        None,
    ),
    (
        CompoundAssignOp::MulAssign,
        Builtin::MultiplyAssignProtocol,
        "multiplyAssign",
        None,
    ),
    (
        CompoundAssignOp::DivAssign,
        Builtin::DivideAssignProtocol,
        "divideAssign",
        None,
    ),
    (
        CompoundAssignOp::RemAssign,
        Builtin::ModuloAssignProtocol,
        "modAssign",
        None,
    ),
    (
        CompoundAssignOp::BitAndAssign,
        Builtin::BitwiseAndAssignProtocol,
        "bitwiseAndAssign",
        None,
    ),
    (
        CompoundAssignOp::BitOrAssign,
        Builtin::BitwiseOrAssignProtocol,
        "bitwiseOrAssign",
        None,
    ),
    (
        CompoundAssignOp::BitXorAssign,
        Builtin::BitwiseXorAssignProtocol,
        "bitwiseXorAssign",
        None,
    ),
    (
        CompoundAssignOp::ShlAssign,
        Builtin::ShiftLeftAssignProtocol,
        "shiftLeftAssign",
        Some("by"),
    ),
    (
        CompoundAssignOp::ShrAssign,
        Builtin::ShiftRightAssignProtocol,
        "shiftRightAssign",
        Some("by"),
    ),
];

/// Look up the protocol for a binary operator.
/// Returns `(protocol_builtin, method_name, arg_label)` or `None` if not found.
pub fn lookup_binary_op(op: &BinaryOp) -> Option<(Builtin, &'static str, Option<&'static str>)> {
    BINARY_OP_PROTOCOLS
        .iter()
        .find(|(o, ..)| o == op)
        .map(|(_, proto, method, label)| (*proto, *method, *label))
}

/// Look up the protocol for a short-circuit binary operator.
/// Returns `(protocol_builtin, method_name, arg_label)` or `None` if not found.
pub fn lookup_short_circuit_op(
    op: &BinaryOp,
) -> Option<(Builtin, &'static str, Option<&'static str>)> {
    SHORT_CIRCUIT_OP_PROTOCOLS
        .iter()
        .find(|(o, ..)| o == op)
        .map(|(_, proto, method, label)| (*proto, *method, *label))
}

/// Look up the protocol for a unary operator.
/// Returns `(protocol_builtin, method_name)` or `None` if not found.
pub fn lookup_unary_op(op: &UnaryOp) -> Option<(Builtin, &'static str)> {
    UNARY_OP_PROTOCOLS
        .iter()
        .find(|(o, ..)| o == op)
        .map(|(_, proto, method)| (*proto, *method))
}

/// Look up the protocol for a postfix operator.
/// Returns `(protocol_builtin, method_name)` or `None` if not found.
pub fn lookup_postfix_op(op: &PostfixOp) -> Option<(Builtin, &'static str)> {
    POSTFIX_OP_PROTOCOLS
        .iter()
        .find(|(o, ..)| o == op)
        .map(|(_, proto, method)| (*proto, *method))
}

/// Look up the protocol for a compound assignment operator.
/// Returns `(protocol_builtin, method_name, arg_label)` or `None` if not found.
pub fn lookup_compound_assign_op(
    op: &CompoundAssignOp,
) -> Option<(Builtin, &'static str, Option<&'static str>)> {
    COMPOUND_ASSIGN_PROTOCOLS
        .iter()
        .find(|(o, ..)| o == op)
        .map(|(_, proto, method, label)| (*proto, *method, *label))
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast::arena::Arena;

    #[test]
    fn arena_alloc_hir_expr() {
        let mut exprs: Arena<HirExpr> = Arena::new();
        let span = Span::synthetic(0);

        let lit = exprs.alloc(HirExpr::Literal {
            value: HirLiteral::Integer(42),
            span: span.clone(),
        });
        let ret = exprs.alloc(HirExpr::Return {
            value: Some(lit),
            span: span.clone(),
        });

        // Verify we can index back into the arena
        assert!(matches!(
            &exprs[lit],
            HirExpr::Literal {
                value: HirLiteral::Integer(42),
                ..
            }
        ));
        assert!(matches!(
            &exprs[ret],
            HirExpr::Return { value: Some(_), .. }
        ));
        assert_eq!(exprs.len(), 2);
    }

    #[test]
    fn arena_alloc_hir_pat() {
        let mut pats: Arena<HirPat> = Arena::new();
        let span = Span::synthetic(0);

        let w = pats.alloc(HirPat::Wildcard { span: span.clone() });
        let err = pats.alloc(HirPat::Error { span });

        assert!(matches!(&pats[w], HirPat::Wildcard { .. }));
        assert!(matches!(&pats[err], HirPat::Error { .. }));
    }

    #[test]
    fn binary_op_table_coverage() {
        // Every BinaryOp should be in either BINARY_OP_PROTOCOLS or SHORT_CIRCUIT_OP_PROTOCOLS
        let all_ops = [
            BinaryOp::Add,
            BinaryOp::Sub,
            BinaryOp::Mul,
            BinaryOp::Div,
            BinaryOp::Rem,
            BinaryOp::BitAnd,
            BinaryOp::BitOr,
            BinaryOp::BitXor,
            BinaryOp::Shl,
            BinaryOp::Shr,
            BinaryOp::Eq,
            BinaryOp::Ne,
            BinaryOp::Lt,
            BinaryOp::Gt,
            BinaryOp::Le,
            BinaryOp::Ge,
            BinaryOp::And,
            BinaryOp::Or,
            BinaryOp::Coalesce,
            BinaryOp::RangeInclusive,
            BinaryOp::RangeExclusive,
        ];

        for op in &all_ops {
            let found = lookup_binary_op(op).is_some() || lookup_short_circuit_op(op).is_some();
            assert!(found, "BinaryOp::{op:?} has no protocol mapping");
        }
    }

    #[test]
    fn unary_op_table_coverage() {
        // Neg, BitNot, LogicalNot should all have mappings.
        // Pos (+x) is a no-op and intentionally omitted.
        assert!(lookup_unary_op(&UnaryOp::Neg).is_some());
        assert!(lookup_unary_op(&UnaryOp::BitNot).is_some());
        assert!(lookup_unary_op(&UnaryOp::LogicalNot).is_some());
        assert!(lookup_unary_op(&UnaryOp::Pos).is_none()); // intentionally no protocol
    }

    #[test]
    fn compound_assign_table_coverage() {
        let all_ops = [
            CompoundAssignOp::AddAssign,
            CompoundAssignOp::SubAssign,
            CompoundAssignOp::MulAssign,
            CompoundAssignOp::DivAssign,
            CompoundAssignOp::RemAssign,
            CompoundAssignOp::BitAndAssign,
            CompoundAssignOp::BitOrAssign,
            CompoundAssignOp::BitXorAssign,
            CompoundAssignOp::ShlAssign,
            CompoundAssignOp::ShrAssign,
        ];

        for op in &all_ops {
            assert!(
                lookup_compound_assign_op(op).is_some(),
                "CompoundAssignOp::{op:?} has no protocol mapping"
            );
        }
    }

    #[test]
    fn hir_literal_equality() {
        assert_eq!(HirLiteral::Integer(42), HirLiteral::Integer(42));
        assert_ne!(HirLiteral::Integer(1), HirLiteral::Integer(2));
        assert_eq!(HirLiteral::Bool(true), HirLiteral::Bool(true));
        assert_eq!(HirLiteral::Null, HirLiteral::Null);
        assert_eq!(
            HirLiteral::String {
                value: "hello".into(),
                escape_errors: vec![]
            },
            HirLiteral::String {
                value: "hello".into(),
                escape_errors: vec![]
            }
        );
    }
}
