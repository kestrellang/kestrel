// Result[T, E] - represents either success (Ok) or failure (Err)

module std.result

import std.core.(Equatable, Formattable, Bool, ControlFlow, Tryable, FromResidual, FromValue)
import std.text.(String)
import std.result.(Optional)

/// Represents the result of an operation: either `Ok(value)` or `Err(error)`.
///
/// Used for operations that can fail, providing explicit error handling
/// without exceptions. The compiler enforces handling of the error case.
///
/// Syntactic sugar:
///     T throws E   desugars to  Result[T, E]
public enum Result[T, E]: Tryable {
    /// Contains a success value.
    case Ok(T)

    /// Contains an error value.
    case Err(E)

    // Tryable - associated types
    type Output = T
    type Early = E

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Creates an Ok containing the value.
    public static func ok(value: T) -> Result[T, E] {
        .Ok(value)
    }

    /// Creates an Err containing the error.
    public static func err(error: E) -> Result[T, E] {
        .Err(error)
    }

    // ========================================================================
    // QUERY METHODS
    // ========================================================================

    /// Returns true if this is Ok.
    public func isOk() -> Bool {
        match self {
            .Ok(_) => true,
            .Err(_) => false
        }
    }

    /// Returns true if this is Err.
    public func isErr() -> Bool {
        match self {
            .Ok(_) => false,
            .Err(_) => true
        }
    }

    // ========================================================================
    // PROTOCOL CONFORMANCES (inline)
    // ========================================================================

    /// Extracts the value for the try operator.
    public func tryExtract() -> ControlFlow[T, E] {
        match self {
            .Ok(value) => .Continue(value),
            .Err(error) => .Break(error)
        }
    }

    // ========================================================================
    // UNWRAPPING - SUCCESS VALUE
    // ========================================================================

    /// Returns the success value, panicking if Err.
    ///
    /// WARNING: Only use when you are certain the result is Ok.
    /// Prefer `unwrapOr`, `unwrap(orElse:)`, or pattern matching.
    public func unwrap() -> T {
        match self {
            .Ok(value) => value,
            .Err(_) => lang.panic("called unwrap() on Err")
        }
    }

    /// Returns the success value or the default if Err.
    ///
    /// Note: The default is eagerly evaluated. Use `unwrap(orElse:)` for
    /// lazy evaluation when the default is expensive to compute.
    public func unwrapOr(default: T) -> T {
        match self {
            .Ok(value) => value,
            .Err(_) => default
        }
    }

    /// Returns the success value or calls the function with the error if Err.
    ///
    /// The function receives the error and can use it to compute a default.
    public func unwrap(orElse defaultFn: (E) -> T) -> T {
        match self {
            .Ok(value) => value,
            .Err(error) => defaultFn(error)
        }
    }

    // ========================================================================
    // UNWRAPPING - ERROR VALUE
    // ========================================================================

    /// Returns the error value, panicking if Ok.
    ///
    /// Useful in tests to assert that an operation failed.
    public func unwrapErr() -> E {
        match self {
            .Ok(_) => lang.panic("called unwrapErr() on Ok"),
            .Err(error) => error
        }
    }

    // ========================================================================
    // TRANSFORMATIONS - SUCCESS VALUE
    // ========================================================================

    /// Transforms the success value using the function.
    ///
    /// Returns Err unchanged if this is Err.
    public func map[U](transform: (T) -> U) -> Result[U, E] {
        match self {
            .Ok(value) => .Ok(transform(value)),
            .Err(error) => .Err(error)
        }
    }

    /// Transforms the success value, flattening the result.
    ///
    /// Use when your transform function returns a Result.
    public func flatMap[U](transform: (T) -> Result[U, E]) -> Result[U, E] {
        match self {
            .Ok(value) => transform(value),
            .Err(error) => .Err(error)
        }
    }

    // ========================================================================
    // TRANSFORMATIONS - ERROR VALUE
    // ========================================================================

    /// Transforms the error value using the function.
    ///
    /// Returns Ok unchanged if this is Ok.
    public func mapErr[F](transform: (E) -> F) -> Result[T, F] {
        match self {
            .Ok(value) => .Ok(value),
            .Err(error) => .Err(transform(error))
        }
    }

    /// Transforms the error value, flattening the result.
    ///
    /// Use when your transform function returns a Result.
    public func flatMapErr[F](transform: (E) -> Result[T, F]) -> Result[T, F] {
        match self {
            .Ok(value) => .Ok(value),
            .Err(error) => transform(error)
        }
    }

    // ========================================================================
    // CONVERSION TO OPTIONAL
    // ========================================================================

    /// Converts to Optional, discarding the error.
    ///
    /// Returns Some(value) if Ok, None if Err.
    public func ok() -> Optional[T] {
        match self {
            .Ok(value) => .Some(value),
            .Err(_) => .None
        }
    }

    /// Converts to Optional, discarding the success value.
    ///
    /// Returns Some(error) if Err, None if Ok.
    public func err() -> Optional[E] {
        match self {
            .Ok(_) => .None,
            .Err(error) => .Some(error)
        }
    }

    // ========================================================================
    // COMBINATORS
    // ========================================================================

    /// Returns other if this is Ok, otherwise returns the Err.
    /// Note: 'and' is a keyword, so we use 'andValue'.
    public func andValue[U](other: Result[U, E]) -> Result[U, E] {
        match self {
            .Ok(_) => other,
            .Err(error) => .Err(error)
        }
    }

    /// Alias for flatMap - chains result operations.
    public func andThen[U](transform: (T) -> Result[U, E]) -> Result[U, E] {
        match self {
            .Ok(value) => transform(value),
            .Err(error) => .Err(error)
        }
    }

    /// Returns this if Ok, otherwise returns other.
    /// Note: 'or' is a keyword, so we use 'orValue'.
    public func orValue(other: Result[T, E]) -> Result[T, E] {
        match self {
            .Ok(value) => .Ok(value),
            .Err(_) => other
        }
    }

    /// Returns this if Ok, otherwise calls alternative with the error.
    ///
    /// The function receives the error and can attempt recovery.
    public func orElse[F](alternative: (E) -> Result[T, F]) -> Result[T, F] {
        match self {
            .Ok(value) => .Ok(value),
            .Err(error) => alternative(error)
        }
    }

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns an iterator over the success value (0 or 1 elements).
    public func iter() -> ResultIterator[T, E] {
        ResultIterator(self)
    }
}

// ============================================================================
// PROTOCOL CONFORMANCES
// ============================================================================

/// FromResidual extension enabling early return propagation.
extend Result[T, E]: FromResidual[E] {
    public static func fromResidual(residual: E) -> Result[T, E] {
        .Err(residual)
    }
}

/// FromValue extension enabling value promotion.
/// Allows: let r: Int throws Error = 42
extend Result[T, E]: FromValue[T] {
    public static func from(value: T) -> Result[T, E] {
        .Ok(value)
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - EQUATABLE
// ============================================================================

/// Extension for Results with equatable values and errors.
extend Result[T, E]: Equatable where T: Equatable, E: Equatable {
    /// Compares two Results for equality.
    ///
    /// Two Results are equal if both are Ok with equal values, or both
    /// are Err with equal errors.
    public func equals(other: Result[T, E]) -> Bool {
        match (self, other) {
            (.Ok(a), .Ok(b)) => a == b,
            (.Err(a), .Err(b)) => a == b,
            _ => false
        }
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - FORMATTABLE
// ============================================================================

/// Formattable extension when T and E are Formattable.
extend Result[T, E]: Formattable where T: Formattable, E: Formattable {
    /// Formats this result as "Ok(value)" or "Err(error)".
    public func format() -> String {
        match self {
            .Ok(value) => "Ok(" + value.format() + ")",
            .Err(error) => "Err(" + error.format() + ")"
        }
    }
}

// ============================================================================
// RESULT ITERATOR
// ============================================================================

/// Iterator for Result that yields 0 or 1 elements (only Ok values).
///
/// Obtained by calling `iter()` on a Result.
public struct ResultIterator[T, E] {
    type Item = T

    private var value: Optional[T]

    public init(result: Result[T, E]) {
        self.value = result.ok();
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

/// Type operator alias: `T throws E` desugars to `Result[T, E]`.
@builtin(.ResultTypeOperator)
public type ResultTypeOperator[T, E] = Result[T, E];
