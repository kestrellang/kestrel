use crate::symbol::associated_type::AssociatedTypeSymbol;
use crate::symbol::protocol::ProtocolSymbol;
use crate::symbol::r#struct::StructSymbol;
use crate::symbol::type_alias::TypeAliasSymbol;
use crate::symbol::type_parameter::TypeParameterSymbol;
use crate::ty::substitutions::Substitutions;
use crate::ty::Ty;
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
