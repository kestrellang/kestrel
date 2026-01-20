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

/// Represents a bound value in a range pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum RangeBound {
    /// An integer bound: `0`, `10`, `-5`
    Integer(i64),
    /// A character bound: `'a'`, `'z'`
    Char(char),
}

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

/// A field within a struct pattern.
///
/// Used for struct destructuring:
/// - `Point { x, y }` → field_name = "x", pattern binds "x" (shorthand)
/// - `Point { x: a, y: b }` → field_name = "x", pattern binds "a" (explicit)
/// - `Point { x: 0, y }` → field_name = "x", pattern is literal 0
#[derive(Debug, Clone)]
pub struct StructPatternField {
    /// The name of the struct field being matched
    pub field_name: String,
    /// The pattern for this field (could be binding, literal, nested struct, etc.)
    pub pattern: Pattern,
    /// Span of this field pattern
    pub span: Span,
}

impl StructPatternField {
    /// Create a new struct pattern field.
    pub fn new(field_name: String, pattern: Pattern, span: Span) -> Self {
        StructPatternField {
            field_name,
            pattern,
            span,
        }
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

    /// Tuple pattern: `(a, b, c)` or `(first, .., last)`
    ///
    /// Destructures a tuple into its elements. Each element can be
    /// any pattern, including nested tuples or wildcards.
    /// Supports rest patterns (`..`) to match remaining elements.
    Tuple {
        /// Patterns before the rest pattern (or all patterns if no rest)
        prefix: Vec<Pattern>,
        /// Whether there's a rest pattern (`..`)
        has_rest: bool,
        /// Patterns after the rest pattern (empty if no rest or rest is at end)
        suffix: Vec<Pattern>,
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

    /// Range pattern: `0..=9` or `0..<10`
    ///
    /// Matches values within a range. Only valid for integers and characters.
    /// - `0..=9` matches 0, 1, 2, ..., 9 (inclusive)
    /// - `0..<10` matches 0, 1, 2, ..., 9 (exclusive)
    Range {
        /// The start bound of the range
        start: RangeBound,
        /// The end bound of the range
        end: RangeBound,
        /// Whether the end is inclusive (..=) or exclusive (..<)
        inclusive: bool,
    },

    /// Struct pattern: `Point { x, y }` or `Point { x: a, y: b }`
    ///
    /// Matches a struct type and binds its fields.
    /// The struct_id is resolved during type inference when the struct type is known.
    Struct {
        /// The resolved struct symbol ID (None if unresolved)
        struct_id: Option<SymbolId>,
        /// The struct type name (e.g., "Point")
        struct_name: String,
        /// Field patterns
        fields: Vec<StructPatternField>,
        /// Whether the pattern uses `..` to ignore remaining fields
        has_rest: bool,
    },

    /// Array pattern: `[a, b, ..rest]`
    ///
    /// Matches an array/slice and binds its elements.
    /// Supports rest patterns to match variable-length arrays.
    Array {
        /// Patterns before the rest pattern (if any)
        prefix: Vec<Pattern>,
        /// Rest pattern binding: None = no rest, Some((None, _)) = `..`, Some((Some(name), local_id)) = `..name`
        rest: Option<(Option<String>, Option<LocalId>)>,
        /// Patterns after the rest pattern (if any)
        suffix: Vec<Pattern>,
    },

    /// Or-pattern: `p1 or p2 or p3`
    ///
    /// Matches if any of the alternative patterns match.
    /// All alternatives must bind the same names with the same types.
    Or {
        /// The alternative patterns (at least 2)
        alternatives: Vec<Pattern>,
    },

    /// @-pattern: `name @ subpattern`
    ///
    /// Binds the matched value to a name while also matching a subpattern.
    /// For example: `node @ .Cons(head, _)` binds the whole value to `node`
    /// while also destructuring it.
    At {
        /// The binding name
        name: String,
        /// The local ID for the binding
        local_id: LocalId,
        /// Whether the binding is mutable
        mutability: Mutability,
        /// The subpattern to match
        subpattern: Box<Pattern>,
    },

    /// Rest pattern: `..`
    ///
    /// Used in tuple patterns to match remaining elements.
    /// For example: `(first, ..)` or `(.., last)` or `(first, .., last)`
    Rest,

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

    /// Create a tuple pattern without rest.
    ///
    /// The type should be a tuple type with the same arity as the elements.
    pub fn tuple(elements: Vec<Pattern>, ty: Ty, span: Span) -> Self {
        Pattern {
            kind: PatternKind::Tuple {
                prefix: elements,
                has_rest: false,
                suffix: vec![],
            },
            ty,
            span,
        }
    }

    /// Create a tuple pattern with optional rest.
    ///
    /// # Arguments
    /// * `prefix` - Patterns before the rest pattern
    /// * `has_rest` - Whether there's a rest pattern (`..`)
    /// * `suffix` - Patterns after the rest pattern
    /// * `ty` - The tuple type
    /// * `span` - The source span
    pub fn tuple_with_rest(
        prefix: Vec<Pattern>,
        has_rest: bool,
        suffix: Vec<Pattern>,
        ty: Ty,
        span: Span,
    ) -> Self {
        Pattern {
            kind: PatternKind::Tuple {
                prefix,
                has_rest,
                suffix,
            },
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

    /// Create a range pattern.
    ///
    /// The type should be Int for integer ranges or Char for character ranges.
    pub fn range(start: RangeBound, end: RangeBound, inclusive: bool, ty: Ty, span: Span) -> Self {
        Pattern {
            kind: PatternKind::Range {
                start,
                end,
                inclusive,
            },
            ty,
            span,
        }
    }

    /// Create a struct pattern.
    ///
    /// The struct_id is initially None and gets resolved during type inference.
    pub fn struct_pattern(
        struct_id: Option<SymbolId>,
        struct_name: String,
        fields: Vec<StructPatternField>,
        has_rest: bool,
        ty: Ty,
        span: Span,
    ) -> Self {
        Pattern {
            kind: PatternKind::Struct {
                struct_id,
                struct_name,
                fields,
                has_rest,
            },
            ty,
            span,
        }
    }

    /// Create an unresolved struct pattern.
    ///
    /// Used during parsing before type resolution.
    pub fn unresolved_struct_pattern(
        struct_name: String,
        fields: Vec<StructPatternField>,
        has_rest: bool,
        span: Span,
    ) -> Self {
        Pattern {
            kind: PatternKind::Struct {
                struct_id: None,
                struct_name,
                fields,
                has_rest,
            },
            ty: Ty::infer(span.clone()),
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

    /// Check if this is a range pattern.
    pub fn is_range(&self) -> bool {
        matches!(self.kind, PatternKind::Range { .. })
    }

    /// Check if this is a struct pattern.
    pub fn is_struct(&self) -> bool {
        matches!(self.kind, PatternKind::Struct { .. })
    }

    /// Check if this is an or-pattern.
    pub fn is_or(&self) -> bool {
        matches!(self.kind, PatternKind::Or { .. })
    }

    /// Get the alternatives if this is an or-pattern.
    pub fn or_alternatives(&self) -> Option<&[Pattern]> {
        match &self.kind {
            PatternKind::Or { alternatives } => Some(alternatives),
            _ => None,
        }
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
    /// For patterns with rest, returns all non-rest elements (prefix + suffix).
    pub fn tuple_elements(&self) -> Option<Vec<&Pattern>> {
        match &self.kind {
            PatternKind::Tuple { prefix, suffix, .. } => {
                Some(prefix.iter().chain(suffix.iter()).collect())
            }
            _ => None,
        }
    }

    /// Get the tuple prefix elements if this is a tuple pattern.
    pub fn tuple_prefix(&self) -> Option<&[Pattern]> {
        match &self.kind {
            PatternKind::Tuple { prefix, .. } => Some(prefix),
            _ => None,
        }
    }

    /// Get the tuple suffix elements if this is a tuple pattern.
    pub fn tuple_suffix(&self) -> Option<&[Pattern]> {
        match &self.kind {
            PatternKind::Tuple { suffix, .. } => Some(suffix),
            _ => None,
        }
    }

    /// Check if this tuple pattern has a rest pattern.
    pub fn tuple_has_rest(&self) -> Option<bool> {
        match &self.kind {
            PatternKind::Tuple { has_rest, .. } => Some(*has_rest),
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

    /// Check if this is an array pattern.
    pub fn is_array(&self) -> bool {
        matches!(self.kind, PatternKind::Array { .. })
    }

    /// Check if this pattern is irrefutable (always matches).
    ///
    /// Irrefutable patterns are required for let/var bindings.
    /// - Wildcard and local bindings are always irrefutable
    /// - Tuple patterns are irrefutable if all elements are irrefutable
    /// - Struct patterns are irrefutable if all field patterns are irrefutable
    /// - Literal, enum variant, range, and array patterns are refutable (can fail to match)
    pub fn is_irrefutable(&self) -> bool {
        match &self.kind {
            PatternKind::Local { .. } => true,
            PatternKind::Wildcard => true,
            PatternKind::Tuple { prefix, suffix, .. } => prefix
                .iter()
                .chain(suffix.iter())
                .all(|e| e.is_irrefutable()),
            PatternKind::Struct { fields, .. } => {
                // A struct pattern is irrefutable if all field patterns are irrefutable
                fields.iter().all(|f| f.pattern.is_irrefutable())
            }
            PatternKind::Literal { .. } => false,
            PatternKind::EnumVariant { .. } => false, // TODO: single-case enums are irrefutable
            PatternKind::Range { .. } => false,       // Ranges don't cover all values
            PatternKind::Array { .. } => false,       // Array patterns check length
            // Or-patterns are irrefutable if ANY alternative is irrefutable
            PatternKind::Or { alternatives } => alternatives.iter().any(|a| a.is_irrefutable()),
            // At-patterns are irrefutable if the subpattern is irrefutable
            PatternKind::At { subpattern, .. } => subpattern.is_irrefutable(),
            // Rest patterns are always irrefutable (they match any remaining elements)
            PatternKind::Rest => true,
            PatternKind::Error => true, // Treat errors as irrefutable to avoid cascading errors
        }
    }

    /// Get the struct name if this is a struct pattern.
    pub fn struct_name(&self) -> Option<&str> {
        match &self.kind {
            PatternKind::Struct { struct_name, .. } => Some(struct_name),
            _ => None,
        }
    }

    /// Get the struct fields if this is a struct pattern.
    pub fn struct_fields(&self) -> Option<&[StructPatternField]> {
        match &self.kind {
            PatternKind::Struct { fields, .. } => Some(fields),
            _ => None,
        }
    }

    /// Check if this struct pattern has a rest pattern (..)
    pub fn struct_has_rest(&self) -> Option<bool> {
        match &self.kind {
            PatternKind::Struct { has_rest, .. } => Some(*has_rest),
            _ => None,
        }
    }

    /// Create an array pattern.
    ///
    /// # Arguments
    /// * `prefix` - Patterns before the rest pattern
    /// * `rest` - Rest pattern binding: None = no rest, Some((None, _)) = `..`, Some((Some(name), local_id)) = `..name`
    /// * `suffix` - Patterns after the rest pattern
    /// * `ty` - The array type
    /// * `span` - The source span
    pub fn array(
        prefix: Vec<Pattern>,
        rest: Option<(Option<String>, Option<LocalId>)>,
        suffix: Vec<Pattern>,
        ty: Ty,
        span: Span,
    ) -> Self {
        Pattern {
            kind: PatternKind::Array {
                prefix,
                rest,
                suffix,
            },
            ty,
            span,
        }
    }

    /// Create an array pattern.
    ///
    /// # Arguments
    /// * `prefix` - Patterns before the rest pattern (if any)
    /// * `rest` - Rest pattern binding: None = no rest, Some((None, _)) = `..`, Some((Some(name), local_id)) = `..name`
    /// * `suffix` - Patterns after the rest pattern (if any)
    /// * `ty` - The type of the pattern
    /// * `span` - The source span
    pub fn array_pattern(
        prefix: Vec<Pattern>,
        rest: Option<(Option<String>, Option<LocalId>)>,
        suffix: Vec<Pattern>,
        ty: Ty,
        span: Span,
    ) -> Self {
        Pattern {
            kind: PatternKind::Array {
                prefix,
                rest,
                suffix,
            },
            ty,
            span,
        }
    }

    /// Create an or-pattern.
    ///
    /// # Arguments
    /// * `alternatives` - The alternative patterns (at least 2)
    /// * `ty` - The type of the pattern (all alternatives must have compatible types)
    /// * `span` - The source span
    pub fn or_pattern(alternatives: Vec<Pattern>, ty: Ty, span: Span) -> Self {
        Pattern {
            kind: PatternKind::Or { alternatives },
            ty,
            span,
        }
    }

    /// Create an @-pattern.
    ///
    /// # Arguments
    /// * `name` - The binding name
    /// * `local_id` - The local ID for the binding
    /// * `mutability` - Whether the binding is mutable
    /// * `subpattern` - The subpattern to match
    /// * `ty` - The type of the pattern
    /// * `span` - The source span
    pub fn at_pattern(
        name: String,
        local_id: LocalId,
        mutability: Mutability,
        subpattern: Pattern,
        ty: Ty,
        span: Span,
    ) -> Self {
        Pattern {
            kind: PatternKind::At {
                name,
                local_id,
                mutability,
                subpattern: Box::new(subpattern),
            },
            ty,
            span,
        }
    }

    /// Create a rest pattern (`..`).
    ///
    /// # Arguments
    /// * `ty` - The type (usually unit for bare `..`, or the remaining element type)
    /// * `span` - The source span
    pub fn rest(ty: Ty, span: Span) -> Self {
        Pattern {
            kind: PatternKind::Rest,
            ty,
            span,
        }
    }

    /// Check if this is an @-pattern.
    pub fn is_at(&self) -> bool {
        matches!(self.kind, PatternKind::At { .. })
    }

    /// Check if this is a rest pattern.
    pub fn is_rest(&self) -> bool {
        matches!(self.kind, PatternKind::Rest)
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
        let pattern = Pattern::wildcard(Ty::int(IntBits::I64, Span::from(0..1)), Span::from(0..1));
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
        let pattern =
            Pattern::unresolved_enum_variant("None".to_string(), vec![], Span::from(0..5));
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
