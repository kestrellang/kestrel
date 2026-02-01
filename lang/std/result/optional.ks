// Optional[T] - represents a value that may or may not be present

module std.result

import std.core.(Equatable, Formattable, Bool, ControlFlow, Tryable, FromResidual, FromValue, ExpressibleByNullLiteral, Coalesce)
import std.text.(String)
// Note: Iterator import creates circular dependency - Iterator imports Optional
// import std.iter.(Iterator)

/// Represents an optional value: either `Some(value)` or `None`.
///
/// Used for values that may be absent, providing a type-safe alternative to
/// null pointers. The compiler enforces handling of the None case.
///
/// Syntactic sugar:
///     T?           desugars to  Optional[T]
///     null         creates      Optional.None
@builtin(.OptionalEnum)
public enum Optional[T] {
    /// Contains a value.
    @builtin(.OptionalSomeCase)
    case Some(T)

    /// Contains no value.
    @builtin(.OptionalNoneCase)
    case None

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Creates a Some containing the value.
    public static func some(value: T) -> Optional[T] {
        .Some(value)
    }

    /// Creates a None.
    public static func none() -> Optional[T] {
        .None
    }

    // ========================================================================
    // QUERY METHODS
    // ========================================================================

    /// Returns true if this is Some.
    public func isSome() -> Bool {
        match self {
            .Some(_) => true,
            .None => false
        }
    }

    /// Returns true if this is None.
    public func isNone() -> Bool {
        match self {
            .Some(_) => false,
            .None => true
        }
    }

    // ========================================================================
    // UNWRAPPING
    // ========================================================================

    /// Returns the contained value, panicking if None.
    ///
    /// WARNING: Only use when you are certain the value is Some.
    /// Prefer `unwrapOr`, `unwrap(orElse:)`, or pattern matching.
    public func unwrap() -> T {
        match self {
            .Some(value) => value,
            .None => lang.panic("called unwrap() on None")
        }
    }

    /// Returns the contained value or the default if None.
    ///
    /// Note: The default is eagerly evaluated. Use `unwrap(orElse:)` for
    /// lazy evaluation when the default is expensive to compute.
    public func unwrapOr(default: T) -> T {
        match self {
            .Some(value) => value,
            .None => default
        }
    }

    /// Returns the contained value or calls the function if None.
    ///
    /// The function is only called if the Optional is None.
    public func unwrap(orElse defaultFn: () -> T) -> T {
        match self {
            .Some(value) => value,
            .None => defaultFn()
        }
    }

    // ========================================================================
    // TRANSFORMATIONS
    // ========================================================================

    /// Transforms the contained value using the function.
    ///
    /// Returns None if this is None, otherwise applies the function to the
    /// contained value and wraps the result in Some.
    public func map[U](transform: (T) -> U) -> Optional[U] {
        match self {
            .Some(value) => .Some(transform(value)),
            .None => .None
        }
    }

    /// Transforms the contained value, flattening the result.
    ///
    /// Use when your transform function returns an Optional.
    /// Equivalent to `map` followed by `flatten`.
    public func flatMap[U](transform: (T) -> Optional[U]) -> Optional[U] {
        match self {
            .Some(value) => transform(value),
            .None => .None
        }
    }

    /// Returns Some if predicate returns true, otherwise None.
    public func filter(predicate: (T) -> Bool) -> Optional[T] {
        match self {
            .Some(value) => {
                if predicate(value) {
                    .Some(value)
                } else {
                    .None
                }
            },
            .None => .None
        }
    }

    // ========================================================================
    // COMBINATORS
    // ========================================================================

    /// Returns other if this is Some, otherwise None.
    /// Note: 'and' is a keyword, so we use 'andValue'.
    public func andValue[U](other: Optional[U]) -> Optional[U] {
        match self {
            .Some(_) => other,
            .None => .None
        }
    }

    /// Alias for flatMap - chains optional operations.
    public func andThen[U](transform: (T) -> Optional[U]) -> Optional[U] {
        match self {
            .Some(value) => transform(value),
            .None => .None
        }
    }

    /// Returns this if Some, otherwise returns other.
    /// Note: 'or' is a keyword, so we use 'orValue'.
    public func orValue(other: Optional[T]) -> Optional[T] {
        match self {
            .Some(value) => .Some(value),
            .None => other
        }
    }

    /// Returns this if Some, otherwise calls alternative.
    public func orElse(alternative: () -> Optional[T]) -> Optional[T] {
        match self {
            .Some(value) => .Some(value),
            .None => alternative()
        }
    }

    /// Returns Some if exactly one of this or other is Some.
    public func xor(other: Optional[T]) -> Optional[T] {
        match (self, other) {
            (.Some(value), .None) => .Some(value),
            (.None, .Some(value)) => .Some(value),
            _ => .None
        }
    }

    // ========================================================================
    // MUTATING OPERATIONS
    // ========================================================================

    /// Takes the value out, leaving None in its place.
    ///
    /// Returns the original value (Some or None).
    public mutating func take() -> Optional[T] {
        let result = self;
        self = .None;
        result
    }

    /// Replaces the value, returning the old value.
    public mutating func replace(value: T) -> Optional[T] {
        let old = self;
        self = .Some(value);
        old
    }

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns an iterator over the contained value (0 or 1 elements).
    public func iter() -> OptionalIterator[T] {
        OptionalIterator(self)
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - EQUATABLE
// ============================================================================

/// Extension for Optionals with equatable values.
extend Optional[T]: Equatable where T: Equatable {
    /// Compares two Optionals for equality.
    ///
    /// Two Optionals are equal if both are None, or both are Some with equal values.
    public func equals(other: Optional[T]) -> Bool {
        match (self, other) {
            (.Some(a), .Some(b)) => a == b,
            (.None, .None) => true,
            _ => false
        }
    }
}

// ============================================================================
// PROTOCOL CONFORMANCES
// ============================================================================

/// Tryable extension enabling `try` on Optional values.
///
/// When used with the `try` operator, extracts the value from Some or
/// causes early return with None.
extend Optional[T]: Tryable {
    type Output = T
    type Early = ()

    public func tryExtract() -> ControlFlow[T, ()] {
        match self {
            .Some(value) => .Continue(value),
            .None => .Break(())
        }
    }
}

/// FromResidual extension enabling early return propagation.
extend Optional[T]: FromResidual[()] {
    public static func fromResidual(residual: ()) -> Optional[T] {
        .None
    }
}

/// FromValue extension enabling value promotion.
/// Allows: let x: Int? = 5
extend Optional[T]: FromValue[T] {
    public static func from(value: T) -> Optional[T] {
        .Some(value)
    }
}

/// Formattable extension when T is Formattable.
extend Optional[T]: Formattable where T: Formattable {
    /// Formats this optional as "Some(value)" or "None".
    public func format() -> String {
        match self {
            .Some(value) => "Some(" + value.format() + ")",
            .None => "None"
        }
    }
}

/// ExpressibleByNullLiteral - allows `null` to create Optional.None.
extend Optional[T]: ExpressibleByNullLiteral {
    public init() {
        self = .None
    }
}

/// Coalesce extension enabling the ?? operator.
extend Optional[T]: Coalesce[T] {
    type Coalesce.Output = T

    /// Returns the contained value or evaluates the default.
    ///
    /// The default expression is only evaluated if this is None.
    public func coalesce(default: () -> T) -> T {
        match self {
            .Some(value) => value,
            .None => default()
        }
    }
}

// ============================================================================
// OPTIONAL ITERATOR
// ============================================================================

/// Iterator for Optional that yields 0 or 1 elements.
///
/// Obtained by calling `iter()` on an Optional.
public struct OptionalIterator[T] {
    type Item = T

    private var value: Optional[T]

    public init(value: Optional[T]) {
        self.value = value;
    }

    /// Returns the next element, or None if exhausted.
    public mutating func next() -> Optional[T] {
        let result = self.value;
        self.value = .None;
        result
    }
}

// ============================================================================
// TYPE OPERATOR
// ============================================================================

/// Type operator alias: T? desugars to Optional[T].
@builtin(.OptionalTypeOperator)
public type OptionalTypeOperator[T] = Optional[T];
