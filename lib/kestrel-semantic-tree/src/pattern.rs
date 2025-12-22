//! Pattern data types for the semantic tree.
//!
//! Patterns are plain data structures that represent binding patterns
//! in variable declarations and match expressions. They are created during the bind phase.
//!
//! # Pattern Kinds
//!
//! - `Local`: Simple local binding (`let x` or `var x`)
//! - `Wildcard`: Matches anything, binds nothing (`_`)
//! - `Tuple`: Tuple destructuring (`(a, b, c)`)
//! - `Literal`: Matches exact literal value (`42`, `"hello"`, `true`)
//! - `EnumVariant`: Matches enum case (`.None` or `.Some(value)`)
//! - `Error`: Error recovery pattern

use kestrel_span::Span;
use semantic_tree::symbol::SymbolId;

use crate::expr::LiteralValue;
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

/// A binding within an enum pattern.
///
/// Used for enum cases with associated values:
/// - `.Some(value)` → label = None, pattern binds "value"
/// - `.Point(x: a, y: b)` → label = Some("x"), pattern binds "a"
#[derive(Debug, Clone)]
pub struct EnumPatternBinding {
    /// Optional label (for labeled arguments like `x:` in `.Point(x: a)`)
    pub label: Option<String>,
    /// The pattern for this binding
    pub pattern: Box<Pattern>,
    /// Span of this binding
    pub span: Span,
}

impl EnumPatternBinding {
    /// Create a new enum pattern binding.
    pub fn new(label: Option<String>, pattern: Pattern, span: Span) -> Self {
        EnumPatternBinding {
            label,
            pattern: Box::new(pattern),
            span,
        }
    }

    /// Create an unlabeled binding.
    pub fn unlabeled(pattern: Pattern, span: Span) -> Self {
        Self::new(None, pattern, span)
    }

    /// Create a labeled binding.
    pub fn labeled(label: String, pattern: Pattern, span: Span) -> Self {
        Self::new(Some(label), pattern, span)
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

    /// Wildcard pattern: `_`
    ///
    /// Matches anything but binds nothing. Useful for ignoring values
    /// in pattern matching or tuple destructuring.
    Wildcard,

    /// Tuple pattern: `(a, b, c)`
    ///
    /// Destructures a tuple into its elements. Each element can be
    /// any pattern, including nested tuples or wildcards.
    Tuple {
        /// The element patterns
        elements: Vec<Pattern>,
    },

    /// Literal pattern: `42`, `"hello"`, `true`
    ///
    /// Matches a specific literal value. Only useful in match expressions
    /// (not let bindings) since they are refutable.
    Literal {
        /// The literal value to match
        value: LiteralValue,
    },

    /// Enum variant pattern: `.None` or `.Some(value)`
    ///
    /// Matches an enum case, optionally binding associated values.
    /// The case_id is resolved during type inference when the enum type is known.
    EnumVariant {
        /// The resolved case symbol ID (None if unresolved)
        case_id: Option<SymbolId>,
        /// The case name (e.g., "None" or "Some")
        case_name: String,
        /// Bindings for associated values
        bindings: Vec<EnumPatternBinding>,
    },

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
    pub fn local(
        local_id: LocalId,
        mutability: Mutability,
        name: String,
        ty: Ty,
        span: Span,
    ) -> Self {
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

    /// Create a wildcard pattern.
    ///
    /// Wildcard patterns match anything but don't bind any value.
    pub fn wildcard(ty: Ty, span: Span) -> Self {
        Pattern {
            kind: PatternKind::Wildcard,
            ty,
            span,
        }
    }

    /// Create a tuple pattern.
    ///
    /// The type should be a tuple type with the same arity as the elements.
    pub fn tuple(elements: Vec<Pattern>, ty: Ty, span: Span) -> Self {
        Pattern {
            kind: PatternKind::Tuple { elements },
            ty,
            span,
        }
    }

    /// Create a literal pattern.
    ///
    /// The type should match the literal value's type.
    pub fn literal(value: LiteralValue, ty: Ty, span: Span) -> Self {
        Pattern {
            kind: PatternKind::Literal { value },
            ty,
            span,
        }
    }

    /// Create an enum variant pattern.
    ///
    /// The case_id is initially None and gets resolved during type inference.
    pub fn enum_variant(
        case_id: Option<SymbolId>,
        case_name: String,
        bindings: Vec<EnumPatternBinding>,
        ty: Ty,
        span: Span,
    ) -> Self {
        Pattern {
            kind: PatternKind::EnumVariant {
                case_id,
                case_name,
                bindings,
            },
            ty,
            span,
        }
    }

    /// Create an unresolved enum variant pattern.
    ///
    /// Used when the enum type is not yet known. The type is set to infer.
    pub fn unresolved_enum_variant(
        case_name: String,
        bindings: Vec<EnumPatternBinding>,
        span: Span,
    ) -> Self {
        Pattern {
            kind: PatternKind::EnumVariant {
                case_id: None,
                case_name,
                bindings,
            },
            ty: Ty::infer(span.clone()),
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

    /// Check if this is a wildcard pattern.
    pub fn is_wildcard(&self) -> bool {
        matches!(self.kind, PatternKind::Wildcard)
    }

    /// Check if this is a tuple pattern.
    pub fn is_tuple(&self) -> bool {
        matches!(self.kind, PatternKind::Tuple { .. })
    }

    /// Check if this is a literal pattern.
    pub fn is_literal(&self) -> bool {
        matches!(self.kind, PatternKind::Literal { .. })
    }

    /// Check if this is an enum variant pattern.
    pub fn is_enum_variant(&self) -> bool {
        matches!(self.kind, PatternKind::EnumVariant { .. })
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

    /// Get the tuple elements if this is a tuple pattern.
    pub fn tuple_elements(&self) -> Option<&[Pattern]> {
        match &self.kind {
            PatternKind::Tuple { elements } => Some(elements),
            _ => None,
        }
    }

    /// Get the literal value if this is a literal pattern.
    pub fn literal_value(&self) -> Option<&LiteralValue> {
        match &self.kind {
            PatternKind::Literal { value } => Some(value),
            _ => None,
        }
    }

    /// Get the case name if this is an enum variant pattern.
    pub fn case_name(&self) -> Option<&str> {
        match &self.kind {
            PatternKind::EnumVariant { case_name, .. } => Some(case_name),
            _ => None,
        }
    }

    /// Get the case ID if this is a resolved enum variant pattern.
    pub fn case_id(&self) -> Option<SymbolId> {
        match &self.kind {
            PatternKind::EnumVariant { case_id, .. } => *case_id,
            _ => None,
        }
    }

    /// Get the bindings if this is an enum variant pattern.
    pub fn enum_bindings(&self) -> Option<&[EnumPatternBinding]> {
        match &self.kind {
            PatternKind::EnumVariant { bindings, .. } => Some(bindings),
            _ => None,
        }
    }

    /// Check if this pattern is irrefutable (always matches).
    ///
    /// Irrefutable patterns are required for let/var bindings.
    /// - Wildcard and local bindings are always irrefutable
    /// - Tuple patterns are irrefutable if all elements are irrefutable
    /// - Literal and enum variant patterns are refutable (can fail to match)
    pub fn is_irrefutable(&self) -> bool {
        match &self.kind {
            PatternKind::Local { .. } => true,
            PatternKind::Wildcard => true,
            PatternKind::Tuple { elements } => elements.iter().all(|e| e.is_irrefutable()),
            PatternKind::Literal { .. } => false,
            PatternKind::EnumVariant { .. } => false, // TODO: single-case enums are irrefutable
            PatternKind::Error => true, // Treat errors as irrefutable to avoid cascading errors
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ty::IntBits;
    use kestrel_span::Span;

    #[test]
    fn test_local_pattern() {
        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            Ty::int(IntBits::I64, Span::from(0..1)),
            Span::from(0..5),
        );
        assert!(pattern.is_local());
        assert_eq!(pattern.local_id(), Some(LocalId(0)));
        assert_eq!(pattern.mutability(), Some(Mutability::Immutable));
        assert_eq!(pattern.name(), Some("x"));
        assert!(pattern.is_irrefutable());
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
    fn test_wildcard_pattern() {
        let pattern = Pattern::wildcard(
            Ty::int(IntBits::I64, Span::from(0..1)),
            Span::from(0..1),
        );
        assert!(pattern.is_wildcard());
        assert!(pattern.is_irrefutable());
    }

    #[test]
    fn test_tuple_pattern() {
        let elements = vec![
            Pattern::wildcard(Ty::int(IntBits::I64, Span::from(1..2)), Span::from(1..2)),
            Pattern::local(
                LocalId(0),
                Mutability::Immutable,
                "x".to_string(),
                Ty::int(IntBits::I64, Span::from(4..5)),
                Span::from(4..5),
            ),
        ];
        let tuple_ty = Ty::tuple(
            vec![
                Ty::int(IntBits::I64, Span::from(1..2)),
                Ty::int(IntBits::I64, Span::from(4..5)),
            ],
            Span::from(0..6),
        );
        let pattern = Pattern::tuple(elements, tuple_ty, Span::from(0..6));
        assert!(pattern.is_tuple());
        assert_eq!(pattern.tuple_elements().map(|e| e.len()), Some(2));
        assert!(pattern.is_irrefutable());
    }

    #[test]
    fn test_literal_pattern() {
        let pattern = Pattern::literal(
            LiteralValue::Integer(42),
            Ty::int(IntBits::I64, Span::from(0..2)),
            Span::from(0..2),
        );
        assert!(pattern.is_literal());
        assert_eq!(pattern.literal_value(), Some(&LiteralValue::Integer(42)));
        assert!(!pattern.is_irrefutable()); // Literal patterns are refutable
    }

    #[test]
    fn test_enum_variant_pattern() {
        let pattern = Pattern::unresolved_enum_variant(
            "None".to_string(),
            vec![],
            Span::from(0..5),
        );
        assert!(pattern.is_enum_variant());
        assert_eq!(pattern.case_name(), Some("None"));
        assert_eq!(pattern.case_id(), None);
        assert!(!pattern.is_irrefutable()); // Enum patterns are refutable by default
    }

    #[test]
    fn test_error_pattern() {
        let pattern = Pattern::error(Span::from(0..5));
        assert!(pattern.is_error());
        assert!(pattern.ty.is_error());
        assert!(pattern.is_irrefutable()); // Errors are treated as irrefutable
    }

    #[test]
    fn test_tuple_with_refutable_element_is_refutable() {
        let elements = vec![
            Pattern::literal(
                LiteralValue::Integer(0),
                Ty::int(IntBits::I64, Span::from(1..2)),
                Span::from(1..2),
            ),
            Pattern::local(
                LocalId(0),
                Mutability::Immutable,
                "x".to_string(),
                Ty::int(IntBits::I64, Span::from(4..5)),
                Span::from(4..5),
            ),
        ];
        let tuple_ty = Ty::tuple(
            vec![
                Ty::int(IntBits::I64, Span::from(1..2)),
                Ty::int(IntBits::I64, Span::from(4..5)),
            ],
            Span::from(0..6),
        );
        let pattern = Pattern::tuple(elements, tuple_ty, Span::from(0..6));
        assert!(!pattern.is_irrefutable()); // Has a literal element, so refutable
    }
}
