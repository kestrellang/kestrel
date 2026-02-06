//! Resolution types for semantic analysis
//!
//! This module provides types for representing the results of name resolution.

use kestrel_semantic_tree::ty::Ty;
use semantic_tree::symbol::SymbolId;

/// Result of name resolution
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolResolution {
    /// Successfully resolved to one or more symbols
    Found(Vec<SymbolId>),
    /// Name not found in any scope
    NotFound,
    /// Name found but ambiguous (multiple candidates)
    Ambiguous(Vec<SymbolId>),
}

impl SymbolResolution {
    pub fn is_found(&self) -> bool {
        matches!(self, SymbolResolution::Found(_))
    }

    pub fn single(&self) -> Option<SymbolId> {
        match self {
            SymbolResolution::Found(ids) if ids.len() == 1 => Some(ids[0]),
            _ => None,
        }
    }
}

/// Result of type path resolution
#[derive(Debug, Clone)]
pub enum TypePathResolution {
    /// Successfully resolved to a type
    Resolved(Ty),
    /// A segment in the path was not found
    NotFound {
        /// The segment that wasn't found
        segment: String,
        /// Index of the failed segment in the path
        index: usize,
    },
    /// A segment resolved to multiple candidates (ambiguous)
    Ambiguous {
        /// The ambiguous segment
        segment: String,
        /// Index of the ambiguous segment
        index: usize,
        /// The candidate symbol IDs
        candidates: Vec<SymbolId>,
    },
    /// The final symbol doesn't have a type (not a type-defining symbol)
    NotAType {
        /// The symbol that isn't a type
        symbol_id: SymbolId,
    },
}

impl TypePathResolution {
    /// Returns true if resolution succeeded
    pub fn is_resolved(&self) -> bool {
        matches!(self, TypePathResolution::Resolved(_))
    }

    /// Returns the resolved type if successful
    pub fn ty(&self) -> Option<&Ty> {
        match self {
            TypePathResolution::Resolved(ty) => Some(ty),
            _ => None,
        }
    }
}

/// Result of value path resolution (for expressions)
#[derive(Debug, Clone)]
pub enum ValuePathResolution {
    /// Successfully resolved to a symbol with ValueBehavior
    Symbol {
        /// The resolved symbol
        symbol_id: SymbolId,
        /// The type of the value
        ty: Ty,
    },
    /// Resolved to multiple symbols (overloaded functions)
    /// Caller must disambiguate based on context
    Overloaded {
        /// The candidate symbol IDs (all have CallableBehavior)
        candidates: Vec<SymbolId>,
    },
    /// Resolved to a type parameter (for static method calls like T.create())
    TypeParameter {
        /// The type parameter symbol ID
        symbol_id: SymbolId,
    },
    /// Resolved to an associated type (for static member access like Item.zero)
    /// The remaining segments should be handled as member accesses by the caller.
    AssociatedType {
        /// The associated type symbol ID
        symbol_id: SymbolId,
        /// The container type (e.g., for I.Item, the container is I)
        container: Option<Ty>,
    },
    /// Resolved to an enum case value, but there are more path segments.
    /// The remaining segments should be handled as member accesses by the caller.
    /// This handles cases like `Player.player1.description()` where `player1` is an enum case
    /// and `description` is a method on the enum type.
    EnumCaseValue {
        /// The enum case symbol ID
        symbol_id: SymbolId,
        /// The type of the enum case value (the enum type)
        ty: Ty,
        /// The index in the path where the enum case was found
        /// (segments after this should be member accesses)
        resolved_index: usize,
    },
    /// Resolved to a field/getter value, but there are more path segments.
    /// The remaining segments should be handled as member accesses by the caller.
    /// This handles cases like `Float64.e.subtract(1.0)` where `e` is a static field
    /// and `subtract` is a method on the field's value type.
    FieldValue {
        /// The field symbol ID
        symbol_id: SymbolId,
        /// The type of the field value
        ty: Ty,
        /// The index in the path where the field was found
        /// (segments after this should be member accesses)
        resolved_index: usize,
    },
    /// A segment in the path was not found
    NotFound {
        /// The segment that wasn't found
        segment: String,
        /// Index of the failed segment in the path
        index: usize,
    },
    /// A segment resolved to multiple non-overload candidates (ambiguous)
    Ambiguous {
        /// The ambiguous segment
        segment: String,
        /// Index of the ambiguous segment
        index: usize,
        /// The candidate symbol IDs
        candidates: Vec<SymbolId>,
    },
    /// The final symbol doesn't have ValueBehavior (not a value)
    NotAValue {
        /// The symbol that isn't a value
        symbol_id: SymbolId,
    },
}

impl ValuePathResolution {
    /// Returns true if resolution succeeded
    pub fn is_resolved(&self) -> bool {
        matches!(
            self,
            ValuePathResolution::Symbol { .. }
                | ValuePathResolution::Overloaded { .. }
                | ValuePathResolution::TypeParameter { .. }
                | ValuePathResolution::AssociatedType { .. }
                | ValuePathResolution::EnumCaseValue { .. }
                | ValuePathResolution::FieldValue { .. }
        )
    }

    /// Returns true if this resolved to a type parameter
    pub fn is_type_parameter(&self) -> bool {
        matches!(self, ValuePathResolution::TypeParameter { .. })
    }

    /// Returns the type parameter symbol ID if resolved to one
    pub fn type_parameter_id(&self) -> Option<SymbolId> {
        match self {
            ValuePathResolution::TypeParameter { symbol_id } => Some(*symbol_id),
            _ => None,
        }
    }

    /// Returns the single resolved symbol if not overloaded
    pub fn single(&self) -> Option<(SymbolId, &Ty)> {
        match self {
            ValuePathResolution::Symbol { symbol_id, ty } => Some((*symbol_id, ty)),
            _ => None,
        }
    }

    /// Returns the type if resolved to a single symbol
    pub fn ty(&self) -> Option<&Ty> {
        match self {
            ValuePathResolution::Symbol { ty, .. } => Some(ty),
            _ => None,
        }
    }

    /// Returns true if this is an overloaded resolution
    pub fn is_overloaded(&self) -> bool {
        matches!(self, ValuePathResolution::Overloaded { .. })
    }

    /// Returns the overload candidates if overloaded
    pub fn overload_candidates(&self) -> Option<&[SymbolId]> {
        match self {
            ValuePathResolution::Overloaded { candidates } => Some(candidates),
            _ => None,
        }
    }
}
