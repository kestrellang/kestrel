//! Immediate values — constants and references.

use crate::op::{FloatBits, IntBits};
use crate::ty::MirTy;
use kestrel_hecs::Entity;

/// An immediate (constant) value.
#[derive(Debug, Clone)]
pub struct Immediate {
    pub kind: ImmediateKind,
}

/// The different kinds of immediate values.
#[derive(Debug, Clone)]
pub enum ImmediateKind {
    /// Integer literal with explicit bit width.
    IntLiteral { bits: IntBits, value: i128 },
    /// Float literal with explicit bit width.
    FloatLiteral { bits: FloatBits, value: f64 },
    /// Boolean literal.
    BoolLiteral(bool),
    /// String literal (fat pointer: ptr + len).
    StringLiteral(String),
    /// String pointer (just the pointer to string data, without length).
    StringPointer(String),
    /// Unit value `()`.
    Unit,
    /// Reference to a function.
    FunctionRef {
        func: Entity,
        type_args: Vec<MirTy>,
    },
    /// Witness method reference for use as a function value.
    WitnessMethod {
        protocol: Entity,
        method: String,
        for_type: MirTy,
    },
    /// Null pointer of a given type.
    NullPtr(MirTy),
    /// Error/poison value — used when lowering fails.
    Error,
}

impl Immediate {
    /// Create an i8 literal.
    pub fn i8(value: i8) -> Self {
        Self::int(IntBits::I8, value as i128)
    }

    /// Create an i16 literal.
    pub fn i16(value: i16) -> Self {
        Self::int(IntBits::I16, value as i128)
    }

    /// Create an i32 literal.
    pub fn i32(value: i32) -> Self {
        Self::int(IntBits::I32, value as i128)
    }

    /// Create an i64 literal.
    pub fn i64(value: i64) -> Self {
        Self::int(IntBits::I64, value as i128)
    }

    /// Create an integer literal with explicit bit width.
    pub fn int(bits: IntBits, value: i128) -> Self {
        Self {
            kind: ImmediateKind::IntLiteral { bits, value },
        }
    }

    /// Create an f32 literal.
    pub fn f32(value: f32) -> Self {
        Self::float(FloatBits::F32, value as f64)
    }

    /// Create an f64 literal.
    pub fn f64(value: f64) -> Self {
        Self::float(FloatBits::F64, value)
    }

    /// Create a float literal with explicit bit width.
    pub fn float(bits: FloatBits, value: f64) -> Self {
        Self {
            kind: ImmediateKind::FloatLiteral { bits, value },
        }
    }

    /// Create a boolean literal.
    pub fn bool(value: bool) -> Self {
        Self {
            kind: ImmediateKind::BoolLiteral(value),
        }
    }

    /// Create a string literal.
    pub fn string(value: impl Into<String>) -> Self {
        Self {
            kind: ImmediateKind::StringLiteral(value.into()),
        }
    }

    /// Create a string pointer.
    pub fn string_ptr(value: impl Into<String>) -> Self {
        Self {
            kind: ImmediateKind::StringPointer(value.into()),
        }
    }

    /// Create a unit value.
    pub fn unit() -> Self {
        Self {
            kind: ImmediateKind::Unit,
        }
    }

    /// Create a function reference.
    pub fn function_ref(func: Entity) -> Self {
        Self {
            kind: ImmediateKind::FunctionRef {
                func,
                type_args: Vec::new(),
            },
        }
    }

    /// Create a function reference with type arguments.
    pub fn function_ref_generic(func: Entity, type_args: Vec<MirTy>) -> Self {
        Self {
            kind: ImmediateKind::FunctionRef { func, type_args },
        }
    }

    /// Create a witness method reference.
    pub fn witness_method(
        protocol: Entity,
        method: impl Into<String>,
        for_type: MirTy,
    ) -> Self {
        Self {
            kind: ImmediateKind::WitnessMethod {
                protocol,
                method: method.into(),
                for_type,
            },
        }
    }

    /// Create a null pointer.
    pub fn null_ptr(ty: MirTy) -> Self {
        Self {
            kind: ImmediateKind::NullPtr(ty),
        }
    }

    /// Create an error/poison value.
    pub fn error() -> Self {
        Self {
            kind: ImmediateKind::Error,
        }
    }
}
