//! Pattern data types for the semantic tree.
//!
//! Patterns are plain data structures that represent binding patterns
//! in variable declarations. They are created during the bind phase.

use kestrel_span::Span;

use crate::symbol::local::LocalId;
use crate::ty::Ty;

/// Represents whether a binding is mutable or immutable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutability {
    /// Immutable binding: `let x = ...`
    Immutable,
    /// Mutable binding: `var x = ...`
    Mutable,
}

impl Mutability {
    /// Returns true if this is a mutable binding.
    pub fn is_mutable(&self) -> bool {
        matches!(self, Mutability::Mutable)
    }

    /// Returns true if this is an immutable binding.
    pub fn is_immutable(&self) -> bool {
        matches!(self, Mutability::Immutable)
    }
}

/// Represents the kind of pattern.
#[derive(Debug, Clone)]
pub enum PatternKind {
    /// Simple local binding: `let x` or `var x`
    Local {
        /// The local variable ID in the function's locals vector
        local_id: LocalId,
        /// Whether this is a mutable or immutable binding
        mutability: Mutability,
        /// The name of the binding
        name: String,
    },
    // Future: Tuple, Struct, Wildcard, etc.

    /// Error pattern (poison value).
    /// Used when pattern resolution fails - prevents cascading errors.
    Error,
}

/// A resolved pattern in the semantic tree.
///
/// Unlike symbols, patterns are plain data structures without SymbolId.
/// They are created during the bind phase.
#[derive(Debug, Clone)]
pub struct Pattern {
    /// The kind of pattern
    pub kind: PatternKind,
    /// The resolved type of this pattern
    pub ty: Ty,
    /// The source span of this pattern
    pub span: Span,
}

impl Pattern {
    /// Create a new pattern.
    pub fn new(kind: PatternKind, ty: Ty, span: Span) -> Self {
        Pattern { kind, ty, span }
    }

    /// Create a local binding pattern.
    pub fn local(local_id: LocalId, mutability: Mutability, name: String, ty: Ty, span: Span) -> Self {
        Pattern {
            kind: PatternKind::Local {
                local_id,
                mutability,
                name,
            },
            ty,
            span,
        }
    }

    /// Create an error pattern (poison value).
    pub fn error(span: Span) -> Self {
        Pattern {
            kind: PatternKind::Error,
            ty: Ty::error(span.clone()),
            span,
        }
    }

    /// Check if this is an error pattern.
    pub fn is_error(&self) -> bool {
        matches!(self.kind, PatternKind::Error)
    }

    /// Check if this is a local binding pattern.
    pub fn is_local(&self) -> bool {
        matches!(self.kind, PatternKind::Local { .. })
    }

    /// Get the local ID if this is a local binding pattern.
    pub fn local_id(&self) -> Option<LocalId> {
        match &self.kind {
            PatternKind::Local { local_id, .. } => Some(*local_id),
            _ => None,
        }
    }

    /// Get the mutability if this is a local binding pattern.
    pub fn mutability(&self) -> Option<Mutability> {
        match &self.kind {
            PatternKind::Local { mutability, .. } => Some(*mutability),
            _ => None,
        }
    }

    /// Get the name if this is a local binding pattern.
    pub fn name(&self) -> Option<&str> {
        match &self.kind {
            PatternKind::Local { name, .. } => Some(name),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use kestrel_span::Span;
    use super::*;

    #[test]
    fn test_local_pattern() {
        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            Ty::int(crate::ty::IntBits::I64, Span::from(0..1)),
            Span::from(0..5),
        );
        assert!(pattern.is_local());
        assert_eq!(pattern.local_id(), Some(LocalId(0)));
        assert_eq!(pattern.mutability(), Some(Mutability::Immutable));
        assert_eq!(pattern.name(), Some("x"));
    }

    #[test]
    fn test_mutable_pattern() {
        let pattern = Pattern::local(
            LocalId(1),
            Mutability::Mutable,
            "y".to_string(),
            Ty::string(Span::from(0..1)),
            Span::from(0..5),
        );
        assert!(pattern.mutability().unwrap().is_mutable());
    }

    #[test]
    fn test_error_pattern() {
        let pattern = Pattern::error(Span::from(0..5));
        assert!(pattern.is_error());
        assert!(pattern.ty.is_error());
    }
}
