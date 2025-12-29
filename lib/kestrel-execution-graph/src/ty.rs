//! MIR type representation.

use crate::id::{Id, QualifiedName, Ty, TypeParam};
use std::fmt;

/// MIR type representation.
///
/// Types are interned in `MirContext`. Use `Id<Ty>` to reference them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MirTy {
    // === Primitives ===
    I8,
    I16,
    I32,
    I64,
    F16,
    F32,
    F64,
    Bool,
    Unit,
    Never,
    Str,

    // === Pointers and References ===
    /// `p[T]` - raw pointer to T
    Pointer(Id<Ty>),
    /// `&T` - immutable reference
    Ref(Id<Ty>),
    /// `&var T` - mutable reference
    RefMut(Id<Ty>),

    // === Compound ===
    /// `(T1, T2, ...)` - tuple type
    Tuple(Vec<Id<Ty>>),

    /// `[T]` - array type
    Array(Id<Ty>),

    /// Named type (struct, enum, protocol).
    Named {
        name: Id<QualifiedName>,
        type_args: Vec<Id<Ty>>,
    },

    /// Type parameter reference.
    TypeParam(Id<TypeParam>),

    // === Function types ===
    /// Thin function pointer (no environment, FFI-safe).
    /// `func(Args...) -> Ret`
    FuncThin { params: Vec<Id<Ty>>, ret: Id<Ty> },

    /// Thick callable (has environment, can escape).
    /// `func escaping(Args...) -> Ret`
    FuncThick { params: Vec<Id<Ty>>, ret: Id<Ty> },

    /// `Self` - the implementing type in a protocol context.
    /// Only valid in protocol method signatures. During witness lookup,
    /// this gets substituted with the concrete implementing type.
    SelfType,

    /// Associated type projection: `T.Element` where `T: Container`.
    ///
    /// During monomorphization, this is resolved to the concrete type
    /// by looking up the witness table for the implementing type.
    /// For example, if `T` is substituted with `MyStruct` and `MyStruct: Container`
    /// with `type Element = Int`, then `T.Element` resolves to `Int`.
    AssociatedTypeProjection {
        /// The base type (e.g., type parameter T, or Self)
        base: Id<Ty>,
        /// The protocol that defines the associated type
        protocol: Id<QualifiedName>,
        /// The associated type name (e.g., "Element")
        associated: String,
    },

    /// Error/poison type used when lowering fails.
    /// This represents a type that couldn't be lowered due to an error.
    /// Using a dedicated error type instead of Unit makes error cases explicit.
    Error,
}

impl MirTy {
    /// Create a display wrapper that can format this type with context.
    pub fn display<'a>(&'a self, ctx: &'a crate::MirContext) -> impl fmt::Display + 'a {
        MirTyDisplay { ty: self, ctx }
    }

    /// Check if this is a primitive integer type.
    pub fn is_integer(&self) -> bool {
        matches!(self, MirTy::I8 | MirTy::I16 | MirTy::I32 | MirTy::I64)
    }

    /// Check if this is a primitive float type.
    pub fn is_float(&self) -> bool {
        matches!(self, MirTy::F16 | MirTy::F32 | MirTy::F64)
    }

    /// Check if this is a reference type (either mutable or immutable).
    pub fn is_reference(&self) -> bool {
        matches!(self, MirTy::Ref(_) | MirTy::RefMut(_))
    }

    /// Check if this is a pointer type.
    pub fn is_pointer(&self) -> bool {
        matches!(self, MirTy::Pointer(_))
    }
}

struct MirTyDisplay<'a> {
    ty: &'a MirTy,
    ctx: &'a crate::MirContext,
}

impl fmt::Display for MirTyDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ty {
            MirTy::I8 => write!(f, "i8"),
            MirTy::I16 => write!(f, "i16"),
            MirTy::I32 => write!(f, "i32"),
            MirTy::I64 => write!(f, "i64"),
            MirTy::F16 => write!(f, "f16"),
            MirTy::F32 => write!(f, "f32"),
            MirTy::F64 => write!(f, "f64"),
            MirTy::Bool => write!(f, "bool"),
            MirTy::Unit => write!(f, "()"),
            MirTy::Never => write!(f, "!"),
            MirTy::Str => write!(f, "str"),

            MirTy::Pointer(inner) => {
                write!(f, "p[{}]", self.ctx.ty(*inner).display(self.ctx))
            }
            MirTy::Ref(inner) => {
                write!(f, "&{}", self.ctx.ty(*inner).display(self.ctx))
            }
            MirTy::RefMut(inner) => {
                write!(f, "&var {}", self.ctx.ty(*inner).display(self.ctx))
            }

            MirTy::Tuple(elems) => {
                write!(f, "(")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", self.ctx.ty(*elem).display(self.ctx))?;
                }
                write!(f, ")")
            }

            MirTy::Array(elem) => {
                write!(f, "[{}]", self.ctx.ty(*elem).display(self.ctx))
            }

            MirTy::Named { name, type_args } => {
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

            MirTy::TypeParam(id) => {
                write!(f, "{}", self.ctx.type_param(*id).name)
            }

            MirTy::FuncThin { params, ret } => {
                write!(f, "func(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", self.ctx.ty(*p).display(self.ctx))?;
                }
                write!(f, ") -> {}", self.ctx.ty(*ret).display(self.ctx))
            }

            MirTy::FuncThick { params, ret } => {
                write!(f, "func escaping(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", self.ctx.ty(*p).display(self.ctx))?;
                }
                write!(f, ") -> {}", self.ctx.ty(*ret).display(self.ctx))
            }

            MirTy::SelfType => write!(f, "Self"),

            MirTy::AssociatedTypeProjection {
                base,
                protocol,
                associated,
            } => {
                write!(
                    f,
                    "({}.{} for {})",
                    self.ctx.name(*protocol),
                    associated,
                    self.ctx.ty(*base).display(self.ctx),
                )
            }

            MirTy::Error => write!(f, "<error>"),
        }
    }
}
