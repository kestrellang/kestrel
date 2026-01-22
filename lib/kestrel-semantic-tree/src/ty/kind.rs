use crate::symbol::associated_type::AssociatedTypeSymbol;
use crate::symbol::enum_symbol::EnumSymbol;
use crate::symbol::protocol::ProtocolSymbol;
use crate::symbol::r#struct::StructSymbol;
use crate::symbol::type_alias::TypeAliasSymbol;
use crate::symbol::type_parameter::TypeParameterSymbol;
use crate::ty::Ty;
use crate::ty::substitutions::Substitutions;
use std::sync::Arc;

/// Integer bit widths
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntBits {
    I8,
    I16,
    I32,
    I64,
}

/// Float bit widths
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatBits {
    F16,
    F32,
    F64,
}

/// Parameter information for an unresolved closure type.
///
/// When a closure is created without explicit parameter annotations, we may not
/// know the full function type yet. This enum tracks what we do know about the
/// parameters based on how the closure body is written.
#[derive(Debug, Clone)]
pub enum ParamInfo {
    /// No constraints on arity - closure body doesn't reference any parameters.
    /// The closure could have any number of parameters (0, 1, 2, ...).
    /// Example: `{ 42 }`, `{ "hello" }`
    Unconstrained,

    /// Uses implicit `it` parameter, so must have exactly 1 parameter.
    /// The `it_type` is the type variable for that parameter.
    /// Example: `{ it + 1 }`, `{ it.foo() }`
    ImplicitIt { it_type: Box<Ty> },

    /// Has explicit parameter list (possibly with inferred types).
    /// We know the exact arity but types may still be inference variables.
    /// Example: `{ (x) in x }`, `{ (x: Int, y) in x + y }`
    Explicit { param_types: Vec<Ty> },
}

/// Represents the kind of a semantic type
/// These are resolved types after semantic analysis
#[derive(Clone)]
pub enum TyKind {
    /// Unit type: ()
    Unit,

    /// Never type: !
    Never,

    /// Integer type with bit width
    Int(IntBits),

    /// Float type with bit width
    Float(FloatBits),

    /// Boolean type
    Bool,

    /// String type
    String,

    /// Tuple type: (T1, T2, ...)
    Tuple(Vec<Ty>),

    /// Array type: [T]
    Array(Box<Ty>),

    /// Raw pointer type: lang.ptr[T]
    Pointer(Box<Ty>),

    /// Function type: (P1, P2, ...) -> R
    Function {
        params: Vec<Ty>,
        return_type: Box<Ty>,
    },

    /// Error type (poison value)
    /// Used when type resolution fails - prevents cascading errors
    Error,

    /// Self type reference
    /// Represents the `Self` keyword within a type context
    SelfType,

    /// Inference placeholder (to be determined by type inference).
    /// Infer types are identified by their containing Ty's TyId.
    Infer,

    /// Type parameter reference (resolved)
    /// This represents a reference to a type parameter within a generic context
    TypeParameter(Arc<TypeParameterSymbol>),

    /// Protocol type (resolved)
    /// This is a reference to a protocol symbol with optional type arguments
    Protocol {
        symbol: Arc<ProtocolSymbol>,
        substitutions: Substitutions,
    },

    /// Struct type (resolved)
    /// This is a reference to a struct symbol with optional type arguments
    Struct {
        symbol: Arc<StructSymbol>,
        substitutions: Substitutions,
    },

    /// Enum type (resolved)
    /// This is a reference to an enum symbol with optional type arguments
    Enum {
        symbol: Arc<EnumSymbol>,
        substitutions: Substitutions,
    },

    /// Type alias type
    /// This is a reference to a type alias symbol with optional type arguments
    /// During type resolution, this should be replaced with the resolved aliased type
    TypeAlias {
        symbol: Arc<TypeAliasSymbol>,
        substitutions: Substitutions,
    },

    /// Associated type reference
    /// Used when referencing an associated type from a protocol, either:
    /// - Within the protocol itself (e.g., `func next() -> Item` in Iterator protocol)
    /// - Via a qualified path (e.g., `T.Item` where T: Iterator)
    AssociatedType {
        /// The associated type symbol from the protocol
        symbol: Arc<AssociatedTypeSymbol>,
        /// The type that contains this associated type (e.g., `T` in `T.Item`)
        /// None when used within the protocol itself
        container: Option<Box<Ty>>,
    },

    /// Unresolved function type (closure whose full type is not yet determined).
    ///
    /// More specific than `Infer` - we know it's a function type, but the exact
    /// parameter types may depend on context. This allows:
    /// - Closures to be recognized as callable before full type inference
    /// - Proper error messages when `it` is used in wrong arity context
    /// - Immediate invocation of closures like `{ 42 }()`
    UnresolvedFunction {
        /// What we know about the parameters
        param_info: ParamInfo,
        /// The return type (may itself be an inference variable)
        return_type: Box<Ty>,
    },
}

impl std::fmt::Debug for TyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use semantic_tree::symbol::Symbol;

        match self {
            TyKind::Unit => write!(f, "Unit"),
            TyKind::Never => write!(f, "Never"),
            TyKind::Int(bits) => write!(f, "Int({:?})", bits),
            TyKind::Float(bits) => write!(f, "Float({:?})", bits),
            TyKind::Bool => write!(f, "Bool"),
            TyKind::String => write!(f, "String"),
            TyKind::Tuple(elems) => {
                write!(f, "Tuple[")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, "]")
            },
            TyKind::Array(elem) => write!(f, "Array[{}]", elem),
            TyKind::Pointer(elem) => write!(f, "Pointer[{}]", elem),
            TyKind::Function { params, return_type } => {
                write!(f, "Function((")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {})", return_type)
            },
            TyKind::Error => write!(f, "Error"),
            TyKind::SelfType => write!(f, "SelfType"),
            TyKind::Infer => write!(f, "Infer"),
            TyKind::TypeParameter(sym) => {
                write!(f, "TypeParameter({})", sym.metadata().name().value)
            },
            TyKind::Protocol { symbol, substitutions } => {
                write!(f, "Protocol({}", symbol.metadata().name().value)?;
                if !substitutions.is_empty() {
                    write!(f, "[")?;
                    let type_params = symbol.type_parameters();
                    for (i, tp) in type_params.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        if let Some(ty) = substitutions.get(tp.metadata().id()) {
                            write!(f, "{}", ty)?;
                        }
                    }
                    write!(f, "]")?;
                }
                write!(f, ")")
            },
            TyKind::Struct { symbol, substitutions } => {
                write!(f, "Struct({}", symbol.metadata().name().value)?;
                if !substitutions.is_empty() {
                    write!(f, "[")?;
                    let type_params = symbol.type_parameters();
                    for (i, tp) in type_params.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        if let Some(ty) = substitutions.get(tp.metadata().id()) {
                            write!(f, "{}", ty)?;
                        }
                    }
                    write!(f, "]")?;
                }
                write!(f, ")")
            },
            TyKind::Enum { symbol, substitutions } => {
                write!(f, "Enum({}", symbol.metadata().name().value)?;
                if !substitutions.is_empty() {
                    write!(f, "[")?;
                    let type_params = symbol.type_parameters();
                    for (i, tp) in type_params.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        if let Some(ty) = substitutions.get(tp.metadata().id()) {
                            write!(f, "{}", ty)?;
                        }
                    }
                    write!(f, "]")?;
                }
                write!(f, ")")
            },
            TyKind::TypeAlias { symbol, substitutions } => {
                write!(f, "TypeAlias({}", symbol.metadata().name().value)?;
                if !substitutions.is_empty() {
                    write!(f, "[")?;
                    let type_params = symbol.type_parameters();
                    for (i, tp) in type_params.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        if let Some(ty) = substitutions.get(tp.metadata().id()) {
                            write!(f, "{}", ty)?;
                        }
                    }
                    write!(f, "]")?;
                }
                write!(f, ")")
            },
            TyKind::AssociatedType { symbol, container } => {
                if let Some(c) = container {
                    write!(f, "AssociatedType({}.{})", c, symbol.metadata().name().value)
                } else {
                    write!(f, "AssociatedType({})", symbol.metadata().name().value)
                }
            },
            TyKind::UnresolvedFunction { param_info, return_type } => {
                write!(f, "UnresolvedFunction({:?} -> {})", param_info, return_type)
            },
        }
    }
}
