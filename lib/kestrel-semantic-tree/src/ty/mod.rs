mod kind;
pub mod substitutions;
pub mod where_clause;

pub use kind::{FloatBits, IntBits, TyKind};
pub use substitutions::Substitutions;
pub use where_clause::{Constraint, WhereClause};

use std::sync::atomic::{AtomicU64, Ordering};
use std::{fmt, fmt::Write as _};

/// Globally unique type identifier.
/// Every `Ty` instance has a unique `TyId` assigned at construction.
/// Used by the type inference system to track and resolve types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TyId(u64);

impl TyId {
    /// Create a new unique type ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        TyId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw ID value (useful for debugging)
    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl Default for TyId {
    fn default() -> Self {
        Self::new()
    }
}

use crate::language::KestrelLanguage;
use crate::symbol::associated_type::AssociatedTypeSymbol;
use crate::symbol::protocol::ProtocolSymbol;
use crate::symbol::r#struct::StructSymbol;
use crate::symbol::type_alias::TypeAliasSymbol;
use crate::symbol::type_parameter::TypeParameterSymbol;
use kestrel_span::Span;
use semantic_tree::symbol::Symbol;
use std::sync::Arc;

/// Represents a semantic type with its kind and source location.
/// Every type has a unique `TyId` assigned at construction for use
/// in type inference constraint solving.
#[derive(Debug, Clone)]
pub struct Ty {
    id: TyId,
    kind: TyKind,
    span: Span,
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn fmt_list(f: &mut fmt::Formatter<'_>, mut first: bool, items: impl Iterator<Item = Ty>) -> fmt::Result {
            for item in items {
                if !first {
                    f.write_str(", ")?;
                }
                first = false;
                write!(f, "{}", item)?;
            }
            Ok(())
        }

        match self.kind() {
            TyKind::Unit => f.write_str("()"),
            TyKind::Never => f.write_str("!"),
            TyKind::Int(bits) => write!(f, "{:?}", bits),
            TyKind::Float(bits) => write!(f, "{:?}", bits),
            TyKind::Bool => f.write_str("Bool"),
            TyKind::String => f.write_str("String"),
            TyKind::Tuple(elements) => {
                f.write_char('(')?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                f.write_char(')')
            }
            TyKind::Array(elem) => write!(f, "[{}]", elem),
            TyKind::Function {
                params,
                return_type,
            } => {
                f.write_char('(')?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", return_type)
            }
            TyKind::TypeParameter(param_symbol) => {
                f.write_str(&param_symbol.metadata().name().value)
            }
            TyKind::Protocol {
                symbol,
                substitutions,
            } => {
                f.write_str(&symbol.metadata().name().value)?;
                if substitutions.is_empty() {
                    return Ok(());
                }
                f.write_char('[')?;
                // Note: Substitutions is a HashMap; ordering is not guaranteed.
                fmt_list(
                    f,
                    true,
                    substitutions.iter().map(|(_, ty)| ty.clone()),
                )?;
                f.write_char(']')
            }
            TyKind::Struct {
                symbol,
                substitutions,
            } => {
                f.write_str(&symbol.metadata().name().value)?;
                if substitutions.is_empty() {
                    return Ok(());
                }
                f.write_char('[')?;
                fmt_list(
                    f,
                    true,
                    substitutions.iter().map(|(_, ty)| ty.clone()),
                )?;
                f.write_char(']')
            }
            TyKind::TypeAlias {
                symbol,
                substitutions,
            } => {
                f.write_str(&symbol.metadata().name().value)?;
                if substitutions.is_empty() {
                    return Ok(());
                }
                f.write_char('[')?;
                fmt_list(
                    f,
                    true,
                    substitutions.iter().map(|(_, ty)| ty.clone()),
                )?;
                f.write_char(']')
            }
            TyKind::AssociatedType { symbol, container } => {
                if let Some(container_ty) = container {
                    write!(f, "{}.", container_ty)?;
                }
                f.write_str(&symbol.metadata().name().value)
            }
            TyKind::SelfType => f.write_str("Self"),
            TyKind::Infer => f.write_str("_"),
            TyKind::Error => f.write_str("<error>"),
        }
    }
}

/// Generate simple type constructors that take only a span
macro_rules! simple_constructor {
    ($($(#[$meta:meta])* $name:ident => $variant:expr),* $(,)?) => {
        $(
            $(#[$meta])*
            pub fn $name(span: Span) -> Self {
                Self::new($variant, span)
            }
        )*
    };
}

/// Generate type checking methods (is_* methods)
macro_rules! is_type {
    ($($(#[$meta:meta])* $name:ident => $pattern:pat),* $(,)?) => {
        $(
            $(#[$meta])*
            pub fn $name(&self) -> bool {
                matches!(self.kind, $pattern)
            }
        )*
    };
}

impl Ty {
    /// Create a new type with the given kind and span.
    /// A fresh `TyId` is automatically assigned.
    pub fn new(kind: TyKind, span: Span) -> Self {
        Self {
            id: TyId::new(),
            kind,
            span,
        }
    }

    /// Get the unique identifier of this type
    pub fn id(&self) -> TyId {
        self.id
    }

    /// Get the kind of this type
    pub fn kind(&self) -> &TyKind {
        &self.kind
    }

    /// Get the span of this type
    pub fn span(&self) -> &Span {
        &self.span
    }

    // === Simple constructors (generated) ===
    simple_constructor! {
        /// Create a unit type: ()
        unit => TyKind::Unit,
        /// Create a never type: !
        never => TyKind::Never,
        /// Create a boolean type
        bool => TyKind::Bool,
        /// Create a string type
        string => TyKind::String,
        /// Create an error type (poison value)
        error => TyKind::Error,
        /// Create a Self type reference
        self_type => TyKind::SelfType,
        /// Create an inference placeholder type
        infer => TyKind::Infer,
    }

    // === Parameterized constructors ===

    /// Create an integer type with the given bit width
    pub fn int(bits: IntBits, span: Span) -> Self {
        Self::new(TyKind::Int(bits), span)
    }

    /// Create a float type with the given bit width
    pub fn float(bits: FloatBits, span: Span) -> Self {
        Self::new(TyKind::Float(bits), span)
    }

    /// Create a tuple type: (T1, T2, ...)
    pub fn tuple(elements: Vec<Ty>, span: Span) -> Self {
        Self::new(TyKind::Tuple(elements), span)
    }

    /// Create an array type: [T]
    pub fn array(element_type: Ty, span: Span) -> Self {
        Self::new(TyKind::Array(Box::new(element_type)), span)
    }

    /// Create a function type: (P1, P2, ...) -> R
    pub fn function(params: Vec<Ty>, return_type: Ty, span: Span) -> Self {
        Self::new(
            TyKind::Function {
                params,
                return_type: Box::new(return_type),
            },
            span,
        )
    }

    /// Create a type parameter reference
    pub fn type_parameter(param_symbol: Arc<TypeParameterSymbol>, span: Span) -> Self {
        Self::new(TyKind::TypeParameter(param_symbol), span)
    }

    /// Create a protocol type (resolved) with no type arguments
    pub fn protocol(protocol_symbol: Arc<ProtocolSymbol>, span: Span) -> Self {
        Self::new(
            TyKind::Protocol {
                symbol: protocol_symbol,
                substitutions: Substitutions::new(),
            },
            span,
        )
    }

    /// Create a generic protocol type (resolved) with type arguments
    pub fn generic_protocol(
        protocol_symbol: Arc<ProtocolSymbol>,
        substitutions: Substitutions,
        span: Span,
    ) -> Self {
        Self::new(
            TyKind::Protocol {
                symbol: protocol_symbol,
                substitutions,
            },
            span,
        )
    }

    /// Create a struct type (resolved) with no type arguments
    pub fn r#struct(struct_symbol: Arc<StructSymbol>, span: Span) -> Self {
        Self::new(
            TyKind::Struct {
                symbol: struct_symbol,
                substitutions: Substitutions::new(),
            },
            span,
        )
    }

    /// Create a generic struct type (resolved) with type arguments
    pub fn generic_struct(
        struct_symbol: Arc<StructSymbol>,
        substitutions: Substitutions,
        span: Span,
    ) -> Self {
        Self::new(
            TyKind::Struct {
                symbol: struct_symbol,
                substitutions,
            },
            span,
        )
    }

    /// Create a type alias type with no type arguments
    pub fn type_alias(type_alias_symbol: Arc<TypeAliasSymbol>, span: Span) -> Self {
        Self::new(
            TyKind::TypeAlias {
                symbol: type_alias_symbol,
                substitutions: Substitutions::new(),
            },
            span,
        )
    }

    /// Create a generic type alias type with type arguments
    pub fn generic_type_alias(
        type_alias_symbol: Arc<TypeAliasSymbol>,
        substitutions: Substitutions,
        span: Span,
    ) -> Self {
        Self::new(
            TyKind::TypeAlias {
                symbol: type_alias_symbol,
                substitutions,
            },
            span,
        )
    }

    /// Create an associated type reference (within a protocol)
    pub fn associated_type(symbol: Arc<AssociatedTypeSymbol>, span: Span) -> Self {
        Self::new(
            TyKind::AssociatedType {
                symbol,
                container: None,
            },
            span,
        )
    }

    /// Create a qualified associated type reference (e.g., T.Item)
    pub fn qualified_associated_type(
        symbol: Arc<AssociatedTypeSymbol>,
        container: Ty,
        span: Span,
    ) -> Self {
        Self::new(
            TyKind::AssociatedType {
                symbol,
                container: Some(Box::new(container)),
            },
            span,
        )
    }

    // === Type joining (for Never propagation) ===

    /// Join two types, handling Never type propagation.
    ///
    /// This is used for computing the type of if expressions and other
    /// control flow constructs where branches may have different types.
    ///
    /// Rules:
    /// - If either type is Never, return the other type (Never is the bottom type)
    /// - If either type is Error, return Error (poison propagation)
    /// - Otherwise, return the first type (type checking validates they match)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // if cond { 1 } else { return }  -> Int (return has type Never)
    /// Ty::int(...).join(&Ty::never(...)) // returns Int
    ///
    /// // if cond { return } else { "hello" } -> String
    /// Ty::never(...).join(&Ty::string(...)) // returns String
    ///
    /// // if cond { return } else { break } -> Never (both are Never)
    /// Ty::never(...).join(&Ty::never(...)) // returns Never
    /// ```
    pub fn join(&self, other: &Ty) -> Ty {
        // Error propagates (poison)
        if self.is_error() || other.is_error() {
            return Ty::error(self.span.clone());
        }

        // Never is the bottom type - it joins with anything to produce the other type
        if self.is_never() {
            return other.clone();
        }
        if other.is_never() {
            return self.clone();
        }

        // Otherwise return self (type checking will validate compatibility)
        self.clone()
    }

    // === Type compatibility ===

    /// Apply substitutions to this type, replacing type parameters with concrete types.
    pub fn apply_substitutions(&self, substitutions: &Substitutions) -> Ty {
        substitutions.apply(self)
    }

    /// Expand type aliases to their underlying types.
    ///
    /// This follows type alias chains until reaching a non-alias type.
    /// Returns a clone of self if not a type alias.
    pub fn expand_aliases(&self) -> Ty {
        use crate::behavior::KestrelBehaviorKind;
        use crate::symbol::type_alias::TypeAliasTypedBehavior;
        use semantic_tree::symbol::Symbol;

        match &self.kind {
            TyKind::TypeAlias {
                symbol,
                substitutions,
            } => {
                // Get the resolved type from the TypeAliasTypedBehavior
                let behaviors = symbol.metadata().behaviors();
                let resolved = behaviors
                    .iter()
                    .find(|b| matches!(b.kind(), KestrelBehaviorKind::TypeAliasTyped))
                    .and_then(|b| b.as_ref().downcast_ref::<TypeAliasTypedBehavior>())
                    .map(|tab| tab.resolved_ty().clone());

                match resolved {
                    Some(ty) => {
                        // Apply substitutions if any, then recursively expand
                        if substitutions.is_empty() {
                            ty.expand_aliases()
                        } else {
                            ty.apply_substitutions(substitutions).expand_aliases()
                        }
                    }
                    // Alias not yet resolved - return as-is
                    None => self.clone(),
                }
            }
            // Not a type alias - return as-is
            _ => self.clone(),
        }
    }

    /// Check if this type is assignable to the target type.
    ///
    /// This handles:
    /// - Never is assignable to any type (bottom type)
    /// - Error is assignable to any type (suppress cascading errors)
    /// - Type aliases are expanded before comparison
    /// - Structural equality for tuples, arrays, functions
    /// - Nominal equality for structs and protocols (by symbol identity)
    /// - Type parameters are compared by identity
    ///
    /// Note: This does NOT handle subtyping or coercions.
    pub fn is_assignable_to(&self, target: &Ty) -> bool {
        // Expand aliases first
        let from = self.expand_aliases();
        let to = target.expand_aliases();

        // Never is assignable to anything (it never produces a value)
        if from.is_never() {
            return true;
        }

        // Error is assignable to anything (suppress cascading errors)
        if from.is_error() || to.is_error() {
            return true;
        }

        // Inference placeholders are compatible with anything (not yet resolved)
        if from.is_infer() || to.is_infer() {
            return true;
        }

        // Compare by kind
        match (from.kind(), to.kind()) {
            // Primitives - exact match
            (TyKind::Unit, TyKind::Unit) => true,
            (TyKind::Bool, TyKind::Bool) => true,
            (TyKind::String, TyKind::String) => true,
            (TyKind::Int(a), TyKind::Int(b)) => a == b,
            (TyKind::Float(a), TyKind::Float(b)) => a == b,

            // Tuples - element-wise comparison
            (TyKind::Tuple(a_elems), TyKind::Tuple(b_elems)) => {
                a_elems.len() == b_elems.len()
                    && a_elems
                        .iter()
                        .zip(b_elems.iter())
                        .all(|(a, b)| a.is_assignable_to(b))
            }

            // Arrays - element type comparison
            (TyKind::Array(a_elem), TyKind::Array(b_elem)) => a_elem.is_assignable_to(b_elem),

            // Functions - contravariant params, covariant return
            // For now, we use simple equality (no variance)
            (
                TyKind::Function {
                    params: a_params,
                    return_type: a_ret,
                },
                TyKind::Function {
                    params: b_params,
                    return_type: b_ret,
                },
            ) => {
                a_params.len() == b_params.len()
                    && a_params
                        .iter()
                        .zip(b_params.iter())
                        .all(|(a, b)| a.is_assignable_to(b))
                    && a_ret.is_assignable_to(b_ret)
            }

            // Structs - nominal equality (same symbol by ID)
            (
                TyKind::Struct {
                    symbol: a_sym,
                    substitutions: a_subs,
                },
                TyKind::Struct {
                    symbol: b_sym,
                    substitutions: b_subs,
                },
            ) => {
                // Same struct symbol by ID (not pointer) to handle different Arc instances
                Symbol::<KestrelLanguage>::metadata(a_sym.as_ref()).id()
                    == Symbol::<KestrelLanguage>::metadata(b_sym.as_ref()).id()
                    && substitutions_equal(a_subs, b_subs)
            }

            // Protocols - nominal equality (same symbol by ID)
            (
                TyKind::Protocol {
                    symbol: a_sym,
                    substitutions: a_subs,
                },
                TyKind::Protocol {
                    symbol: b_sym,
                    substitutions: b_subs,
                },
            ) => {
                Symbol::<KestrelLanguage>::metadata(a_sym.as_ref()).id()
                    == Symbol::<KestrelLanguage>::metadata(b_sym.as_ref()).id()
                    && substitutions_equal(a_subs, b_subs)
            }

            // Type parameters - only the same type parameter is assignable to itself
            // Different type parameters (T vs U) are not compatible even with shared bounds
            (TyKind::TypeParameter(a), TyKind::TypeParameter(b)) => {
                Symbol::<KestrelLanguage>::metadata(a.as_ref()).id()
                    == Symbol::<KestrelLanguage>::metadata(b.as_ref()).id()
            }

            // Type parameter vs concrete type - not assignable
            // Future: where clause equality constraints (T == U, T.Item == Int) could allow this
            (TyKind::TypeParameter(_), _) | (_, TyKind::TypeParameter(_)) => false,

            // Self type - equal to Self or any Struct/Protocol
            // In a struct/protocol context, Self represents the containing type
            (TyKind::SelfType, TyKind::SelfType) => true,
            (TyKind::SelfType, TyKind::Struct { .. }) => true,
            (TyKind::Struct { .. }, TyKind::SelfType) => true,
            (TyKind::SelfType, TyKind::Protocol { .. }) => true,
            (TyKind::Protocol { .. }, TyKind::SelfType) => true,

            // Associated types - by symbol ID for now
            // Full checking requires constraint verification
            (
                TyKind::AssociatedType { symbol: a_sym, .. },
                TyKind::AssociatedType { symbol: b_sym, .. },
            ) => {
                Symbol::<KestrelLanguage>::metadata(a_sym.as_ref()).id()
                    == Symbol::<KestrelLanguage>::metadata(b_sym.as_ref()).id()
            }

            // Associated type can be assigned to anything (and vice versa) for now
            // This allows generic code to compile before constraint checking
            // (TyKind::AssociatedType { .. }, _) | (_, TyKind::AssociatedType { .. }) => true,

            // Everything else is not assignable
            _ => false,
        }
    }

    // === Type checking methods (generated) ===
    is_type! {
        /// Check if this is a unit type
        is_unit => TyKind::Unit,
        /// Check if this is a never type
        is_never => TyKind::Never,
        /// Check if this is an integer type
        is_int => TyKind::Int(_),
        /// Check if this is a float type
        is_float => TyKind::Float(_),
        /// Check if this is a boolean type
        is_bool => TyKind::Bool,
        /// Check if this is a string type
        is_string => TyKind::String,
        /// Check if this is a tuple type
        is_tuple => TyKind::Tuple(_),
        /// Check if this is an array type
        is_array => TyKind::Array(_),
        /// Check if this is a function type
        is_function => TyKind::Function { .. },
        /// Check if this is an error type
        is_error => TyKind::Error,
        /// Check if this is a Self type reference
        is_self_type => TyKind::SelfType,
        /// Check if this is an inference placeholder type
        is_infer => TyKind::Infer,
        /// Check if this is a type parameter type
        is_type_parameter => TyKind::TypeParameter(_),
        /// Check if this is a protocol type (resolved)
        is_protocol => TyKind::Protocol { .. },
        /// Check if this is a struct type (resolved)
        is_struct => TyKind::Struct { .. },
        /// Check if this is a type alias type
        is_type_alias => TyKind::TypeAlias { .. },
        /// Check if this is an associated type reference
        is_associated_type => TyKind::AssociatedType { .. },
    }

    // === Accessor methods ===

    /// Get integer bit width if this is an integer type
    pub fn as_int(&self) -> Option<IntBits> {
        match &self.kind {
            TyKind::Int(bits) => Some(*bits),
            _ => None,
        }
    }

    /// Get float bit width if this is a float type
    pub fn as_float(&self) -> Option<FloatBits> {
        match &self.kind {
            TyKind::Float(bits) => Some(*bits),
            _ => None,
        }
    }

    /// Get tuple elements if this is a tuple type
    pub fn as_tuple(&self) -> Option<&Vec<Ty>> {
        match &self.kind {
            TyKind::Tuple(elements) => Some(elements),
            _ => None,
        }
    }

    /// Get array element type if this is an array type
    pub fn as_array(&self) -> Option<&Ty> {
        match &self.kind {
            TyKind::Array(element_type) => Some(element_type),
            _ => None,
        }
    }

    /// Get function parameters and return type if this is a function type
    pub fn as_function(&self) -> Option<(&Vec<Ty>, &Ty)> {
        match &self.kind {
            TyKind::Function {
                params,
                return_type,
            } => Some((params, return_type)),
            _ => None,
        }
    }

    /// Get type parameter symbol if this is a type parameter type
    pub fn as_type_parameter(&self) -> Option<&Arc<TypeParameterSymbol>> {
        match &self.kind {
            TyKind::TypeParameter(symbol) => Some(symbol),
            _ => None,
        }
    }

    /// Get protocol symbol if this is a protocol type
    pub fn as_protocol(&self) -> Option<&Arc<ProtocolSymbol>> {
        match &self.kind {
            TyKind::Protocol { symbol, .. } => Some(symbol),
            _ => None,
        }
    }

    /// Get protocol symbol and substitutions if this is a protocol type
    pub fn as_protocol_with_subs(&self) -> Option<(&Arc<ProtocolSymbol>, &Substitutions)> {
        match &self.kind {
            TyKind::Protocol {
                symbol,
                substitutions,
            } => Some((symbol, substitutions)),
            _ => None,
        }
    }

    /// Get struct symbol if this is a struct type
    pub fn as_struct(&self) -> Option<&Arc<StructSymbol>> {
        match &self.kind {
            TyKind::Struct { symbol, .. } => Some(symbol),
            _ => None,
        }
    }

    /// Get struct symbol and substitutions if this is a struct type
    pub fn as_struct_with_subs(&self) -> Option<(&Arc<StructSymbol>, &Substitutions)> {
        match &self.kind {
            TyKind::Struct {
                symbol,
                substitutions,
            } => Some((symbol, substitutions)),
            _ => None,
        }
    }

    /// Get type alias symbol if this is a type alias type
    pub fn as_type_alias(&self) -> Option<&Arc<TypeAliasSymbol>> {
        match &self.kind {
            TyKind::TypeAlias { symbol, .. } => Some(symbol),
            _ => None,
        }
    }

    /// Get type alias symbol and substitutions if this is a type alias type
    pub fn as_type_alias_with_subs(&self) -> Option<(&Arc<TypeAliasSymbol>, &Substitutions)> {
        match &self.kind {
            TyKind::TypeAlias {
                symbol,
                substitutions,
            } => Some((symbol, substitutions)),
            _ => None,
        }
    }

    /// Get associated type symbol if this is an associated type reference
    pub fn as_associated_type(&self) -> Option<&Arc<AssociatedTypeSymbol>> {
        match &self.kind {
            TyKind::AssociatedType { symbol, .. } => Some(symbol),
            _ => None,
        }
    }

    /// Get associated type symbol and container if this is an associated type reference
    pub fn as_associated_type_with_container(
        &self,
    ) -> Option<(&Arc<AssociatedTypeSymbol>, Option<&Ty>)> {
        match &self.kind {
            TyKind::AssociatedType { symbol, container } => {
                Some((symbol, container.as_ref().map(|b| b.as_ref())))
            }
            _ => None,
        }
    }
}

/// Check if two substitution maps are equal (all mapped types are assignable).
///
/// For now, if both have the same length, we check positionally rather than by ID.
/// This is because type parameters from different scopes have different IDs
/// even when they represent the same logical type parameter.
fn substitutions_equal(a: &Substitutions, b: &Substitutions) -> bool {
    if a.len() != b.len() {
        return false;
    }

    // If both are empty, they're equal
    if a.is_empty() {
        return true;
    }

    // Compare substitutions positionally (sorted by ID for consistency)
    let mut a_types: Vec<_> = a.iter().collect();
    let mut b_types: Vec<_> = b.iter().collect();
    a_types.sort_by_key(|(id, _)| id.raw());
    b_types.sort_by_key(|(id, _)| id.raw());

    for ((_, a_ty), (_, b_ty)) in a_types.iter().zip(b_types.iter()) {
        if !a_ty.is_assignable_to(b_ty) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_span::Span;

    #[test]
    fn test_unit_type() {
        let ty = Ty::unit(Span::from(0..2));
        assert!(ty.is_unit());
        assert!(!ty.is_never());
        assert!(!ty.is_tuple());
        assert!(!ty.is_function());
        assert!(!ty.is_error());
        assert!(!ty.is_type_alias());
    }

    #[test]
    fn test_never_type() {
        let ty = Ty::never(Span::from(0..1));
        assert!(!ty.is_unit());
        assert!(ty.is_never());
    }

    #[test]
    fn test_error_type() {
        let ty = Ty::error(Span::from(0..5));
        assert!(ty.is_error());
        assert!(!ty.is_unit());
    }

    #[test]
    fn test_self_type() {
        let ty = Ty::self_type(Span::from(0..4));
        assert!(ty.is_self_type());
        assert!(!ty.is_unit());
    }

    #[test]
    fn test_tuple_type() {
        let ty = Ty::tuple(
            vec![Ty::unit(Span::from(0..2)), Ty::never(Span::from(3..4))],
            Span::from(0..5),
        );
        assert!(ty.is_tuple());
        assert!(!ty.is_unit());

        let elements = ty.as_tuple().unwrap();
        assert_eq!(elements.len(), 2);
        assert!(elements[0].is_unit());
        assert!(elements[1].is_never());
    }

    #[test]
    fn test_function_type() {
        let ty = Ty::function(
            vec![Ty::unit(Span::from(0..2)), Ty::never(Span::from(4..5))],
            Ty::unit(Span::from(10..12)),
            Span::from(0..12),
        );
        assert!(ty.is_function());
        assert!(!ty.is_unit());

        let (params, ret) = ty.as_function().unwrap();
        assert_eq!(params.len(), 2);
        assert!(params[0].is_unit());
        assert!(params[1].is_never());
        assert!(ret.is_unit());
    }

    #[test]
    fn test_nested_types() {
        let tuple_param = Ty::tuple(
            vec![Ty::unit(Span::from(1..3)), Ty::never(Span::from(5..6))],
            Span::from(0..7),
        );
        let fn_ty = Ty::function(
            vec![tuple_param],
            Ty::unit(Span::from(12..14)),
            Span::from(0..14),
        );

        assert!(fn_ty.is_function());

        let (params, ret) = fn_ty.as_function().unwrap();
        assert_eq!(params.len(), 1);
        assert!(params[0].is_tuple());
        assert!(ret.is_unit());

        let tuple_elements = params[0].as_tuple().unwrap();
        assert_eq!(tuple_elements.len(), 2);
        assert!(tuple_elements[0].is_unit());
        assert!(tuple_elements[1].is_never());
    }

    #[test]
    fn test_join_never_with_int() {
        let never = Ty::never(Span::from(0..1));
        let int = Ty::int(IntBits::I64, Span::from(0..3));

        // Never joined with Int should give Int
        let result = never.join(&int);
        assert!(result.is_int());

        // Int joined with Never should give Int
        let result = int.join(&never);
        assert!(result.is_int());
    }

    #[test]
    fn test_join_never_with_never() {
        let never1 = Ty::never(Span::from(0..1));
        let never2 = Ty::never(Span::from(2..3));

        // Never joined with Never should give Never
        let result = never1.join(&never2);
        assert!(result.is_never());
    }

    #[test]
    fn test_join_error_propagates() {
        let error = Ty::error(Span::from(0..1));
        let int = Ty::int(IntBits::I64, Span::from(0..3));

        // Error joined with anything should give Error
        let result = error.join(&int);
        assert!(result.is_error());

        // Anything joined with Error should give Error
        let result = int.join(&error);
        assert!(result.is_error());
    }

    #[test]
    fn test_join_same_types() {
        let int1 = Ty::int(IntBits::I64, Span::from(0..3));
        let int2 = Ty::int(IntBits::I64, Span::from(4..7));

        // Same types should return the first
        let result = int1.join(&int2);
        assert!(result.is_int());
    }

    // === is_assignable_to tests ===

    #[test]
    fn test_assignable_same_primitives() {
        let int1 = Ty::int(IntBits::I64, Span::from(0..3));
        let int2 = Ty::int(IntBits::I64, Span::from(4..7));
        assert!(int1.is_assignable_to(&int2));

        let bool1 = Ty::bool(Span::from(0..4));
        let bool2 = Ty::bool(Span::from(5..9));
        assert!(bool1.is_assignable_to(&bool2));

        let string1 = Ty::string(Span::from(0..6));
        let string2 = Ty::string(Span::from(7..13));
        assert!(string1.is_assignable_to(&string2));

        let unit1 = Ty::unit(Span::from(0..2));
        let unit2 = Ty::unit(Span::from(3..5));
        assert!(unit1.is_assignable_to(&unit2));
    }

    #[test]
    fn test_not_assignable_different_primitives() {
        let int = Ty::int(IntBits::I64, Span::from(0..3));
        let float = Ty::float(FloatBits::F64, Span::from(0..3));
        let bool_ty = Ty::bool(Span::from(0..4));
        let string = Ty::string(Span::from(0..6));

        assert!(!int.is_assignable_to(&float));
        assert!(!int.is_assignable_to(&bool_ty));
        assert!(!int.is_assignable_to(&string));
        assert!(!float.is_assignable_to(&int));
        assert!(!bool_ty.is_assignable_to(&string));
    }

    #[test]
    fn test_never_assignable_to_anything() {
        let never = Ty::never(Span::from(0..1));
        let int = Ty::int(IntBits::I64, Span::from(0..3));
        let string = Ty::string(Span::from(0..6));
        let unit = Ty::unit(Span::from(0..2));

        assert!(never.is_assignable_to(&int));
        assert!(never.is_assignable_to(&string));
        assert!(never.is_assignable_to(&unit));
        assert!(never.is_assignable_to(&never));
    }

    #[test]
    fn test_error_assignable_to_anything() {
        let error = Ty::error(Span::from(0..1));
        let int = Ty::int(IntBits::I64, Span::from(0..3));

        // Error is assignable to anything (suppress cascading)
        assert!(error.is_assignable_to(&int));
        assert!(int.is_assignable_to(&error));
    }

    #[test]
    fn test_assignable_tuples() {
        let tuple1 = Ty::tuple(
            vec![
                Ty::int(IntBits::I64, Span::from(0..3)),
                Ty::bool(Span::from(4..8)),
            ],
            Span::from(0..9),
        );
        let tuple2 = Ty::tuple(
            vec![
                Ty::int(IntBits::I64, Span::from(10..13)),
                Ty::bool(Span::from(14..18)),
            ],
            Span::from(10..19),
        );
        assert!(tuple1.is_assignable_to(&tuple2));

        // Different element types
        let tuple3 = Ty::tuple(
            vec![Ty::string(Span::from(0..6)), Ty::bool(Span::from(7..11))],
            Span::from(0..12),
        );
        assert!(!tuple1.is_assignable_to(&tuple3));

        // Different lengths
        let tuple4 = Ty::tuple(
            vec![Ty::int(IntBits::I64, Span::from(0..3))],
            Span::from(0..4),
        );
        assert!(!tuple1.is_assignable_to(&tuple4));
    }

    #[test]
    fn test_assignable_arrays() {
        let arr1 = Ty::array(Ty::int(IntBits::I64, Span::from(0..3)), Span::from(0..5));
        let arr2 = Ty::array(Ty::int(IntBits::I64, Span::from(6..9)), Span::from(6..11));
        assert!(arr1.is_assignable_to(&arr2));

        // Different element types
        let arr3 = Ty::array(Ty::string(Span::from(0..6)), Span::from(0..8));
        assert!(!arr1.is_assignable_to(&arr3));
    }

    #[test]
    fn test_assignable_functions() {
        let fn1 = Ty::function(
            vec![Ty::int(IntBits::I64, Span::from(0..3))],
            Ty::bool(Span::from(4..8)),
            Span::from(0..9),
        );
        let fn2 = Ty::function(
            vec![Ty::int(IntBits::I64, Span::from(10..13))],
            Ty::bool(Span::from(14..18)),
            Span::from(10..19),
        );
        assert!(fn1.is_assignable_to(&fn2));

        // Different param types
        let fn3 = Ty::function(
            vec![Ty::string(Span::from(0..6))],
            Ty::bool(Span::from(7..11)),
            Span::from(0..12),
        );
        assert!(!fn1.is_assignable_to(&fn3));

        // Different return type
        let fn4 = Ty::function(
            vec![Ty::int(IntBits::I64, Span::from(0..3))],
            Ty::string(Span::from(4..10)),
            Span::from(0..11),
        );
        assert!(!fn1.is_assignable_to(&fn4));

        // Different arity
        let fn5 = Ty::function(
            vec![
                Ty::int(IntBits::I64, Span::from(0..3)),
                Ty::int(IntBits::I64, Span::from(4..7)),
            ],
            Ty::bool(Span::from(8..12)),
            Span::from(0..13),
        );
        assert!(!fn1.is_assignable_to(&fn5));
    }

    #[test]
    fn test_int_bit_widths_not_assignable() {
        let i32_ty = Ty::int(IntBits::I32, Span::from(0..3));
        let i64_ty = Ty::int(IntBits::I64, Span::from(0..3));

        // Different bit widths are not assignable
        assert!(!i32_ty.is_assignable_to(&i64_ty));
        assert!(!i64_ty.is_assignable_to(&i32_ty));
    }
}
