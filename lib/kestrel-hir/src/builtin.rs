//! Well-known builtin types, protocols, and language features.
//!
//! This module defines the full `@builtin(.Feature)` system. Each variant of
//! `Builtin` corresponds to a language feature that can be annotated in stdlib
//! source files with `@builtin(.FeatureName)`.
//!
//! **Resolution**: Forward lookup (entity → Builtin) uses `EntityBuiltin` query
//! in kestrel-name-res. Reverse lookup (Builtin → entity) uses `ResolveBuiltin`
//! with name-based resolution + `BuiltinIndex` attribute-scanning fallback.

/// What kind of symbol a builtin expects, with kind-specific configuration.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinKind {
    /// A builtin protocol.
    Protocol {
        /// If true, types implicitly conform unless opted out with `not Protocol`.
        implicit_conformance: bool,
        /// If true, must be a marker protocol (no required methods/types).
        must_be_marker: bool,
        /// If true, tuples conform if all elements conform.
        tuple_conformance_propagation: bool,
        /// If true, conforming types must have all fields also conform.
        requires_fields_conform: bool,
        /// If true, enums cannot conform to this protocol.
        disallow_enum_conformance: bool,
    },
    /// A builtin protocol method.
    ProtocolMethod,
    /// A builtin struct.
    Struct,
    /// A builtin enum.
    Enum,
    /// A builtin enum case.
    EnumCase,
    /// A builtin function or initializer.
    Function,
    /// A builtin type alias.
    TypeAlias,
}

impl BuiltinKind {
    /// Shorthand for a protocol with all flags false.
    const fn protocol() -> Self {
        Self::Protocol {
            implicit_conformance: false,
            must_be_marker: false,
            tuple_conformance_propagation: false,
            requires_fields_conform: false,
            disallow_enum_conformance: false,
        }
    }

    /// Human-readable kind name for error messages.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Protocol { .. } => "protocol",
            Self::ProtocolMethod => "protocol method",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::EnumCase => "enum case",
            Self::Function => "function",
            Self::TypeAlias => "type alias",
        }
    }
}

/// A well-known type, protocol, or language feature that the compiler needs
/// to reference by entity ID.
///
/// Each variant maps to a `@builtin(.Name)` attribute in stdlib source files.
/// The `kind()` method returns metadata about what symbol kind is expected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Builtin {
    // ===== Well-known types =====
    Bool,

    // ===== Copy/Clone semantics =====
    Copyable,
    Cloneable,
    CloneMethod,

    // ===== Pattern matching =====
    Matchable,
    RangeMatchable,
    RangeMatchableIsAtLeast,
    RangeMatchableIsAtMost,
    RangeMatchableIsBelow,
    ArrayMatchable,
    ArrayMatchableMatchLength,
    ArrayMatchableMatchGet,
    ArrayMatchableMatchSlice,

    // ===== Literal protocols =====
    ExpressibleByIntegerLiteral,
    ExpressibleByFloatLiteral,
    ExpressibleByStringLiteral,
    ExpressibleByBoolLiteral,
    ExpressibleByCharLiteral,
    ExpressibleByNullLiteral,
    InternalExpressibleByArrayLiteral,
    InternalExpressibleByDictionaryLiteral,

    // ===== Default literal types =====
    DefaultIntegerLiteralType,
    DefaultFloatLiteralType,
    DefaultStringLiteralType,
    DefaultBooleanLiteralType,
    DefaultCharLiteralType,
    DefaultNullLiteralType,
    DefaultArrayLiteralType,
    DefaultDictionaryLiteralType,

    // ===== FFI =====
    FFISafe,

    // ===== Arithmetic operators =====
    AddOperatorProtocol,
    AddOperatorMethod,
    SubtractOperatorProtocol,
    SubtractOperatorMethod,
    MultiplyOperatorProtocol,
    MultiplyOperatorMethod,
    DivideOperatorProtocol,
    DivideOperatorMethod,
    ModuloOperatorProtocol,
    ModuloOperatorMethod,
    NegateOperatorProtocol,
    NegateOperatorMethod,

    // ===== Comparison operators =====
    EqualsOperatorProtocol,
    EqualsOperatorMethod,
    NotEqualsOperatorProtocol,
    NotEqualsOperatorMethod,
    LessThanOperatorProtocol,
    LessThanOperatorMethod,
    GreaterThanOperatorProtocol,
    GreaterThanOperatorMethod,
    LessOrEqualOperatorProtocol,
    LessOrEqualOperatorMethod,
    GreaterOrEqualOperatorProtocol,
    GreaterOrEqualOperatorMethod,

    // ===== Bitwise operators =====
    BitwiseAndOperatorProtocol,
    BitwiseAndOperatorMethod,
    BitwiseOrOperatorProtocol,
    BitwiseOrOperatorMethod,
    BitwiseXorOperatorProtocol,
    BitwiseXorOperatorMethod,
    ShiftLeftOperatorProtocol,
    ShiftLeftOperatorMethod,
    ShiftRightOperatorProtocol,
    ShiftRightOperatorMethod,
    BitwiseNotOperatorProtocol,
    BitwiseNotOperatorMethod,

    // ===== Logical / short-circuit operators =====
    LogicalAndOperatorProtocol,
    LogicalAndOperatorMethod,
    LogicalOrOperatorProtocol,
    LogicalOrOperatorMethod,
    LogicalNotOperatorProtocol,
    LogicalNotOperatorMethod,

    // ===== Null coalescing =====
    CoalesceOperatorProtocol,
    CoalesceOperatorMethod,

    // ===== Boolean conditional =====
    BooleanConditional,

    // ===== Range operators =====
    ExclusiveRangeOperatorProtocol,
    ExclusiveRangeOperatorMethod,
    InclusiveRangeOperatorProtocol,
    InclusiveRangeOperatorMethod,

    // ===== Compound assignment =====
    AddAssignProtocol,
    AddAssignMethod,
    SubtractAssignProtocol,
    SubtractAssignMethod,
    MultiplyAssignProtocol,
    MultiplyAssignMethod,
    DivideAssignProtocol,
    DivideAssignMethod,
    ModuloAssignProtocol,
    ModuloAssignMethod,
    BitwiseAndAssignProtocol,
    BitwiseAndAssignMethod,
    BitwiseOrAssignProtocol,
    BitwiseOrAssignMethod,
    BitwiseXorAssignProtocol,
    BitwiseXorAssignMethod,
    ShiftLeftAssignProtocol,
    ShiftLeftAssignMethod,
    ShiftRightAssignProtocol,
    ShiftRightAssignMethod,

    // ===== Try / error handling =====
    ControlFlowEnum,
    TryableProtocol,
    TryExtractMethod,
    FromResidualProtocol,
    FromResidualMethod,

    // ===== Value promotion =====
    FromValueProtocol,
    FromValueMethod,

    // ===== Iterator / Iterable =====
    IteratorProtocol,
    IteratorNextMethod,
    IterableProtocol,
    IterableIterMethod,

    // ===== Optional =====
    OptionalEnum,
    OptionalSomeCase,
    OptionalNoneCase,

    // ===== Type operators (type aliases) =====
    OptionalTypeOperator,
    ArrayTypeOperator,
    DictionaryTypeOperator,
    ResultTypeOperator,

    // ===== Builtin structs =====
    ArrayStruct,
    SliceStruct,

    // ===== String interpolation =====
    DefaultStringInterpolation,
    DefaultStringInterpolationInit,
    DefaultStringInterpolationAppendLiteral,
    DefaultStringInterpolationAppendInterpolation,
    DefaultStringInterpolationBuild,
    FormatOptions,
    FormattableProtocol,
    FormattableFormatIntoMethod,
}

impl Builtin {
    /// The type/protocol name as it appears in Kestrel source code.
    ///
    /// Used by `ResolveBuiltin` for name-based resolution (looking up types
    /// that are auto-imported from std). For features that are NOT resolvable
    /// by source name (protocol methods, enum cases, etc.), this returns the
    /// `@builtin` attribute name instead.
    pub fn name(self) -> &'static str {
        match self {
            // Well-known types — resolved by source name
            Self::Bool => "Bool",

            // Copy/Clone
            Self::Copyable => "Copyable",
            Self::Cloneable => "Cloneable",
            Self::CloneMethod => "Clone",

            // Pattern matching
            Self::Matchable => "Matchable",
            Self::RangeMatchable => "RangeMatchable",
            Self::RangeMatchableIsAtLeast => "RangeMatchableIsAtLeast",
            Self::RangeMatchableIsAtMost => "RangeMatchableIsAtMost",
            Self::RangeMatchableIsBelow => "RangeMatchableIsBelow",
            Self::ArrayMatchable => "ArrayMatchable",
            Self::ArrayMatchableMatchLength => "ArrayMatchableMatchLength",
            Self::ArrayMatchableMatchGet => "ArrayMatchableMatchGet",
            Self::ArrayMatchableMatchSlice => "ArrayMatchableMatchSlice",

            // Literal protocols — resolved by source name
            Self::ExpressibleByIntegerLiteral => "ExpressibleByIntegerLiteral",
            Self::ExpressibleByFloatLiteral => "ExpressibleByFloatLiteral",
            Self::ExpressibleByStringLiteral => "ExpressibleByStringLiteral",
            Self::ExpressibleByBoolLiteral => "ExpressibleByBoolLiteral",
            Self::ExpressibleByCharLiteral => "ExpressibleByCharLiteral",
            Self::ExpressibleByNullLiteral => "ExpressibleByNullLiteral",
            Self::InternalExpressibleByArrayLiteral => "_ExpressibleByArrayLiteral",
            Self::InternalExpressibleByDictionaryLiteral => "_ExpressibleByDictionaryLiteral",

            // Default literal types — resolved by source name
            Self::DefaultIntegerLiteralType => "Int64",
            Self::DefaultFloatLiteralType => "Float64",
            Self::DefaultStringLiteralType => "String",
            Self::DefaultBooleanLiteralType => "Bool",
            Self::DefaultCharLiteralType => "Char",
            Self::DefaultNullLiteralType => "Optional",
            Self::DefaultArrayLiteralType => "Array",
            Self::DefaultDictionaryLiteralType => "Dictionary",

            // FFI
            Self::FFISafe => "FFISafe",

            // Arithmetic operators — protocols resolved by source name
            Self::AddOperatorProtocol => "Addable",
            Self::AddOperatorMethod => "AddOperatorMethod",
            Self::SubtractOperatorProtocol => "Subtractable",
            Self::SubtractOperatorMethod => "SubtractOperatorMethod",
            Self::MultiplyOperatorProtocol => "Multipliable",
            Self::MultiplyOperatorMethod => "MultiplyOperatorMethod",
            Self::DivideOperatorProtocol => "Divisible",
            Self::DivideOperatorMethod => "DivideOperatorMethod",
            Self::ModuloOperatorProtocol => "Modulo",
            Self::ModuloOperatorMethod => "ModuloOperatorMethod",
            Self::NegateOperatorProtocol => "Negatable",
            Self::NegateOperatorMethod => "NegateOperatorMethod",

            // Comparison operators
            Self::EqualsOperatorProtocol => "Equal",
            Self::EqualsOperatorMethod => "EqualsOperatorMethod",
            Self::NotEqualsOperatorProtocol => "NotEqual",
            Self::NotEqualsOperatorMethod => "NotEqualsOperatorMethod",
            Self::LessThanOperatorProtocol => "Less",
            Self::LessThanOperatorMethod => "LessThanOperatorMethod",
            Self::GreaterThanOperatorProtocol => "Greater",
            Self::GreaterThanOperatorMethod => "GreaterThanOperatorMethod",
            Self::LessOrEqualOperatorProtocol => "LessOrEqual",
            Self::LessOrEqualOperatorMethod => "LessOrEqualOperatorMethod",
            Self::GreaterOrEqualOperatorProtocol => "GreaterOrEqual",
            Self::GreaterOrEqualOperatorMethod => "GreaterOrEqualOperatorMethod",

            // Bitwise operators
            Self::BitwiseAndOperatorProtocol => "BitwiseAnd",
            Self::BitwiseAndOperatorMethod => "BitwiseAndOperatorMethod",
            Self::BitwiseOrOperatorProtocol => "BitwiseOr",
            Self::BitwiseOrOperatorMethod => "BitwiseOrOperatorMethod",
            Self::BitwiseXorOperatorProtocol => "BitwiseXor",
            Self::BitwiseXorOperatorMethod => "BitwiseXorOperatorMethod",
            Self::ShiftLeftOperatorProtocol => "LeftShift",
            Self::ShiftLeftOperatorMethod => "ShiftLeftOperatorMethod",
            Self::ShiftRightOperatorProtocol => "RightShift",
            Self::ShiftRightOperatorMethod => "ShiftRightOperatorMethod",
            Self::BitwiseNotOperatorProtocol => "BitwiseNot",
            Self::BitwiseNotOperatorMethod => "BitwiseNotOperatorMethod",

            // Logical / short-circuit operators
            Self::LogicalAndOperatorProtocol => "And",
            Self::LogicalAndOperatorMethod => "LogicalAndOperatorMethod",
            Self::LogicalOrOperatorProtocol => "Or",
            Self::LogicalOrOperatorMethod => "LogicalOrOperatorMethod",
            Self::LogicalNotOperatorProtocol => "Not",
            Self::LogicalNotOperatorMethod => "LogicalNotOperatorMethod",

            // Null coalescing
            Self::CoalesceOperatorProtocol => "Coalesce",
            Self::CoalesceOperatorMethod => "CoalesceOperatorMethod",

            // Boolean conditional
            Self::BooleanConditional => "BooleanConditional",

            // Range operators
            Self::ExclusiveRangeOperatorProtocol => "RangeConstructible",
            Self::ExclusiveRangeOperatorMethod => "ExclusiveRangeOperatorMethod",
            Self::InclusiveRangeOperatorProtocol => "ClosedRangeConstructible",
            Self::InclusiveRangeOperatorMethod => "InclusiveRangeOperatorMethod",

            // Compound assignment
            Self::AddAssignProtocol => "AddAssign",
            Self::AddAssignMethod => "AddAssignMethod",
            Self::SubtractAssignProtocol => "SubtractAssign",
            Self::SubtractAssignMethod => "SubtractAssignMethod",
            Self::MultiplyAssignProtocol => "MultiplyAssign",
            Self::MultiplyAssignMethod => "MultiplyAssignMethod",
            Self::DivideAssignProtocol => "DivideAssign",
            Self::DivideAssignMethod => "DivideAssignMethod",
            Self::ModuloAssignProtocol => "ModuloAssign",
            Self::ModuloAssignMethod => "ModuloAssignMethod",
            Self::BitwiseAndAssignProtocol => "BitwiseAndAssign",
            Self::BitwiseAndAssignMethod => "BitwiseAndAssignMethod",
            Self::BitwiseOrAssignProtocol => "BitwiseOrAssign",
            Self::BitwiseOrAssignMethod => "BitwiseOrAssignMethod",
            Self::BitwiseXorAssignProtocol => "BitwiseXorAssign",
            Self::BitwiseXorAssignMethod => "BitwiseXorAssignMethod",
            Self::ShiftLeftAssignProtocol => "LeftShiftAssign",
            Self::ShiftLeftAssignMethod => "ShiftLeftAssignMethod",
            Self::ShiftRightAssignProtocol => "RightShiftAssign",
            Self::ShiftRightAssignMethod => "ShiftRightAssignMethod",

            // Try / error handling
            Self::ControlFlowEnum => "ControlFlowEnum",
            Self::TryableProtocol => "TryableProtocol",
            Self::TryExtractMethod => "TryExtractMethod",
            Self::FromResidualProtocol => "FromResidualProtocol",
            Self::FromResidualMethod => "FromResidualMethod",

            // Value promotion
            Self::FromValueProtocol => "FromValue",
            Self::FromValueMethod => "FromValueMethod",

            // Iterator / Iterable
            Self::IteratorProtocol => "IteratorProtocol",
            Self::IteratorNextMethod => "IteratorNextMethod",
            Self::IterableProtocol => "IterableProtocol",
            Self::IterableIterMethod => "IterableIterMethod",

            // Optional
            Self::OptionalEnum => "OptionalEnum",
            Self::OptionalSomeCase => "OptionalSomeCase",
            Self::OptionalNoneCase => "OptionalNoneCase",

            // Type operators
            Self::OptionalTypeOperator => "OptionalTypeOperator",
            Self::ArrayTypeOperator => "ArrayTypeOperator",
            Self::DictionaryTypeOperator => "DictionaryTypeOperator",
            Self::ResultTypeOperator => "ResultTypeOperator",

            // Builtin structs
            Self::ArrayStruct => "ArrayStruct",
            Self::SliceStruct => "SliceStruct",

            // String interpolation
            Self::DefaultStringInterpolation => "DefaultStringInterpolation",
            Self::DefaultStringInterpolationInit => "DefaultStringInterpolationInit",
            Self::DefaultStringInterpolationAppendLiteral => {
                "DefaultStringInterpolationAppendLiteral"
            },
            Self::DefaultStringInterpolationAppendInterpolation => {
                "DefaultStringInterpolationAppendInterpolation"
            },
            Self::DefaultStringInterpolationBuild => "DefaultStringInterpolationBuild",
            Self::FormatOptions => "FormatOptions",
            Self::FormattableProtocol => "FormattableProtocol",
            Self::FormattableFormatIntoMethod => "FormattableFormatInto",
        }
    }

    /// Parse a builtin from its `@builtin(.Name)` attribute string (without
    /// the leading dot). Returns None for unknown names.
    ///
    /// This must handle the attribute names as they appear in stdlib `.ks` files.
    /// For operator protocols, the attribute name differs from the source type name
    /// (e.g., attribute "AddOperatorProtocol" maps to the protocol named "Addable").
    pub fn from_attribute_name(name: &str) -> Option<Builtin> {
        match name {
            // Copy/Clone
            "Copyable" => Some(Self::Copyable),
            "Cloneable" => Some(Self::Cloneable),
            "Clone" => Some(Self::CloneMethod),

            // Pattern matching
            "Matchable" => Some(Self::Matchable),
            "RangeMatchable" => Some(Self::RangeMatchable),
            "RangeMatchableIsAtLeast" => Some(Self::RangeMatchableIsAtLeast),
            "RangeMatchableIsAtMost" => Some(Self::RangeMatchableIsAtMost),
            "RangeMatchableIsBelow" => Some(Self::RangeMatchableIsBelow),
            "ArrayMatchable" => Some(Self::ArrayMatchable),
            "ArrayMatchableMatchLength" => Some(Self::ArrayMatchableMatchLength),
            "ArrayMatchableMatchGet" => Some(Self::ArrayMatchableMatchGet),
            "ArrayMatchableMatchSlice" => Some(Self::ArrayMatchableMatchSlice),

            // Literal protocols
            "ExpressibleByIntLiteral" => Some(Self::ExpressibleByIntegerLiteral),
            "ExpressibleByFloatLiteral" => Some(Self::ExpressibleByFloatLiteral),
            "ExpressibleByStringLiteral" => Some(Self::ExpressibleByStringLiteral),
            "ExpressibleByBoolLiteral" => Some(Self::ExpressibleByBoolLiteral),
            "ExpressibleByCharLiteral" => Some(Self::ExpressibleByCharLiteral),
            "ExpressibleByNullLiteral" => Some(Self::ExpressibleByNullLiteral),
            "_ExpressibleByArrayLiteral" => Some(Self::InternalExpressibleByArrayLiteral),
            "_ExpressibleByDictionaryLiteral" => Some(Self::InternalExpressibleByDictionaryLiteral),

            // FFI
            "FFISafe" => Some(Self::FFISafe),

            // Default literal types
            "DefaultIntegerLiteralType" => Some(Self::DefaultIntegerLiteralType),
            "DefaultFloatLiteralType" => Some(Self::DefaultFloatLiteralType),
            "DefaultStringLiteralType" => Some(Self::DefaultStringLiteralType),
            "DefaultBooleanLiteralType" => Some(Self::DefaultBooleanLiteralType),
            "DefaultCharLiteralType" => Some(Self::DefaultCharLiteralType),
            "DefaultNullLiteralType" => Some(Self::DefaultNullLiteralType),
            "DefaultDictionaryLiteralType" => Some(Self::DefaultDictionaryLiteralType),

            // Arithmetic operators
            "AddOperatorProtocol" => Some(Self::AddOperatorProtocol),
            "AddOperatorMethod" => Some(Self::AddOperatorMethod),
            "SubtractOperatorProtocol" => Some(Self::SubtractOperatorProtocol),
            "SubtractOperatorMethod" => Some(Self::SubtractOperatorMethod),
            "MultiplyOperatorProtocol" => Some(Self::MultiplyOperatorProtocol),
            "MultiplyOperatorMethod" => Some(Self::MultiplyOperatorMethod),
            "DivideOperatorProtocol" => Some(Self::DivideOperatorProtocol),
            "DivideOperatorMethod" => Some(Self::DivideOperatorMethod),
            "ModuloOperatorProtocol" => Some(Self::ModuloOperatorProtocol),
            "ModuloOperatorMethod" => Some(Self::ModuloOperatorMethod),
            "NegateOperatorProtocol" => Some(Self::NegateOperatorProtocol),
            "NegateOperatorMethod" => Some(Self::NegateOperatorMethod),

            // Comparison operators
            "EqualsOperatorProtocol" => Some(Self::EqualsOperatorProtocol),
            "EqualsOperatorMethod" => Some(Self::EqualsOperatorMethod),
            "NotEqualsOperatorProtocol" => Some(Self::NotEqualsOperatorProtocol),
            "NotEqualsOperatorMethod" => Some(Self::NotEqualsOperatorMethod),
            "LessThanOperatorProtocol" => Some(Self::LessThanOperatorProtocol),
            "LessThanOperatorMethod" => Some(Self::LessThanOperatorMethod),
            "LessOrEqualOperatorProtocol" => Some(Self::LessOrEqualOperatorProtocol),
            "LessOrEqualOperatorMethod" => Some(Self::LessOrEqualOperatorMethod),
            "GreaterThanOperatorProtocol" => Some(Self::GreaterThanOperatorProtocol),
            "GreaterThanOperatorMethod" => Some(Self::GreaterThanOperatorMethod),
            "GreaterOrEqualOperatorProtocol" => Some(Self::GreaterOrEqualOperatorProtocol),
            "GreaterOrEqualOperatorMethod" => Some(Self::GreaterOrEqualOperatorMethod),

            // Bitwise operators
            "BitwiseAndOperatorProtocol" => Some(Self::BitwiseAndOperatorProtocol),
            "BitwiseAndOperatorMethod" => Some(Self::BitwiseAndOperatorMethod),
            "BitwiseOrOperatorProtocol" => Some(Self::BitwiseOrOperatorProtocol),
            "BitwiseOrOperatorMethod" => Some(Self::BitwiseOrOperatorMethod),
            "BitwiseXorOperatorProtocol" => Some(Self::BitwiseXorOperatorProtocol),
            "BitwiseXorOperatorMethod" => Some(Self::BitwiseXorOperatorMethod),
            "ShiftLeftOperatorProtocol" => Some(Self::ShiftLeftOperatorProtocol),
            "ShiftLeftOperatorMethod" => Some(Self::ShiftLeftOperatorMethod),
            "ShiftRightOperatorProtocol" => Some(Self::ShiftRightOperatorProtocol),
            "ShiftRightOperatorMethod" => Some(Self::ShiftRightOperatorMethod),
            "BitwiseNotOperatorProtocol" => Some(Self::BitwiseNotOperatorProtocol),
            "BitwiseNotOperatorMethod" => Some(Self::BitwiseNotOperatorMethod),

            // Logical operators
            "LogicalAndOperatorProtocol" => Some(Self::LogicalAndOperatorProtocol),
            "LogicalAndOperatorMethod" => Some(Self::LogicalAndOperatorMethod),
            "LogicalOrOperatorProtocol" => Some(Self::LogicalOrOperatorProtocol),
            "LogicalOrOperatorMethod" => Some(Self::LogicalOrOperatorMethod),
            "LogicalNotOperatorProtocol" => Some(Self::LogicalNotOperatorProtocol),
            "LogicalNotOperatorMethod" => Some(Self::LogicalNotOperatorMethod),

            // Null coalescing
            "CoalesceOperatorProtocol" => Some(Self::CoalesceOperatorProtocol),
            "CoalesceOperatorMethod" => Some(Self::CoalesceOperatorMethod),

            // Boolean conditional
            "BooleanConditional" => Some(Self::BooleanConditional),

            // Range operators
            "ExclusiveRangeOperatorProtocol" => Some(Self::ExclusiveRangeOperatorProtocol),
            "ExclusiveRangeOperatorMethod" => Some(Self::ExclusiveRangeOperatorMethod),
            "InclusiveRangeOperatorProtocol" => Some(Self::InclusiveRangeOperatorProtocol),
            "InclusiveRangeOperatorMethod" => Some(Self::InclusiveRangeOperatorMethod),

            // Compound assignment
            "AddAssignProtocol" => Some(Self::AddAssignProtocol),
            "AddAssignMethod" => Some(Self::AddAssignMethod),
            "SubtractAssignProtocol" => Some(Self::SubtractAssignProtocol),
            "SubtractAssignMethod" => Some(Self::SubtractAssignMethod),
            "MultiplyAssignProtocol" => Some(Self::MultiplyAssignProtocol),
            "MultiplyAssignMethod" => Some(Self::MultiplyAssignMethod),
            "DivideAssignProtocol" => Some(Self::DivideAssignProtocol),
            "DivideAssignMethod" => Some(Self::DivideAssignMethod),
            "ModuloAssignProtocol" => Some(Self::ModuloAssignProtocol),
            "ModuloAssignMethod" => Some(Self::ModuloAssignMethod),
            "BitwiseAndAssignProtocol" => Some(Self::BitwiseAndAssignProtocol),
            "BitwiseAndAssignMethod" => Some(Self::BitwiseAndAssignMethod),
            "BitwiseOrAssignProtocol" => Some(Self::BitwiseOrAssignProtocol),
            "BitwiseOrAssignMethod" => Some(Self::BitwiseOrAssignMethod),
            "BitwiseXorAssignProtocol" => Some(Self::BitwiseXorAssignProtocol),
            "BitwiseXorAssignMethod" => Some(Self::BitwiseXorAssignMethod),
            "ShiftLeftAssignProtocol" => Some(Self::ShiftLeftAssignProtocol),
            "ShiftLeftAssignMethod" => Some(Self::ShiftLeftAssignMethod),
            "ShiftRightAssignProtocol" => Some(Self::ShiftRightAssignProtocol),
            "ShiftRightAssignMethod" => Some(Self::ShiftRightAssignMethod),

            // Try / error handling
            "ControlFlowEnum" => Some(Self::ControlFlowEnum),
            "TryableProtocol" => Some(Self::TryableProtocol),
            "TryExtractMethod" => Some(Self::TryExtractMethod),
            "FromResidualProtocol" => Some(Self::FromResidualProtocol),
            "FromResidualMethod" => Some(Self::FromResidualMethod),

            // Value promotion
            "FromValueProtocol" => Some(Self::FromValueProtocol),
            "FromValueMethod" => Some(Self::FromValueMethod),

            // Iterator / Iterable
            "IteratorProtocol" => Some(Self::IteratorProtocol),
            "IteratorNextMethod" => Some(Self::IteratorNextMethod),
            "IterableProtocol" => Some(Self::IterableProtocol),
            "IterableIterMethod" => Some(Self::IterableIterMethod),

            // Optional
            "OptionalEnum" => Some(Self::OptionalEnum),
            "OptionalSomeCase" => Some(Self::OptionalSomeCase),
            "OptionalNoneCase" => Some(Self::OptionalNoneCase),

            // Type operators
            "OptionalTypeOperator" => Some(Self::OptionalTypeOperator),
            "ArrayTypeOperator" => Some(Self::ArrayTypeOperator),
            "DictionaryTypeOperator" => Some(Self::DictionaryTypeOperator),
            "ResultTypeOperator" => Some(Self::ResultTypeOperator),

            // Builtin structs
            "ArrayStruct" => Some(Self::ArrayStruct),
            "SliceStruct" => Some(Self::SliceStruct),

            // String interpolation
            "DefaultStringInterpolation" => Some(Self::DefaultStringInterpolation),
            "DefaultStringInterpolationInit" => Some(Self::DefaultStringInterpolationInit),
            "DefaultStringInterpolationAppendLiteral" => {
                Some(Self::DefaultStringInterpolationAppendLiteral)
            },
            "DefaultStringInterpolationAppendInterpolation" => {
                Some(Self::DefaultStringInterpolationAppendInterpolation)
            },
            "DefaultStringInterpolationBuild" => Some(Self::DefaultStringInterpolationBuild),
            "FormatOptions" => Some(Self::FormatOptions),
            "FormattableProtocol" => Some(Self::FormattableProtocol),
            "FormattableFormatInto" => Some(Self::FormattableFormatIntoMethod),

            _ => None,
        }
    }

    /// Metadata about what symbol kind this builtin expects.
    pub fn kind(self) -> BuiltinKind {
        match self {
            // Copy/Clone
            Self::Copyable => BuiltinKind::Protocol {
                implicit_conformance: true,
                must_be_marker: true,
                tuple_conformance_propagation: false,
                requires_fields_conform: false,
                disallow_enum_conformance: false,
            },
            Self::Cloneable => BuiltinKind::protocol(),
            Self::CloneMethod => BuiltinKind::ProtocolMethod,

            // Pattern matching
            Self::Matchable | Self::RangeMatchable | Self::ArrayMatchable => {
                BuiltinKind::protocol()
            },
            Self::RangeMatchableIsAtLeast
            | Self::RangeMatchableIsAtMost
            | Self::RangeMatchableIsBelow
            | Self::ArrayMatchableMatchLength
            | Self::ArrayMatchableMatchGet
            | Self::ArrayMatchableMatchSlice => BuiltinKind::ProtocolMethod,

            // Literal protocols
            Self::ExpressibleByIntegerLiteral
            | Self::ExpressibleByFloatLiteral
            | Self::ExpressibleByStringLiteral
            | Self::ExpressibleByBoolLiteral
            | Self::ExpressibleByCharLiteral
            | Self::ExpressibleByNullLiteral
            | Self::InternalExpressibleByArrayLiteral
            | Self::InternalExpressibleByDictionaryLiteral => BuiltinKind::protocol(),

            // Default literal types
            Self::DefaultIntegerLiteralType
            | Self::DefaultFloatLiteralType
            | Self::DefaultStringLiteralType
            | Self::DefaultBooleanLiteralType
            | Self::DefaultCharLiteralType
            | Self::DefaultNullLiteralType
            | Self::DefaultArrayLiteralType
            | Self::DefaultDictionaryLiteralType => BuiltinKind::TypeAlias,

            // FFI
            Self::FFISafe => BuiltinKind::Protocol {
                implicit_conformance: false,
                must_be_marker: true,
                tuple_conformance_propagation: true,
                requires_fields_conform: true,
                disallow_enum_conformance: true,
            },

            // Operator protocols (all follow the same pattern)
            Self::AddOperatorProtocol
            | Self::SubtractOperatorProtocol
            | Self::MultiplyOperatorProtocol
            | Self::DivideOperatorProtocol
            | Self::ModuloOperatorProtocol
            | Self::NegateOperatorProtocol
            | Self::EqualsOperatorProtocol
            | Self::NotEqualsOperatorProtocol
            | Self::LessThanOperatorProtocol
            | Self::GreaterThanOperatorProtocol
            | Self::LessOrEqualOperatorProtocol
            | Self::GreaterOrEqualOperatorProtocol
            | Self::BitwiseAndOperatorProtocol
            | Self::BitwiseOrOperatorProtocol
            | Self::BitwiseXorOperatorProtocol
            | Self::ShiftLeftOperatorProtocol
            | Self::ShiftRightOperatorProtocol
            | Self::BitwiseNotOperatorProtocol
            | Self::LogicalAndOperatorProtocol
            | Self::LogicalOrOperatorProtocol
            | Self::LogicalNotOperatorProtocol
            | Self::CoalesceOperatorProtocol
            | Self::ExclusiveRangeOperatorProtocol
            | Self::InclusiveRangeOperatorProtocol
            | Self::AddAssignProtocol
            | Self::SubtractAssignProtocol
            | Self::MultiplyAssignProtocol
            | Self::DivideAssignProtocol
            | Self::ModuloAssignProtocol
            | Self::BitwiseAndAssignProtocol
            | Self::BitwiseOrAssignProtocol
            | Self::BitwiseXorAssignProtocol
            | Self::ShiftLeftAssignProtocol
            | Self::ShiftRightAssignProtocol => BuiltinKind::protocol(),

            // Operator methods
            Self::AddOperatorMethod
            | Self::SubtractOperatorMethod
            | Self::MultiplyOperatorMethod
            | Self::DivideOperatorMethod
            | Self::ModuloOperatorMethod
            | Self::NegateOperatorMethod
            | Self::EqualsOperatorMethod
            | Self::NotEqualsOperatorMethod
            | Self::LessThanOperatorMethod
            | Self::GreaterThanOperatorMethod
            | Self::LessOrEqualOperatorMethod
            | Self::GreaterOrEqualOperatorMethod
            | Self::BitwiseAndOperatorMethod
            | Self::BitwiseOrOperatorMethod
            | Self::BitwiseXorOperatorMethod
            | Self::ShiftLeftOperatorMethod
            | Self::ShiftRightOperatorMethod
            | Self::BitwiseNotOperatorMethod
            | Self::LogicalAndOperatorMethod
            | Self::LogicalOrOperatorMethod
            | Self::LogicalNotOperatorMethod
            | Self::CoalesceOperatorMethod
            | Self::ExclusiveRangeOperatorMethod
            | Self::InclusiveRangeOperatorMethod
            | Self::AddAssignMethod
            | Self::SubtractAssignMethod
            | Self::MultiplyAssignMethod
            | Self::DivideAssignMethod
            | Self::ModuloAssignMethod
            | Self::BitwiseAndAssignMethod
            | Self::BitwiseOrAssignMethod
            | Self::BitwiseXorAssignMethod
            | Self::ShiftLeftAssignMethod
            | Self::ShiftRightAssignMethod => BuiltinKind::ProtocolMethod,

            // Boolean conditional
            Self::BooleanConditional => BuiltinKind::protocol(),

            // Try / error handling
            Self::ControlFlowEnum => BuiltinKind::Enum,
            Self::TryableProtocol | Self::FromResidualProtocol => BuiltinKind::protocol(),
            Self::TryExtractMethod | Self::FromResidualMethod => BuiltinKind::ProtocolMethod,

            // Value promotion
            Self::FromValueProtocol => BuiltinKind::protocol(),
            Self::FromValueMethod => BuiltinKind::ProtocolMethod,

            // Iterator / Iterable
            Self::IteratorProtocol | Self::IterableProtocol => BuiltinKind::protocol(),
            Self::IteratorNextMethod | Self::IterableIterMethod => BuiltinKind::ProtocolMethod,

            // Optional
            Self::OptionalEnum => BuiltinKind::Enum,
            Self::OptionalSomeCase | Self::OptionalNoneCase => BuiltinKind::EnumCase,

            // Type operators
            Self::OptionalTypeOperator
            | Self::ArrayTypeOperator
            | Self::DictionaryTypeOperator
            | Self::ResultTypeOperator => BuiltinKind::TypeAlias,

            // Builtin structs
            Self::ArrayStruct | Self::SliceStruct => BuiltinKind::Struct,

            // String interpolation
            Self::DefaultStringInterpolation | Self::FormatOptions => BuiltinKind::Struct,
            Self::DefaultStringInterpolationInit
            | Self::DefaultStringInterpolationAppendLiteral
            | Self::DefaultStringInterpolationAppendInterpolation
            | Self::DefaultStringInterpolationBuild => BuiltinKind::Function,
            Self::FormattableProtocol => BuiltinKind::protocol(),
            Self::FormattableFormatIntoMethod => BuiltinKind::ProtocolMethod,

            // Well-known types — Bool is resolved by name, doesn't need @builtin
            Self::Bool => BuiltinKind::Struct,
        }
    }
}
