//! Operator registry for Pratt parsing and method desugaring.
//!
//! This module defines the operator precedence and associativity rules
//! used during semantic analysis to restructure flat binary expressions
//! into properly nested trees and desugar them into method calls.

use crate::builtins::LanguageFeature;
use kestrel_syntax_tree::SyntaxKind;
use std::collections::HashMap;

/// Binary operators that can appear between two expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    // Exponential (shift)
    Shl,
    Shr,
    // Multiplicative
    Mul,
    Div,
    Rem,
    BitAnd,
    // Additive
    Add,
    Sub,
    BitOr,
    BitXor,
    // Range
    RangeInclusive,
    RangeExclusive,
    // Comparative
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    // Logical
    And,
    Or,
    // Coalesce
    Coalesce,
}

impl BinaryOp {
    /// Get the method name that this operator desugars to.
    pub fn method_name(&self) -> &'static str {
        match self {
            BinaryOp::Add => "add",
            BinaryOp::Sub => "subtract",
            BinaryOp::Mul => "multiply",
            BinaryOp::Div => "divide",
            BinaryOp::Rem => "modulo",
            BinaryOp::BitAnd => "bitwiseAnd",
            BinaryOp::BitOr => "bitwiseOr",
            BinaryOp::BitXor => "bitwiseXor",
            BinaryOp::Shl => "shiftLeft",
            BinaryOp::Shr => "shiftRight",
            BinaryOp::Eq => "equals",
            BinaryOp::Ne => "notEquals",
            BinaryOp::Lt => "lessThan",
            BinaryOp::Gt => "greaterThan",
            BinaryOp::Le => "lessThanOrEqual",
            BinaryOp::Ge => "greaterThanOrEqual",
            BinaryOp::And => "logicalAnd",
            BinaryOp::Or => "logicalOr",
            BinaryOp::RangeInclusive => "inclusiveRange",
            BinaryOp::RangeExclusive => "exclusiveRange",
            BinaryOp::Coalesce => "coalesce",
        }
    }

    /// Get the symbol representation of this operator for error messages.
    pub fn symbol(&self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Rem => "%",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::Shl => "<<",
            BinaryOp::Shr => ">>",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Gt => ">",
            BinaryOp::Le => "<=",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "and",
            BinaryOp::Or => "or",
            BinaryOp::RangeInclusive => "..=",
            BinaryOp::RangeExclusive => "..<",
            BinaryOp::Coalesce => "??",
        }
    }

    /// Get the language feature for the protocol method that implements this operator.
    /// Returns `None` for operators like Coalesce that don't have protocol methods.
    pub fn method_feature(&self) -> Option<LanguageFeature> {
        match self {
            BinaryOp::Add => Some(LanguageFeature::AddOperatorMethod),
            BinaryOp::Sub => Some(LanguageFeature::SubtractOperatorMethod),
            BinaryOp::Mul => Some(LanguageFeature::MultiplyOperatorMethod),
            BinaryOp::Div => Some(LanguageFeature::DivideOperatorMethod),
            BinaryOp::Rem => Some(LanguageFeature::ModuloOperatorMethod),
            BinaryOp::BitAnd => Some(LanguageFeature::BitwiseAndOperatorMethod),
            BinaryOp::BitOr => Some(LanguageFeature::BitwiseOrOperatorMethod),
            BinaryOp::BitXor => Some(LanguageFeature::BitwiseXorOperatorMethod),
            BinaryOp::Shl => Some(LanguageFeature::ShiftLeftOperatorMethod),
            BinaryOp::Shr => Some(LanguageFeature::ShiftRightOperatorMethod),
            BinaryOp::Eq => Some(LanguageFeature::EqualsOperatorMethod),
            BinaryOp::Ne => Some(LanguageFeature::NotEqualsOperatorMethod),
            BinaryOp::Lt => Some(LanguageFeature::LessThanOperatorMethod),
            BinaryOp::Gt => Some(LanguageFeature::GreaterThanOperatorMethod),
            BinaryOp::Le => Some(LanguageFeature::LessOrEqualOperatorMethod),
            BinaryOp::Ge => Some(LanguageFeature::GreaterOrEqualOperatorMethod),
            BinaryOp::And => Some(LanguageFeature::LogicalAndOperatorMethod),
            BinaryOp::Or => Some(LanguageFeature::LogicalOrOperatorMethod),
            BinaryOp::RangeInclusive => Some(LanguageFeature::InclusiveRangeOperatorMethod),
            BinaryOp::RangeExclusive => Some(LanguageFeature::ExclusiveRangeOperatorMethod),
            BinaryOp::Coalesce => None, // No protocol for coalesce
        }
    }
}

/// Compound assignment operators (+=, -=, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompoundOp {
    Add,      // +=
    Sub,      // -=
    Mul,      // *=
    Div,      // /=
    Rem,      // %=
    BitAnd,   // &=
    BitOr,    // |=
    BitXor,   // ^=
    Shl,      // <<=
    Shr,      // >>=
}

impl CompoundOp {
    /// Get the method name that this compound operator desugars to.
    pub fn method_name(&self) -> &'static str {
        match self {
            CompoundOp::Add => "addAssign",
            CompoundOp::Sub => "subtractAssign",
            CompoundOp::Mul => "multiplyAssign",
            CompoundOp::Div => "divideAssign",
            CompoundOp::Rem => "modAssign",
            CompoundOp::BitAnd => "bitwiseAndAssign",
            CompoundOp::BitOr => "bitwiseOrAssign",
            CompoundOp::BitXor => "bitwiseXorAssign",
            CompoundOp::Shl => "shiftLeftAssign",
            CompoundOp::Shr => "shiftRightAssign",
        }
    }

    /// Get the symbol representation of this operator for error messages.
    pub fn symbol(&self) -> &'static str {
        match self {
            CompoundOp::Add => "+=",
            CompoundOp::Sub => "-=",
            CompoundOp::Mul => "*=",
            CompoundOp::Div => "/=",
            CompoundOp::Rem => "%=",
            CompoundOp::BitAnd => "&=",
            CompoundOp::BitOr => "|=",
            CompoundOp::BitXor => "^=",
            CompoundOp::Shl => "<<=",
            CompoundOp::Shr => ">>=",
        }
    }

    /// Convert from a SyntaxKind token to a CompoundOp.
    pub fn from_syntax_kind(kind: SyntaxKind) -> Option<CompoundOp> {
        match kind {
            SyntaxKind::PlusEquals => Some(CompoundOp::Add),
            SyntaxKind::MinusEquals => Some(CompoundOp::Sub),
            SyntaxKind::StarEquals => Some(CompoundOp::Mul),
            SyntaxKind::SlashEquals => Some(CompoundOp::Div),
            SyntaxKind::PercentEquals => Some(CompoundOp::Rem),
            SyntaxKind::AmpersandEquals => Some(CompoundOp::BitAnd),
            SyntaxKind::PipeEquals => Some(CompoundOp::BitOr),
            SyntaxKind::CaretEquals => Some(CompoundOp::BitXor),
            SyntaxKind::LessLessEquals => Some(CompoundOp::Shl),
            SyntaxKind::GreaterGreaterEquals => Some(CompoundOp::Shr),
            _ => None,
        }
    }

    /// Get the language feature for this compound operator's method, if any.
    pub fn method_feature(&self) -> LanguageFeature {
        match self {
            CompoundOp::Add => LanguageFeature::AddAssignMethod,
            CompoundOp::Sub => LanguageFeature::SubtractAssignMethod,
            CompoundOp::Mul => LanguageFeature::MultiplyAssignMethod,
            CompoundOp::Div => LanguageFeature::DivideAssignMethod,
            CompoundOp::Rem => LanguageFeature::ModuloAssignMethod,
            CompoundOp::BitAnd => LanguageFeature::BitwiseAndAssignMethod,
            CompoundOp::BitOr => LanguageFeature::BitwiseOrAssignMethod,
            CompoundOp::BitXor => LanguageFeature::BitwiseXorAssignMethod,
            CompoundOp::Shl => LanguageFeature::ShiftLeftAssignMethod,
            CompoundOp::Shr => LanguageFeature::ShiftRightAssignMethod,
        }
    }
}

/// Unary operators (both prefix and postfix).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    // Prefix
    Neg,        // -x
    BitNot,     // !x (prefix)
    LogicalNot, // not x
    // Postfix
    Unwrap, // x!
}

impl UnaryOp {
    /// Get the method name that this operator desugars to.
    pub fn method_name(&self) -> &'static str {
        match self {
            UnaryOp::Neg => "negate",
            UnaryOp::BitNot => "bitwiseNot",
            UnaryOp::LogicalNot => "logicalNot",
            UnaryOp::Unwrap => "unwrap",
        }
    }

    /// Get the symbol representation of this operator for error messages.
    pub fn symbol(&self) -> &'static str {
        match self {
            UnaryOp::Neg => "-",
            UnaryOp::BitNot => "!",
            UnaryOp::LogicalNot => "not",
            UnaryOp::Unwrap => "!",
        }
    }

    /// Get the language feature for the protocol method that implements this operator.
    /// Returns `None` for operators like Unwrap that don't have protocol methods.
    pub fn method_feature(&self) -> Option<LanguageFeature> {
        match self {
            UnaryOp::Neg => Some(LanguageFeature::NegateOperatorMethod),
            UnaryOp::BitNot => Some(LanguageFeature::BitwiseNotOperatorMethod),
            UnaryOp::LogicalNot => Some(LanguageFeature::LogicalNotOperatorMethod),
            UnaryOp::Unwrap => None, // No protocol for unwrap
        }
    }
}

/// Result of querying the registry for what to do with an infix/postfix operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfixAction {
    /// Stop parsing, return current lhs
    Stop,
    /// Parse infix: left-associative (precedence included for recursive call)
    InfixLeft(BinaryOp, u8),
    /// Parse infix: right-associative
    InfixRight(BinaryOp, u8),
    /// Apply postfix operator
    Postfix(UnaryOp),
}

/// Result of querying the registry for a prefix operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefixAction {
    pub op: UnaryOp,
    pub precedence: u8,
}

/// Associativity of an operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Associativity {
    Left,
    Right,
}

/// Internal entry for infix operators.
#[derive(Debug, Clone, Copy)]
struct InfixEntry {
    op: BinaryOp,
    precedence: u8,
    associativity: Associativity,
}

/// Internal entry for prefix operators.
#[derive(Debug, Clone, Copy)]
struct PrefixEntry {
    op: UnaryOp,
    precedence: u8,
}

/// Internal entry for postfix operators.
#[derive(Debug, Clone, Copy)]
struct PostfixEntry {
    op: UnaryOp,
    precedence: u8,
}

/// Registry of operators with their precedence and associativity.
///
/// The registry is used during Pratt parsing to determine how to
/// restructure flat binary expressions into properly nested trees.
pub struct OperatorRegistry {
    prefix: HashMap<SyntaxKind, PrefixEntry>,
    infix: HashMap<SyntaxKind, InfixEntry>,
    postfix: HashMap<SyntaxKind, PostfixEntry>,
}

impl Default for OperatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl OperatorRegistry {
    /// Create a new operator registry with all built-in operators.
    pub fn new() -> Self {
        let mut registry = Self {
            prefix: HashMap::new(),
            infix: HashMap::new(),
            postfix: HashMap::new(),
        };
        registry.register_builtins();
        registry
    }

    fn register_builtins(&mut self) {
        // Precedence levels (higher = tighter binding)
        const DISJUNCTIVE: u8 = 10;
        const CONJUNCTIVE: u8 = 20;
        const COMPARATIVE: u8 = 30;
        const RANGE: u8 = 40;
        const ADDITIVE: u8 = 50;
        const MULTIPLICATIVE: u8 = 60;
        const EXPONENTIAL: u8 = 70;
        const PREFIX: u8 = 80;
        const POSTFIX: u8 = 90;

        use Associativity::*;
        use BinaryOp::*;
        use UnaryOp::*;

        // Postfix operators
        self.postfix.insert(
            SyntaxKind::Bang,
            PostfixEntry {
                op: Unwrap,
                precedence: POSTFIX,
            },
        );

        // Prefix operators
        self.prefix.insert(
            SyntaxKind::Minus,
            PrefixEntry {
                op: Neg,
                precedence: PREFIX,
            },
        );
        self.prefix.insert(
            SyntaxKind::Bang,
            PrefixEntry {
                op: BitNot,
                precedence: PREFIX,
            },
        );
        self.prefix.insert(
            SyntaxKind::Not,
            PrefixEntry {
                op: LogicalNot,
                precedence: PREFIX,
            },
        );

        // Exponential (left-assoc)
        self.infix.insert(
            SyntaxKind::LessLess,
            InfixEntry {
                op: Shl,
                precedence: EXPONENTIAL,
                associativity: Left,
            },
        );
        self.infix.insert(
            SyntaxKind::GreaterGreater,
            InfixEntry {
                op: Shr,
                precedence: EXPONENTIAL,
                associativity: Left,
            },
        );

        // Multiplicative (left-assoc)
        self.infix.insert(
            SyntaxKind::Star,
            InfixEntry {
                op: Mul,
                precedence: MULTIPLICATIVE,
                associativity: Left,
            },
        );
        self.infix.insert(
            SyntaxKind::Slash,
            InfixEntry {
                op: Div,
                precedence: MULTIPLICATIVE,
                associativity: Left,
            },
        );
        self.infix.insert(
            SyntaxKind::Percent,
            InfixEntry {
                op: Rem,
                precedence: MULTIPLICATIVE,
                associativity: Left,
            },
        );
        self.infix.insert(
            SyntaxKind::Ampersand,
            InfixEntry {
                op: BitAnd,
                precedence: MULTIPLICATIVE,
                associativity: Left,
            },
        );

        // Additive (left-assoc)
        self.infix.insert(
            SyntaxKind::Plus,
            InfixEntry {
                op: Add,
                precedence: ADDITIVE,
                associativity: Left,
            },
        );
        self.infix.insert(
            SyntaxKind::Minus,
            InfixEntry {
                op: Sub,
                precedence: ADDITIVE,
                associativity: Left,
            },
        );
        self.infix.insert(
            SyntaxKind::Pipe,
            InfixEntry {
                op: BitOr,
                precedence: ADDITIVE,
                associativity: Left,
            },
        );
        self.infix.insert(
            SyntaxKind::Caret,
            InfixEntry {
                op: BitXor,
                precedence: ADDITIVE,
                associativity: Left,
            },
        );

        // Range (left-assoc)
        self.infix.insert(
            SyntaxKind::DotDotEquals,
            InfixEntry {
                op: RangeInclusive,
                precedence: RANGE,
                associativity: Left,
            },
        );
        self.infix.insert(
            SyntaxKind::DotDotLess,
            InfixEntry {
                op: RangeExclusive,
                precedence: RANGE,
                associativity: Left,
            },
        );

        // Comparative (left-assoc)
        self.infix.insert(
            SyntaxKind::EqualsEquals,
            InfixEntry {
                op: Eq,
                precedence: COMPARATIVE,
                associativity: Left,
            },
        );
        self.infix.insert(
            SyntaxKind::BangEquals,
            InfixEntry {
                op: Ne,
                precedence: COMPARATIVE,
                associativity: Left,
            },
        );
        self.infix.insert(
            SyntaxKind::Less,
            InfixEntry {
                op: Lt,
                precedence: COMPARATIVE,
                associativity: Left,
            },
        );
        self.infix.insert(
            SyntaxKind::Greater,
            InfixEntry {
                op: Gt,
                precedence: COMPARATIVE,
                associativity: Left,
            },
        );
        self.infix.insert(
            SyntaxKind::LessEquals,
            InfixEntry {
                op: Le,
                precedence: COMPARATIVE,
                associativity: Left,
            },
        );
        self.infix.insert(
            SyntaxKind::GreaterEquals,
            InfixEntry {
                op: Ge,
                precedence: COMPARATIVE,
                associativity: Left,
            },
        );

        // Conjunctive (left-assoc)
        self.infix.insert(
            SyntaxKind::And,
            InfixEntry {
                op: And,
                precedence: CONJUNCTIVE,
                associativity: Left,
            },
        );

        // Disjunctive (left-assoc)
        self.infix.insert(
            SyntaxKind::Or,
            InfixEntry {
                op: Or,
                precedence: DISJUNCTIVE,
                associativity: Left,
            },
        );
        self.infix.insert(
            SyntaxKind::QuestionQuestion,
            InfixEntry {
                op: Coalesce,
                precedence: DISJUNCTIVE,
                associativity: Left,
            },
        );
    }

    /// Get the prefix action for a token, if it's a valid prefix operator.
    pub fn prefix(&self, token: SyntaxKind) -> Option<PrefixAction> {
        self.prefix.get(&token).map(|entry| PrefixAction {
            op: entry.op,
            precedence: entry.precedence,
        })
    }

    /// Determine what action to take given current token and minimum precedence.
    pub fn infix_action(&self, token: SyntaxKind, min_precedence: u8) -> InfixAction {
        // Check postfix first (higher precedence)
        if let Some(entry) = self.postfix.get(&token)
            && entry.precedence >= min_precedence
        {
            return InfixAction::Postfix(entry.op);
        }

        // Check infix
        if let Some(entry) = self.infix.get(&token)
            && entry.precedence >= min_precedence
        {
            return match entry.associativity {
                Associativity::Left => InfixAction::InfixLeft(entry.op, entry.precedence),
                Associativity::Right => InfixAction::InfixRight(entry.op, entry.precedence),
            };
        }

        InfixAction::Stop
    }

    /// Register a prefix operator (for future extensibility).
    pub fn register_prefix(&mut self, token: SyntaxKind, op: UnaryOp, precedence: u8) {
        self.prefix.insert(token, PrefixEntry { op, precedence });
    }

    /// Register an infix operator (for future extensibility).
    pub fn register_infix(
        &mut self,
        token: SyntaxKind,
        op: BinaryOp,
        precedence: u8,
        right_associative: bool,
    ) {
        self.infix.insert(
            token,
            InfixEntry {
                op,
                precedence,
                associativity: if right_associative {
                    Associativity::Right
                } else {
                    Associativity::Left
                },
            },
        );
    }

    /// Register a postfix operator (for future extensibility).
    pub fn register_postfix(&mut self, token: SyntaxKind, op: UnaryOp, precedence: u8) {
        self.postfix.insert(token, PostfixEntry { op, precedence });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precedence_mul_higher_than_add() {
        let registry = OperatorRegistry::new();

        // At precedence 0, both + and * should be available
        match registry.infix_action(SyntaxKind::Plus, 0) {
            InfixAction::InfixLeft(BinaryOp::Add, add_prec) => {
                match registry.infix_action(SyntaxKind::Star, 0) {
                    InfixAction::InfixLeft(BinaryOp::Mul, mul_prec) => {
                        assert!(
                            mul_prec > add_prec,
                            "* should have higher precedence than +"
                        );
                    },
                    _ => panic!("Expected InfixLeft for *"),
                }
            },
            _ => panic!("Expected InfixLeft for +"),
        }
    }

    #[test]
    fn test_prefix_operators() {
        let registry = OperatorRegistry::new();

        assert!(registry.prefix(SyntaxKind::Minus).is_some());
        assert!(registry.prefix(SyntaxKind::Plus).is_none()); // Identity operator removed
        assert!(registry.prefix(SyntaxKind::Bang).is_some());
        assert!(registry.prefix(SyntaxKind::Not).is_some());
    }

    #[test]
    fn test_postfix_bang() {
        let registry = OperatorRegistry::new();

        match registry.infix_action(SyntaxKind::Bang, 0) {
            InfixAction::Postfix(UnaryOp::Unwrap) => {},
            _ => panic!("Expected Postfix(Unwrap) for !"),
        }
    }

    #[test]
    fn test_method_names() {
        assert_eq!(BinaryOp::Add.method_name(), "add");
        assert_eq!(BinaryOp::Sub.method_name(), "subtract");
        assert_eq!(BinaryOp::Eq.method_name(), "equals");
        assert_eq!(UnaryOp::Neg.method_name(), "negate");
        assert_eq!(UnaryOp::Unwrap.method_name(), "unwrap");
    }
}
