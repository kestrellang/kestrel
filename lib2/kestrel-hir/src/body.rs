//! HIR body types: expressions, patterns, statements, and their arena container.
//!
//! The HIR is a desugared, partially-resolved representation where:
//! - All syntactic sugar is expanded (operators → protocol calls, for → loop, etc.)
//! - Scope-resolvable names are resolved to entities/locals
//! - Type-dependent names (methods, fields) remain as strings for type inference
//! - Types are resolved to entities (not just names)

use kestrel_ast::arena::{Arena, Idx};
use kestrel_ast::{BinaryOp, CompoundAssignOp, UnaryOp};
use kestrel_hecs::Entity;
use kestrel_span2::Span;

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
}

/// A nested code block (if/loop/match arm bodies, desugared blocks).
#[derive(Clone, Debug, Hash)]
pub struct HirBlock {
    pub stmts: Vec<HirStmtId>,
    pub tail_expr: Option<HirExprId>,
}

// ===== Expressions (19 variants) =====

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
    /// Function, enum case, type, etc. — resolved by name resolution
    Def(Entity, Span),

    // === Access (member name resolved by type inference) ===
    Field {
        base: HirExprId,
        name: String,
        span: Span,
    },
    TupleIndex {
        base: HirExprId,
        index: u32,
        span: Span,
    },
    /// `.Case` or `.Case(args)` — resolved by type inference based on expected type
    ImplicitMember {
        name: String,
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
        method: String,
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
        method: String,
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
    /// Malformed expression (error recovery)
    Error {
        span: Span,
    },
}

// ===== Statements (3 variants) =====

/// HIR statement. GuardLet is desugared into if + diverging block.
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
    /// Destructor registration: `deinit name` binds a cleanup action to a local
    Deinit {
        name: String,
        span: Span,
    },
    // GuardLet desugared: if !condition { else_body } where else_body diverges
}

// ===== Patterns (10 variants) =====

/// HIR pattern. `At` and `Rest` patterns are absorbed during lowering.
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
        elements: Vec<HirPatId>,
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
        name: String,
        args: Vec<HirPatArg>,
        span: Span,
    },
    Struct {
        entity: Entity,
        fields: Vec<HirStructPatField>,
        has_rest: bool,
        span: Span,
    },
    Or {
        alternatives: Vec<HirPatId>,
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
    String(String),
    /// Unicode scalar value (must be a valid `char`, i.e. `<= 0x10FFFF` and not a surrogate)
    Char(u32),
    Bool(bool),
    Null,
}

/// Manual Hash because f64 doesn't implement Hash.
/// We hash the bit representation which is deterministic.
impl std::hash::Hash for HirLiteral {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            HirLiteral::Integer(v) => v.hash(state),
            HirLiteral::Float(v) => v.to_bits().hash(state),
            HirLiteral::String(v) => v.hash(state),
            HirLiteral::Char(v) => v.hash(state),
            HirLiteral::Bool(v) => v.hash(state),
            HirLiteral::Null => {}
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
#[derive(Clone, Debug, Hash)]
pub struct HirClosureParam {
    pub local: LocalId,
    pub ty: Option<HirTy>,
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
    pub field_name: String,
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
    (BinaryOp::Add, Builtin::Addable, "add", None),
    (BinaryOp::Sub, Builtin::Subtractable, "subtract", None),
    (BinaryOp::Mul, Builtin::Multipliable, "multiply", None),
    (BinaryOp::Div, Builtin::Divisible, "divide", None),
    (BinaryOp::Rem, Builtin::Modulo, "modulo", None),
    (BinaryOp::Eq, Builtin::Equal, "equals", None),
    (BinaryOp::Ne, Builtin::NotEqual, "notEquals", None),
    (BinaryOp::Lt, Builtin::Less, "lessThan", None),
    (BinaryOp::Gt, Builtin::Greater, "greaterThan", None),
    (BinaryOp::Le, Builtin::LessOrEqual, "lessThanOrEqual", None),
    (BinaryOp::Ge, Builtin::GreaterOrEqual, "greaterThanOrEqual", None),
    (BinaryOp::BitAnd, Builtin::BitwiseAnd, "bitwiseAnd", None),
    (BinaryOp::BitOr, Builtin::BitwiseOr, "bitwiseOr", None),
    (BinaryOp::BitXor, Builtin::BitwiseXor, "bitwiseXor", None),
    (BinaryOp::Shl, Builtin::LeftShift, "shiftLeft", Some("by")),
    (BinaryOp::Shr, Builtin::RightShift, "shiftRight", Some("by")),
    (BinaryOp::RangeInclusive, Builtin::ClosedRangeConstructible, "inclusiveRange", Some("to")),
    (BinaryOp::RangeExclusive, Builtin::RangeConstructible, "exclusiveRange", Some("to")),
];

/// Short-circuit operators: right operand is wrapped in a closure.
/// `logicalAnd(other:)` and `logicalOr(other:)` are single-name params (no label).
/// `coalesce(default:)` is also single-name (no label).
pub const SHORT_CIRCUIT_OP_PROTOCOLS: &[(BinaryOp, Builtin, &str, Option<&str>)] = &[
    (BinaryOp::And, Builtin::And, "logicalAnd", None),
    (BinaryOp::Or, Builtin::Or, "logicalOr", None),
    (BinaryOp::Coalesce, Builtin::Coalesce, "coalesce", None),
];

/// (operator, protocol_builtin, method_name)
pub const UNARY_OP_PROTOCOLS: &[(UnaryOp, Builtin, &str)] = &[
    (UnaryOp::Neg, Builtin::Negatable, "negate"),
    (UnaryOp::BitNot, Builtin::BitwiseNot, "bitwiseNot"),
    (UnaryOp::LogicalNot, Builtin::Not, "logicalNot"),
];

/// (operator, protocol_builtin, method_name, arg_label)
///
/// Most compound assign methods use single-name params (no label).
/// Only shift-assign ops have a `"by"` label.
pub const COMPOUND_ASSIGN_PROTOCOLS: &[(CompoundAssignOp, Builtin, &str, Option<&str>)] = &[
    (CompoundAssignOp::AddAssign, Builtin::AddAssign, "addAssign", None),
    (CompoundAssignOp::SubAssign, Builtin::SubtractAssign, "subtractAssign", None),
    (CompoundAssignOp::MulAssign, Builtin::MultiplyAssign, "multiplyAssign", None),
    (CompoundAssignOp::DivAssign, Builtin::DivideAssign, "divideAssign", None),
    (CompoundAssignOp::RemAssign, Builtin::ModuloAssign, "modAssign", None),
    (CompoundAssignOp::BitAndAssign, Builtin::BitwiseAndAssign, "bitwiseAndAssign", None),
    (CompoundAssignOp::BitOrAssign, Builtin::BitwiseOrAssign, "bitwiseOrAssign", None),
    (CompoundAssignOp::BitXorAssign, Builtin::BitwiseXorAssign, "bitwiseXorAssign", None),
    (CompoundAssignOp::ShlAssign, Builtin::LeftShiftAssign, "shiftLeftAssign", Some("by")),
    (CompoundAssignOp::ShrAssign, Builtin::RightShiftAssign, "shiftRightAssign", Some("by")),
];

/// Look up the protocol for a binary operator.
/// Returns `(protocol_builtin, method_name, arg_label)` or `None` if not found.
pub fn lookup_binary_op(
    op: &BinaryOp,
) -> Option<(Builtin, &'static str, Option<&'static str>)> {
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
        assert!(matches!(&exprs[lit], HirExpr::Literal { value: HirLiteral::Integer(42), .. }));
        assert!(matches!(&exprs[ret], HirExpr::Return { value: Some(_), .. }));
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
            let found =
                lookup_binary_op(op).is_some() || lookup_short_circuit_op(op).is_some();
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
            HirLiteral::String("hello".into()),
            HirLiteral::String("hello".into())
        );
    }
}
