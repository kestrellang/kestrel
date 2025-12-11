use crate::symbol::associated_type::AssociatedTypeSymbol;
use crate::symbol::protocol::ProtocolSymbol;
use crate::symbol::r#struct::StructSymbol;
use crate::symbol::type_alias::TypeAliasSymbol;
use crate::symbol::type_parameter::TypeParameterSymbol;
use crate::ty::substitutions::Substitutions;
use crate::ty::{Ty, TypeVarId};
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
    F32,
    F64,
}

/// Represents the kind of a semantic type
/// These are resolved types after semantic analysis
#[derive(Debug, Clone)]
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

    /// Type variable (placeholder for type inference)
    /// Each type variable has a unique ID to distinguish between different
    /// unknown types during inference. The `_` syntax creates a fresh type var.
    TypeVar(TypeVarId),

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
}
