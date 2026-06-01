// Result[T, E] - represents either success (Ok) or failure (Err)

module std.result

import std.core.(Equatable, Bool, ControlFlow, Tryable, FromResidual, FromValue, fatalError)
import std.text.(String, StringBuilder, Formattable, FormatOptions)
import std.result.(Optional)

/// The fallible-operation enum: either `Ok(value)` or `Err(error)`. The
/// project's exception-free error story.
///
/// `T throws E` desugars to `Result[T, E]`, and the `try` operator
/// short-circuits on `Err` so failure propagation reads like normal
/// straight-line code. The compiler refuses to let you read the success
/// value without first handling the error case.
///
/// `Result` composes with `Optional` via `ok()` / `err()`, and with the
/// `?` operator via `Tryable`. Pick `Result` when callers should be able
/// to inspect *why* something failed; pick `Optional` when "absent" is the
/// only failure mode.
///
/// # Examples
///
/// ```
/// func parseAndDouble(s: String) -> Int64 throws ParseError {
///     let n = try Int64.parse(s).okOr(ParseError());
///     n * 2
/// }
///
/// match parseAndDouble("21") {
///     .Ok(let v)  => print("got \{v}"),
///     .Err(let e) => print("failed: \{e}")
/// }
/// ```
///
/// # Representation
///
/// A two-case tagged union — discriminant plus the larger of `T` / `E`.
/// Niche optimisation applies the same way it does to `Optional`.
public enum Result[T, E]: Tryable, not Copyable {
    /// The success branch — wraps a `T`.
    case Ok(T)

    /// The failure branch — wraps an `E`.
    case Err(E)

    // Tryable - associated types
    type Output = T
    type Residual = E

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Wraps `value` in `.Ok`. Rarely needed in practice — `FromValue`
    /// promotes bare values where the context expects a `Result`.
    public static func ok(value: T) -> Result[T, E] {
        .Ok(value)
    }

    /// Wraps `error` in `.Err`. Useful when constructing a `Result` from
    /// a known error in non-promotion contexts.
    public static func err(error: E) -> Result[T, E] {
        .Err(error)
    }

    // ========================================================================
    // QUERY METHODS
    // ========================================================================

    /// True when this is `.Ok`. Cheap discriminator-only check.
    ///
    /// # Examples
    ///
    /// ```
    /// Ok(42).isOk();          // true
    /// Err("oops").isOk();     // false
    /// ```
    public func isOk() -> Bool {
        match self {
            .Ok(_) => true,
            .Err(_) => false
        }
    }

    /// True when this is `.Err`. Complement of `isOk`.
    public func isErr() -> Bool {
        match self {
            .Ok(_) => false,
            .Err(_) => true
        }
    }

    // ========================================================================
    // PROTOCOL CONFORMANCES (inline)
    // ========================================================================

    /// Drives `try` — `Continue(value)` for `.Ok`, `Break(error)` for
    /// `.Err`. Defined inline because `Tryable` is declared in the enum's
    /// conformance list above.
    public consuming func tryExtract() -> ControlFlow[T, E] {
        match self {
            .Ok(value) => .Continue(value),
            .Err(error) => .Break(error)
        }
    }

    // ========================================================================
    // UNWRAPPING - SUCCESS VALUE
    // ========================================================================

    /// Returns the success value, panicking if `Err`. Use
    /// `unwrap(or:)` or pattern matching unless you can prove the
    /// result is `Ok`.
    ///
    /// # Errors
    ///
    /// Panics with `"called unwrap() on Err"` when invoked on `.Err`.
    public func unwrap() -> T {
        match self {
            .Ok(value) => value,
            .Err(_) => fatalError("called unwrap() on Err")
        }
    }

    /// Returns the success value or `default` on `Err`. `default` is
    /// always evaluated — use `unwrap(orElse:)` if computing it is
    /// expensive or depends on the error.
    public func unwrap(or default: T) -> T {
        match self {
            .Ok(value) => value,
            .Err(_) => default
        }
    }

    /// Like `unwrap(or:)`, but `defaultFn` receives the error value and is
    /// only invoked on `Err`. Useful when the recovery value depends on
    /// what went wrong.
    public func unwrap(orElse defaultFn: (E) -> T) -> T {
        match self {
            .Ok(value) => value,
            .Err(error) => defaultFn(error)
        }
    }

    // ========================================================================
    // UNWRAPPING - ERROR VALUE
    // ========================================================================

    /// Returns the error value, panicking if `Ok`. Mostly used in tests
    /// to assert that a call failed.
    ///
    /// # Errors
    ///
    /// Panics with `"called unwrapErr() on Ok"` when invoked on `.Ok`.
    public func unwrapErr() -> E {
        match self {
            .Ok(_) => fatalError("called unwrapErr() on Ok"),
            .Err(error) => error
        }
    }

    // ========================================================================
    // TRANSFORMATIONS - SUCCESS VALUE
    // ========================================================================

    /// Functor map on the success branch. `.Err` passes through unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// Ok(2).map { it * 2 };          // Ok(4)
    /// Err("oops").map { it * 2 };    // Err("oops")
    /// ```
    public func map[U](transform: (T) -> U) -> Result[U, E] {
        match self {
            .Ok(value) => .Ok(transform(value)),
            .Err(error) => .Err(error)
        }
    }

    /// Monadic bind on the success branch — apply a transform that itself
    /// returns a `Result`, without nesting.
    public func flatMap[U](transform: (T) -> Result[U, E]) -> Result[U, E] {
        match self {
            .Ok(value) => transform(value),
            .Err(error) => .Err(error)
        }
    }

    // ========================================================================
    // TRANSFORMATIONS - ERROR VALUE
    // ========================================================================

    /// Functor map on the error branch — typically used to widen a
    /// specific error type into a more general one.
    ///
    /// # Examples
    ///
    /// ```
    /// parse(s).mapErr { AppError.Parse(it) };
    /// ```
    public func mapErr[F](transform: (E) -> F) -> Result[T, F] {
        match self {
            .Ok(value) => .Ok(value),
            .Err(error) => .Err(transform(error))
        }
    }

    /// Monadic bind on the error branch — apply a recovery function that
    /// itself returns a `Result`, without nesting. Mirror of `flatMap`.
    public func flatMapErr[F](transform: (E) -> Result[T, F]) -> Result[T, F] {
        match self {
            .Ok(value) => .Ok(value),
            .Err(error) => transform(error)
        }
    }

    // ========================================================================
    // CONVERSION TO OPTIONAL
    // ========================================================================

    /// Discards the error, returning `Some(value)` for `.Ok` and `None`
    /// for `.Err`.
    public func ok() -> Optional[T] {
        match self {
            .Ok(value) => .Some(value),
            .Err(_) => .None
        }
    }

    /// Discards the success value, returning `Some(error)` for `.Err` and
    /// `None` for `.Ok`. Mirror of `ok()`.
    public func err() -> Optional[E] {
        match self {
            .Ok(_) => .None,
            .Err(error) => .Some(error)
        }
    }

    // ========================================================================
    // COMBINATORS
    // ========================================================================

    /// Returns `other` when `self` is `Ok`, otherwise propagates the
    /// existing `Err`. Named `andValue` (not `and`) because `and` is a
    /// reserved keyword.
    public func andValue[U](other: Result[U, E]) -> Result[U, E] {
        match self {
            .Ok(_) => other,
            .Err(error) => .Err(error)
        }
    }

    /// Alias for `flatMap` — chains a fallible step onto an `Ok` branch.
    /// Reads more naturally in long pipelines (`parseInput().andThen(validate).andThen(persist)`).
    public func andThen[U](transform: (T) -> Result[U, E]) -> Result[U, E] {
        match self {
            .Ok(value) => transform(value),
            .Err(error) => .Err(error)
        }
    }

    /// Returns `self` when `Ok`, otherwise returns `other`. Named
    /// `orValue` because `or` is a reserved keyword.
    public func orValue(other: Result[T, E]) -> Result[T, E] {
        match self {
            .Ok(value) => .Ok(value),
            .Err(_) => other
        }
    }

    /// Returns `self` when `Ok`, otherwise calls `alternative(error)`.
    /// Use this for recovery logic that depends on which error occurred —
    /// e.g. retrying on a transient error but bubbling a permanent one.
    public func orElse[F](alternative: (E) -> Result[T, F]) -> Result[T, F] {
        match self {
            .Ok(value) => .Ok(value),
            .Err(error) => alternative(error)
        }
    }

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns a `ResultIterator` yielding the success value (one element
    /// for `.Ok`, zero for `.Err`). Lets a `Result` plug into iterator
    /// pipelines that only care about the happy path.
    public func iter() -> ResultIterator[T, E] {
        ResultIterator(self)
    }
}

// ============================================================================
// PROTOCOL CONFORMANCES
// ============================================================================

/// `Result` is move-only by default (`not Copyable`) so it can carry a
/// non-Copyable payload (e.g. `Result[File, E]`). It regains bit-copy
/// semantics only when *both* payloads are themselves Copyable — so
/// `Result[Int64, Error]` is Copyable while `Result[Array[Int64], E]` stays
/// move-only.
extend Result[T, E]: Copyable where T: Copyable, E: Copyable { }

/// `FromResidual[E]` — converts a `try`-propagated error back into
/// `.Err`, so chains of `try` returning results compose.
extend Result[T, E]: FromResidual[E] {
    /// Builds `.Err(residual)` from the residual produced by a `try`
    /// short-circuit.
    public static func fromResidual(residual: E) -> Result[T, E] {
        .Err(residual)
    }
}

/// `FromValue[T]` — promotes a bare `T` to `Result[T, E]`. Lets you
/// write `let r: Int throws Error = 42` without explicit `.Ok`.
extend Result[T, E]: FromValue[T] {
    /// Wraps `value` in `.Ok`. Called by the compiler at the promotion
    /// site, not usually by user code.
    public static func from(value: T) -> Result[T, E] {
        .Ok(value)
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - EQUATABLE
// ============================================================================

/// Equatable when both `T` and `E` are. Cross-discriminant comparisons
/// (`Ok` vs `Err`) are always unequal.
extend Result[T, E]: Equatable where T: Equatable, E: Equatable {
    /// Structural equality on the result. Backs `==`.
    ///
    /// # Examples
    ///
    /// ```
    /// Ok(1)       == Ok(1);        // true
    /// Ok(1)       == Ok(2);        // false
    /// Err("x")    == Err("x");     // true
    /// Ok(1)       == Err("x");     // false
    /// ```
    public func isEqual(to other: Result[T, E]) -> Bool {
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

/// Formattable when both `T` and `E` are. Renders as `Ok(value)` or
/// `Err(error)`, forwarding `options` to the inner formatter.
extend Result[T, E]: Formattable where T: Formattable, E: Formattable {
    /// Renders `Ok(...)` or `Err(...)`, forwarding `options` to the inner
    /// `format` for the payload.
    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        match self {
            .Ok(value) => {
                writer.append("Ok(");
                value.format(into: writer, options);
                writer.append(char: ')')
            },
            .Err(error) => {
                writer.append("Err(");
                error.format(into: writer, options);
                writer.append(char: ')')
            }
        }
    }
}

// ============================================================================
// RESULT ITERATOR
// ============================================================================

/// Single-shot iterator yielding zero or one elements (the `Ok` value).
/// Returned by `Result.iter()`. Errors are silently skipped — use
/// `mapErr` / `match` if you need them.
///
/// # Representation
///
/// Stores the success value in an `Optional[T]` field; `next()` empties
/// it on first call.
public struct ResultIterator[T, E] {
    type Item = T

    private var value: Optional[T]

    /// @name From Result
    /// Builds an iterator from a `Result`, projecting `.Ok` to a single
    /// element and `.Err` to an empty stream.
    public init(result: Result[T, E]) {
        self.value = result.ok();
    }

    /// Returns and clears the stored value, then returns `None` forever.
    /// `O(1)` and allocation-free.
    public mutating func next() -> Optional[T] {
        let result = self.value;
        self.value = .None;
        result
    }
}

// ============================================================================
// TYPE OPERATOR
// ============================================================================

/// Compiler hook — `T throws E` desugars to `Result[T, E]` via this
/// alias. Write the sugar in user code; this exists so the operator can
/// resolve to a concrete type.
@builtin(.ResultTypeOperator)
public type ResultTypeOperator[T, E] = Result[T, E];
