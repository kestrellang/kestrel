//! Builtin language features and registry.
//!
//! This module defines the `@builtin(.Feature)` system for marking
//! protocols, structs, enums, functions, and variables as language builtins.

use parking_lot::RwLock;
use semantic_tree::symbol::SymbolId;
use std::collections::HashMap;

/// Language features that can be marked with `@builtin(.Feature)`.
///
/// Each feature defines what kind of symbol it expects and any
/// validation requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageFeature {
    // Protocol builtins - copy semantics
    Copyable,

    // Protocol builtins - clone semantics
    Cloneable,
    Clone,

    // Protocol builtins - literal expressibility
    ExpressibleByIntLiteral,
    ExpressibleByFloatLiteral,
    ExpressibleByStringLiteral,
    ExpressibleByBoolLiteral,
    ExpressibleByNilLiteral,
    ExpressibleByArrayLiteral,
    ExpressibleByDictionaryLiteral,

    // Protocol builtins - FFI
    FFISafe,

    // Type alias builtins - default literal types
    DefaultIntegerLiteralType,
    DefaultFloatLiteralType,

    // Operator protocols - arithmetic
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

    // Operator protocols - comparison
    EqualsOperatorProtocol,
    EqualsOperatorMethod,
    NotEqualsOperatorProtocol,
    NotEqualsOperatorMethod,
    LessThanOperatorProtocol,
    LessThanOperatorMethod,
    LessOrEqualOperatorProtocol,
    LessOrEqualOperatorMethod,
    GreaterThanOperatorProtocol,
    GreaterThanOperatorMethod,
    GreaterOrEqualOperatorProtocol,
    GreaterOrEqualOperatorMethod,

    // Operator protocols - bitwise
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

    // Operator protocols - logical
    LogicalAndOperatorProtocol,
    LogicalAndOperatorMethod,
    LogicalOrOperatorProtocol,
    LogicalOrOperatorMethod,
    LogicalNotOperatorProtocol,
    LogicalNotOperatorMethod,

    // Boolean conditional protocol (for if/while conditions)
    BooleanConditional,

    // Operator protocols - range
    ExclusiveRangeOperatorProtocol,
    ExclusiveRangeOperatorMethod,
    InclusiveRangeOperatorProtocol,
    InclusiveRangeOperatorMethod,

    // Try operator
    ControlFlowEnum,
    TryableProtocol,
    TryExtractMethod,
    FromResidualProtocol,
    FromResidualMethod,
}

impl LanguageFeature {
    /// Parse a language feature from its name (without the leading dot).
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Copyable" => Some(Self::Copyable),
            "Cloneable" => Some(Self::Cloneable),
            "Clone" => Some(Self::Clone),
            "ExpressibleByIntLiteral" => Some(Self::ExpressibleByIntLiteral),
            "ExpressibleByFloatLiteral" => Some(Self::ExpressibleByFloatLiteral),
            "ExpressibleByStringLiteral" => Some(Self::ExpressibleByStringLiteral),
            "ExpressibleByBoolLiteral" => Some(Self::ExpressibleByBoolLiteral),
            "ExpressibleByNilLiteral" => Some(Self::ExpressibleByNilLiteral),
            "ExpressibleByArrayLiteral" => Some(Self::ExpressibleByArrayLiteral),
            "ExpressibleByDictionaryLiteral" => Some(Self::ExpressibleByDictionaryLiteral),
            "FFISafe" => Some(Self::FFISafe),
            "DefaultIntegerLiteralType" => Some(Self::DefaultIntegerLiteralType),
            "DefaultFloatLiteralType" => Some(Self::DefaultFloatLiteralType),
            // Operator protocols - arithmetic
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
            // Operator protocols - comparison
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
            // Operator protocols - bitwise
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
            // Operator protocols - logical
            "LogicalAndOperatorProtocol" => Some(Self::LogicalAndOperatorProtocol),
            "LogicalAndOperatorMethod" => Some(Self::LogicalAndOperatorMethod),
            "LogicalOrOperatorProtocol" => Some(Self::LogicalOrOperatorProtocol),
            "LogicalOrOperatorMethod" => Some(Self::LogicalOrOperatorMethod),
            "LogicalNotOperatorProtocol" => Some(Self::LogicalNotOperatorProtocol),
            "LogicalNotOperatorMethod" => Some(Self::LogicalNotOperatorMethod),
            // Boolean conditional protocol
            "BooleanConditional" => Some(Self::BooleanConditional),
            // Operator protocols - range
            "ExclusiveRangeOperatorProtocol" => Some(Self::ExclusiveRangeOperatorProtocol),
            "ExclusiveRangeOperatorMethod" => Some(Self::ExclusiveRangeOperatorMethod),
            "InclusiveRangeOperatorProtocol" => Some(Self::InclusiveRangeOperatorProtocol),
            "InclusiveRangeOperatorMethod" => Some(Self::InclusiveRangeOperatorMethod),
            // Try operator
            "ControlFlowEnum" => Some(Self::ControlFlowEnum),
            "TryableProtocol" => Some(Self::TryableProtocol),
            "TryExtractMethod" => Some(Self::TryExtractMethod),
            "FromResidualProtocol" => Some(Self::FromResidualProtocol),
            "FromResidualMethod" => Some(Self::FromResidualMethod),
            _ => None,
        }
    }

    /// Get the name of this feature (for error messages).
    pub fn name(&self) -> &'static str {
        match self {
            Self::Copyable => "Copyable",
            Self::Cloneable => "Cloneable",
            Self::Clone => "Clone",
            Self::ExpressibleByIntLiteral => "ExpressibleByIntLiteral",
            Self::ExpressibleByFloatLiteral => "ExpressibleByFloatLiteral",
            Self::ExpressibleByStringLiteral => "ExpressibleByStringLiteral",
            Self::ExpressibleByBoolLiteral => "ExpressibleByBoolLiteral",
            Self::ExpressibleByNilLiteral => "ExpressibleByNilLiteral",
            Self::ExpressibleByArrayLiteral => "ExpressibleByArrayLiteral",
            Self::ExpressibleByDictionaryLiteral => "ExpressibleByDictionaryLiteral",
            Self::FFISafe => "FFISafe",
            Self::DefaultIntegerLiteralType => "DefaultIntegerLiteralType",
            Self::DefaultFloatLiteralType => "DefaultFloatLiteralType",
            // Operator protocols - arithmetic
            Self::AddOperatorProtocol => "AddOperatorProtocol",
            Self::AddOperatorMethod => "AddOperatorMethod",
            Self::SubtractOperatorProtocol => "SubtractOperatorProtocol",
            Self::SubtractOperatorMethod => "SubtractOperatorMethod",
            Self::MultiplyOperatorProtocol => "MultiplyOperatorProtocol",
            Self::MultiplyOperatorMethod => "MultiplyOperatorMethod",
            Self::DivideOperatorProtocol => "DivideOperatorProtocol",
            Self::DivideOperatorMethod => "DivideOperatorMethod",
            Self::ModuloOperatorProtocol => "ModuloOperatorProtocol",
            Self::ModuloOperatorMethod => "ModuloOperatorMethod",
            Self::NegateOperatorProtocol => "NegateOperatorProtocol",
            Self::NegateOperatorMethod => "NegateOperatorMethod",
            // Operator protocols - comparison
            Self::EqualsOperatorProtocol => "EqualsOperatorProtocol",
            Self::EqualsOperatorMethod => "EqualsOperatorMethod",
            Self::NotEqualsOperatorProtocol => "NotEqualsOperatorProtocol",
            Self::NotEqualsOperatorMethod => "NotEqualsOperatorMethod",
            Self::LessThanOperatorProtocol => "LessThanOperatorProtocol",
            Self::LessThanOperatorMethod => "LessThanOperatorMethod",
            Self::LessOrEqualOperatorProtocol => "LessOrEqualOperatorProtocol",
            Self::LessOrEqualOperatorMethod => "LessOrEqualOperatorMethod",
            Self::GreaterThanOperatorProtocol => "GreaterThanOperatorProtocol",
            Self::GreaterThanOperatorMethod => "GreaterThanOperatorMethod",
            Self::GreaterOrEqualOperatorProtocol => "GreaterOrEqualOperatorProtocol",
            Self::GreaterOrEqualOperatorMethod => "GreaterOrEqualOperatorMethod",
            // Operator protocols - bitwise
            Self::BitwiseAndOperatorProtocol => "BitwiseAndOperatorProtocol",
            Self::BitwiseAndOperatorMethod => "BitwiseAndOperatorMethod",
            Self::BitwiseOrOperatorProtocol => "BitwiseOrOperatorProtocol",
            Self::BitwiseOrOperatorMethod => "BitwiseOrOperatorMethod",
            Self::BitwiseXorOperatorProtocol => "BitwiseXorOperatorProtocol",
            Self::BitwiseXorOperatorMethod => "BitwiseXorOperatorMethod",
            Self::ShiftLeftOperatorProtocol => "ShiftLeftOperatorProtocol",
            Self::ShiftLeftOperatorMethod => "ShiftLeftOperatorMethod",
            Self::ShiftRightOperatorProtocol => "ShiftRightOperatorProtocol",
            Self::ShiftRightOperatorMethod => "ShiftRightOperatorMethod",
            Self::BitwiseNotOperatorProtocol => "BitwiseNotOperatorProtocol",
            Self::BitwiseNotOperatorMethod => "BitwiseNotOperatorMethod",
            // Operator protocols - logical
            Self::LogicalAndOperatorProtocol => "LogicalAndOperatorProtocol",
            Self::LogicalAndOperatorMethod => "LogicalAndOperatorMethod",
            Self::LogicalOrOperatorProtocol => "LogicalOrOperatorProtocol",
            Self::LogicalOrOperatorMethod => "LogicalOrOperatorMethod",
            Self::LogicalNotOperatorProtocol => "LogicalNotOperatorProtocol",
            Self::LogicalNotOperatorMethod => "LogicalNotOperatorMethod",
            // Boolean conditional protocol
            Self::BooleanConditional => "BooleanConditional",
            // Operator protocols - range
            Self::ExclusiveRangeOperatorProtocol => "ExclusiveRangeOperatorProtocol",
            Self::ExclusiveRangeOperatorMethod => "ExclusiveRangeOperatorMethod",
            Self::InclusiveRangeOperatorProtocol => "InclusiveRangeOperatorProtocol",
            Self::InclusiveRangeOperatorMethod => "InclusiveRangeOperatorMethod",
            // Try operator
            Self::ControlFlowEnum => "ControlFlowEnum",
            Self::TryableProtocol => "TryableProtocol",
            Self::TryExtractMethod => "TryExtractMethod",
            Self::FromResidualProtocol => "FromResidualProtocol",
            Self::FromResidualMethod => "FromResidualMethod",
        }
    }

    /// Get the expected definition for this feature.
    pub fn definition(&self) -> BuiltinDefinition {
        match self {
            Self::Copyable => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::Protocol {
                    implicit_conformance: true,
                    must_be_marker: true,
                    tuple_conformance_propagation: false,
                    requires_fields_conform: false,
                    disallow_enum_conformance: false,
                },
            },
            Self::Cloneable => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::Protocol {
                    implicit_conformance: false,
                    must_be_marker: false,
                    tuple_conformance_propagation: false,
                    requires_fields_conform: false,
                    disallow_enum_conformance: false,
                },
            },
            Self::Clone => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::Cloneable,
                },
            },
            Self::ExpressibleByIntLiteral => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::Protocol {
                    implicit_conformance: false,
                    must_be_marker: false,
                    tuple_conformance_propagation: false,
                    requires_fields_conform: false,
                    disallow_enum_conformance: false,
                },
            },
            Self::ExpressibleByFloatLiteral => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::Protocol {
                    implicit_conformance: false,
                    must_be_marker: false,
                    tuple_conformance_propagation: false,
                    requires_fields_conform: false,
                    disallow_enum_conformance: false,
                },
            },
            Self::ExpressibleByStringLiteral => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::Protocol {
                    implicit_conformance: false,
                    must_be_marker: false,
                    tuple_conformance_propagation: false,
                    requires_fields_conform: false,
                    disallow_enum_conformance: false,
                },
            },
            Self::ExpressibleByBoolLiteral => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::Protocol {
                    implicit_conformance: false,
                    must_be_marker: false,
                    tuple_conformance_propagation: false,
                    requires_fields_conform: false,
                    disallow_enum_conformance: false,
                },
            },
            Self::ExpressibleByNilLiteral => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::Protocol {
                    implicit_conformance: false,
                    must_be_marker: false,
                    tuple_conformance_propagation: false,
                    requires_fields_conform: false,
                    disallow_enum_conformance: false,
                },
            },
            Self::ExpressibleByArrayLiteral => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::Protocol {
                    implicit_conformance: false,
                    must_be_marker: false,
                    tuple_conformance_propagation: false,
                    requires_fields_conform: false,
                    disallow_enum_conformance: false,
                },
            },
            Self::ExpressibleByDictionaryLiteral => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::Protocol {
                    implicit_conformance: false,
                    must_be_marker: false,
                    tuple_conformance_propagation: false,
                    requires_fields_conform: false,
                    disallow_enum_conformance: false,
                },
            },
            Self::FFISafe => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::Protocol {
                    implicit_conformance: false,
                    must_be_marker: true,
                    tuple_conformance_propagation: true,
                    requires_fields_conform: true,
                    disallow_enum_conformance: true,
                },
            },
            Self::DefaultIntegerLiteralType => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::TypeAlias,
            },
            Self::DefaultFloatLiteralType => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::TypeAlias,
            },
            // Operator protocols - arithmetic
            Self::AddOperatorProtocol
            | Self::SubtractOperatorProtocol
            | Self::MultiplyOperatorProtocol
            | Self::DivideOperatorProtocol
            | Self::ModuloOperatorProtocol
            | Self::NegateOperatorProtocol
            // Operator protocols - comparison
            | Self::EqualsOperatorProtocol
            | Self::NotEqualsOperatorProtocol
            | Self::LessThanOperatorProtocol
            | Self::LessOrEqualOperatorProtocol
            | Self::GreaterThanOperatorProtocol
            | Self::GreaterOrEqualOperatorProtocol
            // Operator protocols - bitwise
            | Self::BitwiseAndOperatorProtocol
            | Self::BitwiseOrOperatorProtocol
            | Self::BitwiseXorOperatorProtocol
            | Self::ShiftLeftOperatorProtocol
            | Self::ShiftRightOperatorProtocol
            | Self::BitwiseNotOperatorProtocol
            // Operator protocols - logical
            | Self::LogicalAndOperatorProtocol
            | Self::LogicalOrOperatorProtocol
            | Self::LogicalNotOperatorProtocol
            // Boolean conditional protocol
            | Self::BooleanConditional
            // Operator protocols - range
            | Self::ExclusiveRangeOperatorProtocol
            | Self::InclusiveRangeOperatorProtocol => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::Protocol {
                    implicit_conformance: false,
                    must_be_marker: false,
                    tuple_conformance_propagation: false,
                    requires_fields_conform: false,
                    disallow_enum_conformance: false,
                },
            },
            // Operator methods - arithmetic
            Self::AddOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::AddOperatorProtocol,
                },
            },
            Self::SubtractOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::SubtractOperatorProtocol,
                },
            },
            Self::MultiplyOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::MultiplyOperatorProtocol,
                },
            },
            Self::DivideOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::DivideOperatorProtocol,
                },
            },
            Self::ModuloOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::ModuloOperatorProtocol,
                },
            },
            Self::NegateOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::NegateOperatorProtocol,
                },
            },
            // Operator methods - comparison
            Self::EqualsOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::EqualsOperatorProtocol,
                },
            },
            Self::NotEqualsOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::NotEqualsOperatorProtocol,
                },
            },
            Self::LessThanOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::LessThanOperatorProtocol,
                },
            },
            Self::LessOrEqualOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::LessOrEqualOperatorProtocol,
                },
            },
            Self::GreaterThanOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::GreaterThanOperatorProtocol,
                },
            },
            Self::GreaterOrEqualOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::GreaterOrEqualOperatorProtocol,
                },
            },
            // Operator methods - bitwise
            Self::BitwiseAndOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::BitwiseAndOperatorProtocol,
                },
            },
            Self::BitwiseOrOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::BitwiseOrOperatorProtocol,
                },
            },
            Self::BitwiseXorOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::BitwiseXorOperatorProtocol,
                },
            },
            Self::ShiftLeftOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::ShiftLeftOperatorProtocol,
                },
            },
            Self::ShiftRightOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::ShiftRightOperatorProtocol,
                },
            },
            Self::BitwiseNotOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::BitwiseNotOperatorProtocol,
                },
            },
            // Operator methods - logical
            Self::LogicalAndOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::LogicalAndOperatorProtocol,
                },
            },
            Self::LogicalOrOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::LogicalOrOperatorProtocol,
                },
            },
            Self::LogicalNotOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::LogicalNotOperatorProtocol,
                },
            },
            // Operator methods - range
            Self::ExclusiveRangeOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::ExclusiveRangeOperatorProtocol,
                },
            },
            Self::InclusiveRangeOperatorMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::InclusiveRangeOperatorProtocol,
                },
            },
            // Try operator
            Self::ControlFlowEnum => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::Enum,
            },
            Self::TryableProtocol | Self::FromResidualProtocol => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::Protocol {
                    implicit_conformance: false,
                    must_be_marker: false,
                    tuple_conformance_propagation: false,
                    requires_fields_conform: false,
                    disallow_enum_conformance: false,
                },
            },
            Self::TryExtractMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::TryableProtocol,
                },
            },
            Self::FromResidualMethod => BuiltinDefinition {
                feature: *self,
                kind: BuiltinKind::ProtocolMethod {
                    protocol_feature: LanguageFeature::FromResidualProtocol,
                },
            },
        }
    }
}

/// What kind of symbol a builtin expects, with kind-specific configuration.
#[derive(Debug, Clone)]
pub enum BuiltinKind {
    /// A builtin protocol.
    Protocol {
        /// If true, types implicitly conform unless opted out with `not Protocol`.
        implicit_conformance: bool,
        /// If true, must be a marker protocol (no required methods/types).
        must_be_marker: bool,
        /// If true, tuples conform to this protocol if all elements conform.
        tuple_conformance_propagation: bool,
        /// If true, structs/enums conforming to this protocol must have all fields conform.
        requires_fields_conform: bool,
        /// If true, enums cannot conform to this protocol.
        disallow_enum_conformance: bool,
    },
    /// A builtin protocol method.
    ProtocolMethod {
        /// The protocol feature this method belongs to.
        protocol_feature: LanguageFeature,
    },
    /// A builtin struct (e.g., Int, Bool, String).
    Struct,
    /// A builtin enum (e.g., Ordering, Optional).
    Enum,
    /// A builtin function (e.g., sizeof, alignof).
    Function,
    /// A builtin variable/constant.
    Variable,
    /// A builtin type alias (e.g., DefaultIntegerLiteralType).
    TypeAlias,
}

impl BuiltinKind {
    /// Check if this kind is for a protocol.
    pub fn is_protocol(&self) -> bool {
        matches!(self, Self::Protocol { .. })
    }

    /// Check if this kind is for a protocol method.
    pub fn is_protocol_method(&self) -> bool {
        matches!(self, Self::ProtocolMethod { .. })
    }

    /// Check if this kind is for a struct.
    pub fn is_struct(&self) -> bool {
        matches!(self, Self::Struct)
    }

    /// Check if this kind is for an enum.
    pub fn is_enum(&self) -> bool {
        matches!(self, Self::Enum)
    }

    /// Check if this kind is for a function.
    pub fn is_function(&self) -> bool {
        matches!(self, Self::Function)
    }

    /// Check if this kind is for a variable.
    pub fn is_variable(&self) -> bool {
        matches!(self, Self::Variable)
    }

    /// Check if this kind is for a type alias.
    pub fn is_type_alias(&self) -> bool {
        matches!(self, Self::TypeAlias)
    }

    /// Get the expected symbol kind name for error messages.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Protocol { .. } => "protocol",
            Self::ProtocolMethod { .. } => "protocol method",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Function => "function",
            Self::Variable => "variable",
            Self::TypeAlias => "type alias",
        }
    }
}

/// Definition of a language feature builtin.
#[derive(Debug, Clone)]
pub struct BuiltinDefinition {
    /// The language feature.
    pub feature: LanguageFeature,
    /// The expected symbol kind and its configuration.
    pub kind: BuiltinKind,
}

/// Registry for builtin language features.
///
/// Maintains separate maps for each symbol kind to allow efficient
/// lookup and different behavior per kind.
#[derive(Debug, Default)]
pub struct BuiltinRegistry {
    // Protocol builtins
    protocols: RwLock<HashMap<LanguageFeature, SymbolId>>,
    protocol_features: RwLock<HashMap<SymbolId, LanguageFeature>>,

    // Struct builtins
    structs: RwLock<HashMap<LanguageFeature, SymbolId>>,
    struct_features: RwLock<HashMap<SymbolId, LanguageFeature>>,

    // Enum builtins
    enums: RwLock<HashMap<LanguageFeature, SymbolId>>,
    enum_features: RwLock<HashMap<SymbolId, LanguageFeature>>,

    // Function builtins
    functions: RwLock<HashMap<LanguageFeature, SymbolId>>,
    function_features: RwLock<HashMap<SymbolId, LanguageFeature>>,

    // Variable builtins
    variables: RwLock<HashMap<LanguageFeature, SymbolId>>,
    variable_features: RwLock<HashMap<SymbolId, LanguageFeature>>,

    // Method builtins
    methods: RwLock<HashMap<LanguageFeature, SymbolId>>,
    method_features: RwLock<HashMap<SymbolId, LanguageFeature>>,

    // Type alias builtins
    type_aliases: RwLock<HashMap<LanguageFeature, SymbolId>>,
    type_alias_features: RwLock<HashMap<SymbolId, LanguageFeature>>,
}

impl BuiltinRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    // =========================================================================
    // Protocol methods
    // =========================================================================

    /// Register a protocol as a builtin. Returns true if successful,
    /// false if the feature was already registered.
    pub fn register_protocol(&self, feature: LanguageFeature, id: SymbolId) -> bool {
        let mut protocols = self.protocols.write();
        if protocols.contains_key(&feature) {
            return false;
        }
        protocols.insert(feature, id);
        self.protocol_features.write().insert(id, feature);
        true
    }

    /// Get the symbol ID for a builtin protocol.
    pub fn protocol(&self, feature: LanguageFeature) -> Option<SymbolId> {
        self.protocols.read().get(&feature).copied()
    }

    /// Check if a symbol is a builtin protocol.
    pub fn is_builtin_protocol(&self, id: SymbolId) -> bool {
        self.protocol_features.read().contains_key(&id)
    }

    /// Get the feature for a builtin protocol.
    pub fn protocol_feature(&self, id: SymbolId) -> Option<LanguageFeature> {
        self.protocol_features.read().get(&id).copied()
    }

    /// Convenience: Get the Copyable protocol.
    pub fn copyable_protocol(&self) -> Option<SymbolId> {
        self.protocol(LanguageFeature::Copyable)
    }

    /// Convenience: Get the Cloneable protocol.
    pub fn cloneable_protocol(&self) -> Option<SymbolId> {
        self.protocol(LanguageFeature::Cloneable)
    }

    // =========================================================================
    // Struct methods
    // =========================================================================

    /// Register a struct as a builtin. Returns true if successful,
    /// false if the feature was already registered.
    pub fn register_struct(&self, feature: LanguageFeature, id: SymbolId) -> bool {
        let mut structs = self.structs.write();
        if structs.contains_key(&feature) {
            return false;
        }
        structs.insert(feature, id);
        self.struct_features.write().insert(id, feature);
        true
    }

    /// Get the symbol ID for a builtin struct.
    pub fn builtin_struct(&self, feature: LanguageFeature) -> Option<SymbolId> {
        self.structs.read().get(&feature).copied()
    }

    /// Check if a symbol is a builtin struct.
    pub fn is_builtin_struct(&self, id: SymbolId) -> bool {
        self.struct_features.read().contains_key(&id)
    }

    /// Get the feature for a builtin struct.
    pub fn struct_feature(&self, id: SymbolId) -> Option<LanguageFeature> {
        self.struct_features.read().get(&id).copied()
    }

    // =========================================================================
    // Enum methods
    // =========================================================================

    /// Register an enum as a builtin. Returns true if successful,
    /// false if the feature was already registered.
    pub fn register_enum(&self, feature: LanguageFeature, id: SymbolId) -> bool {
        let mut enums = self.enums.write();
        if enums.contains_key(&feature) {
            return false;
        }
        enums.insert(feature, id);
        self.enum_features.write().insert(id, feature);
        true
    }

    /// Get the symbol ID for a builtin enum.
    pub fn builtin_enum(&self, feature: LanguageFeature) -> Option<SymbolId> {
        self.enums.read().get(&feature).copied()
    }

    /// Check if a symbol is a builtin enum.
    pub fn is_builtin_enum(&self, id: SymbolId) -> bool {
        self.enum_features.read().contains_key(&id)
    }

    /// Get the feature for a builtin enum.
    pub fn enum_feature(&self, id: SymbolId) -> Option<LanguageFeature> {
        self.enum_features.read().get(&id).copied()
    }

    // =========================================================================
    // Function methods
    // =========================================================================

    /// Register a function as a builtin. Returns true if successful,
    /// false if the feature was already registered.
    pub fn register_function(&self, feature: LanguageFeature, id: SymbolId) -> bool {
        let mut functions = self.functions.write();
        if functions.contains_key(&feature) {
            return false;
        }
        functions.insert(feature, id);
        self.function_features.write().insert(id, feature);
        true
    }

    /// Get the symbol ID for a builtin function.
    pub fn builtin_function(&self, feature: LanguageFeature) -> Option<SymbolId> {
        self.functions.read().get(&feature).copied()
    }

    /// Check if a symbol is a builtin function.
    pub fn is_builtin_function(&self, id: SymbolId) -> bool {
        self.function_features.read().contains_key(&id)
    }

    /// Get the feature for a builtin function.
    pub fn function_feature(&self, id: SymbolId) -> Option<LanguageFeature> {
        self.function_features.read().get(&id).copied()
    }

    // =========================================================================
    // Variable methods
    // =========================================================================

    /// Register a variable as a builtin. Returns true if successful,
    /// false if the feature was already registered.
    pub fn register_variable(&self, feature: LanguageFeature, id: SymbolId) -> bool {
        let mut variables = self.variables.write();
        if variables.contains_key(&feature) {
            return false;
        }
        variables.insert(feature, id);
        self.variable_features.write().insert(id, feature);
        true
    }

    /// Get the symbol ID for a builtin variable.
    pub fn builtin_variable(&self, feature: LanguageFeature) -> Option<SymbolId> {
        self.variables.read().get(&feature).copied()
    }

    /// Check if a symbol is a builtin variable.
    pub fn is_builtin_variable(&self, id: SymbolId) -> bool {
        self.variable_features.read().contains_key(&id)
    }

    /// Get the feature for a builtin variable.
    pub fn variable_feature(&self, id: SymbolId) -> Option<LanguageFeature> {
        self.variable_features.read().get(&id).copied()
    }

    // =========================================================================
    // Method methods
    // =========================================================================

    /// Register a method as a builtin. Returns true if successful,
    /// false if the feature was already registered.
    pub fn register_method(&self, feature: LanguageFeature, id: SymbolId) -> bool {
        let mut methods = self.methods.write();
        if methods.contains_key(&feature) {
            return false;
        }
        methods.insert(feature, id);
        self.method_features.write().insert(id, feature);
        true
    }

    /// Get the symbol ID for a builtin method.
    pub fn method(&self, feature: LanguageFeature) -> Option<SymbolId> {
        self.methods.read().get(&feature).copied()
    }

    /// Check if a symbol is a builtin method.
    pub fn is_builtin_method(&self, id: SymbolId) -> bool {
        self.method_features.read().contains_key(&id)
    }

    /// Get the feature for a builtin method.
    pub fn method_feature(&self, id: SymbolId) -> Option<LanguageFeature> {
        self.method_features.read().get(&id).copied()
    }

    /// Convenience: Get the Clone method.
    pub fn clone_method(&self) -> Option<SymbolId> {
        self.method(LanguageFeature::Clone)
    }

    // =========================================================================
    // Type alias methods
    // =========================================================================

    /// Register a type alias as a builtin. Returns true if successful,
    /// false if the feature was already registered.
    pub fn register_type_alias(&self, feature: LanguageFeature, id: SymbolId) -> bool {
        let mut type_aliases = self.type_aliases.write();
        if type_aliases.contains_key(&feature) {
            return false;
        }
        type_aliases.insert(feature, id);
        self.type_alias_features.write().insert(id, feature);
        true
    }

    /// Get the symbol ID for a builtin type alias.
    pub fn type_alias(&self, feature: LanguageFeature) -> Option<SymbolId> {
        self.type_aliases.read().get(&feature).copied()
    }

    /// Check if a symbol is a builtin type alias.
    pub fn is_builtin_type_alias(&self, id: SymbolId) -> bool {
        self.type_alias_features.read().contains_key(&id)
    }

    /// Get the feature for a builtin type alias.
    pub fn type_alias_feature(&self, id: SymbolId) -> Option<LanguageFeature> {
        self.type_alias_features.read().get(&id).copied()
    }

    // =========================================================================
    // Generic methods
    // =========================================================================

    /// Check if a feature is already registered (for any kind).
    pub fn is_feature_registered(&self, feature: LanguageFeature) -> bool {
        self.protocols.read().contains_key(&feature)
            || self.structs.read().contains_key(&feature)
            || self.enums.read().contains_key(&feature)
            || self.functions.read().contains_key(&feature)
            || self.variables.read().contains_key(&feature)
            || self.methods.read().contains_key(&feature)
            || self.type_aliases.read().contains_key(&feature)
    }
}
