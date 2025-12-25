//! Immediate values (constants).

use crate::id::{Id, QualifiedName, Ty};
use crate::metadata::Metadata;
use crate::MirContext;
use std::fmt;

/// An immediate (constant) value.
#[derive(Debug, Clone)]
pub struct Immediate {
    pub meta: Metadata,
    /// Optional inline name for this immediate.
    pub inline_name: Option<String>,
    /// The kind of immediate.
    pub kind: ImmediateKind,
}

/// The different kinds of immediate values.
#[derive(Debug, Clone)]
pub enum ImmediateKind {
    // === Literals ===
    /// Integer literal with explicit bit width.
    IntLiteral { bits: IntBits, value: i128 },
    /// Float literal with explicit bit width.
    FloatLiteral { bits: FloatBits, value: f64 },
    /// Boolean literal.
    BoolLiteral(bool),
    /// String literal.
    StringLiteral(String),
    /// Unit value.
    Unit,

    // === Function references ===
    /// Reference to a function.
    FunctionRef {
        name: Id<QualifiedName>,
        type_args: Vec<Id<Ty>>,
    },

    // === Witness method lookup ===
    /// `witness_method Protocol.method for Type`
    WitnessMethod {
        protocol: Id<QualifiedName>,
        method: String,
        for_type: Id<Ty>,
    },

    // === Null pointer ===
    /// `ptr.null[T]`
    NullPtr(Id<Ty>),
}

/// Integer bit widths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntBits {
    I8,
    I16,
    I32,
    I64,
}

/// Float bit widths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatBits {
    F16,
    F32,
    F64,
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
            meta: Metadata::new(),
            inline_name: None,
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
            meta: Metadata::new(),
            inline_name: None,
            kind: ImmediateKind::FloatLiteral { bits, value },
        }
    }

    /// Create a boolean literal.
    pub fn bool(value: bool) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: ImmediateKind::BoolLiteral(value),
        }
    }

    /// Create a string literal.
    pub fn string(value: impl Into<String>) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: ImmediateKind::StringLiteral(value.into()),
        }
    }

    /// Create a unit value.
    pub fn unit() -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: ImmediateKind::Unit,
        }
    }

    /// Create a function reference.
    pub fn function_ref(name: Id<QualifiedName>) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: ImmediateKind::FunctionRef {
                name,
                type_args: Vec::new(),
            },
        }
    }

    /// Create a function reference with type arguments.
    pub fn function_ref_generic(name: Id<QualifiedName>, type_args: Vec<Id<Ty>>) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: ImmediateKind::FunctionRef { name, type_args },
        }
    }

    /// Create a witness method reference.
    pub fn witness_method(
        protocol: Id<QualifiedName>,
        method: impl Into<String>,
        for_type: Id<Ty>,
    ) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: ImmediateKind::WitnessMethod {
                protocol,
                method: method.into(),
                for_type,
            },
        }
    }

    /// Create a null pointer.
    pub fn null_ptr(ty: Id<Ty>) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: ImmediateKind::NullPtr(ty),
        }
    }

    /// Set an inline name for this immediate.
    pub fn with_inline_name(mut self, name: impl Into<String>) -> Self {
        self.inline_name = Some(name.into());
        self
    }

    /// Create a display wrapper for printing this immediate.
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        ImmediateDisplay { imm: self, ctx }
    }
}

struct ImmediateDisplay<'a> {
    imm: &'a Immediate,
    ctx: &'a MirContext,
}

impl fmt::Display for ImmediateDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.imm.kind {
            ImmediateKind::IntLiteral { bits, value } => {
                let prefix = match bits {
                    IntBits::I8 => "i8",
                    IntBits::I16 => "i16",
                    IntBits::I32 => "i32",
                    IntBits::I64 => "i64",
                };
                write!(f, "{}.literal {}", prefix, value)
            }
            ImmediateKind::FloatLiteral { bits, value } => {
                let prefix = match bits {
                    FloatBits::F16 => "f16",
                    FloatBits::F32 => "f32",
                    FloatBits::F64 => "f64",
                };
                write!(f, "{}.literal {}", prefix, value)
            }
            ImmediateKind::BoolLiteral(b) => write!(f, "{}", b),
            ImmediateKind::StringLiteral(s) => write!(f, "str.literal {:?}", s),
            ImmediateKind::Unit => write!(f, "()"),
            ImmediateKind::FunctionRef { name, type_args } => {
                write!(f, "{}", self.ctx.name(*name))?;
                if !type_args.is_empty() {
                    write!(f, "[")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", self.ctx.ty(*arg).display(self.ctx))?;
                    }
                    write!(f, "]")?;
                }
                Ok(())
            }
            ImmediateKind::WitnessMethod {
                protocol,
                method,
                for_type,
            } => {
                write!(
                    f,
                    "witness_method {}.{} for {}",
                    self.ctx.name(*protocol),
                    method,
                    self.ctx.ty(*for_type).display(self.ctx)
                )
            }
            ImmediateKind::NullPtr(ty) => {
                write!(f, "ptr.null[{}]", self.ctx.ty(*ty).display(self.ctx))
            }
        }
    }
}
