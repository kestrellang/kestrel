//! Well-known builtin types and protocols.
//!
//! These are types/protocols that the compiler needs to reference by identity
//! (entity ID) rather than by string name. Resolved once via `ResolveBuiltin`
//! query in kestrel-name-res, then cached.

/// A well-known type or protocol that the compiler needs to resolve by entity ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Builtin {
    // ===== Well-known types =====
    Bool,

    // ===== Literal protocols =====
    ExpressibleByIntegerLiteral,
    ExpressibleByFloatLiteral,
    ExpressibleByStringLiteral,
    ExpressibleByBoolLiteral,
    ExpressibleByCharLiteral,
    ExpressibleByNullLiteral,
    ExpressibleByArrayLiteral,
    ExpressibleByDictionaryLiteral,

    // ===== Default literal types =====
    DefaultIntegerLiteralType,
    DefaultFloatLiteralType,
    DefaultStringLiteralType,
    DefaultBooleanLiteralType,
    DefaultCharLiteralType,
    DefaultNullLiteralType,
    DefaultArrayLiteralType,
    DefaultDictionaryLiteralType,

    // ===== Value promotion =====
    FromValue,

    // ===== Comparison operator protocols =====
    Equal,
    NotEqual,
    Less,
    Greater,
    LessOrEqual,
    GreaterOrEqual,

    // ===== Arithmetic operator protocols =====
    Addable,
    Subtractable,
    Multipliable,
    Divisible,
    Modulo,

    // ===== Bitwise operator protocols =====
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,

    // ===== Logical / short-circuit operator protocols =====
    And,
    Or,
    Coalesce,
    Not,

    // ===== Unary operator protocols =====
    Negatable,
    BitwiseNot,

    // ===== Compound assignment protocols =====
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    ModuloAssign,
    BitwiseAndAssign,
    BitwiseOrAssign,
    BitwiseXorAssign,
    LeftShiftAssign,
    RightShiftAssign,

    // ===== Range protocols =====
    ClosedRangeConstructible,
    RangeConstructible,
}

impl Builtin {
    /// The type/protocol name as it appears in Kestrel source code.
    pub fn name(self) -> &'static str {
        match self {
            // Well-known types
            Self::Bool => "Bool",

            // Literal protocols
            Self::ExpressibleByIntegerLiteral => "ExpressibleByIntegerLiteral",
            Self::ExpressibleByFloatLiteral => "ExpressibleByFloatLiteral",
            Self::ExpressibleByStringLiteral => "ExpressibleByStringLiteral",
            Self::ExpressibleByBoolLiteral => "ExpressibleByBoolLiteral",
            Self::ExpressibleByCharLiteral => "ExpressibleByCharLiteral",
            Self::ExpressibleByNullLiteral => "ExpressibleByNullLiteral",
            Self::ExpressibleByArrayLiteral => "ExpressibleByArrayLiteral",
            Self::ExpressibleByDictionaryLiteral => "ExpressibleByDictionaryLiteral",

            // Default literal types
            Self::DefaultIntegerLiteralType => "Int64",
            Self::DefaultFloatLiteralType => "Float64",
            Self::DefaultStringLiteralType => "String",
            Self::DefaultBooleanLiteralType => "Bool",
            Self::DefaultCharLiteralType => "Char",
            Self::DefaultNullLiteralType => "Optional",
            Self::DefaultArrayLiteralType => "Array",
            Self::DefaultDictionaryLiteralType => "Dictionary",

            // Value promotion
            Self::FromValue => "FromValue",

            // Comparison
            Self::Equal => "Equal",
            Self::NotEqual => "NotEqual",
            Self::Less => "Less",
            Self::Greater => "Greater",
            Self::LessOrEqual => "LessOrEqual",
            Self::GreaterOrEqual => "GreaterOrEqual",

            // Arithmetic
            Self::Addable => "Addable",
            Self::Subtractable => "Subtractable",
            Self::Multipliable => "Multipliable",
            Self::Divisible => "Divisible",
            Self::Modulo => "Modulo",

            // Bitwise
            Self::BitwiseAnd => "BitwiseAnd",
            Self::BitwiseOr => "BitwiseOr",
            Self::BitwiseXor => "BitwiseXor",
            Self::LeftShift => "LeftShift",
            Self::RightShift => "RightShift",

            // Logical / short-circuit
            Self::And => "And",
            Self::Or => "Or",
            Self::Coalesce => "Coalesce",
            Self::Not => "Not",

            // Unary
            Self::Negatable => "Negatable",
            Self::BitwiseNot => "BitwiseNot",

            // Compound assignment
            Self::AddAssign => "AddAssign",
            Self::SubtractAssign => "SubtractAssign",
            Self::MultiplyAssign => "MultiplyAssign",
            Self::DivideAssign => "DivideAssign",
            Self::ModuloAssign => "ModuloAssign",
            Self::BitwiseAndAssign => "BitwiseAndAssign",
            Self::BitwiseOrAssign => "BitwiseOrAssign",
            Self::BitwiseXorAssign => "BitwiseXorAssign",
            Self::LeftShiftAssign => "LeftShiftAssign",
            Self::RightShiftAssign => "RightShiftAssign",

            // Range
            Self::ClosedRangeConstructible => "ClosedRangeConstructible",
            Self::RangeConstructible => "RangeConstructible",
        }
    }
}
