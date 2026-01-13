// Result type

module std.result

import std.core.(Equatable)
import std.iter.(Iterator, Functor)

public enum Result[T, E]:
    Tryable[T, E],
    Throwable[E],
    Returnable[T]
{
    case Ok(T)
    case Err(E)

    // Convenience constructors
    public static func ok(value: T) -> Result[T, E] {
        .Ok(value)
    }

    public static func err(error: E) -> Result[T, E] {
        .Err(error)
    }

    // Properties
    public var isOk: Bool {
        match self {
            .Ok(_) => true,
            .Err(_) => false
        }
    }

    public var isErr: Bool {
        match self {
            .Ok(_) => false,
            .Err(_) => true
        }
    }

    // Tryable - enables `try`
    public func tryExtract() -> Residual[T, E] {
        match self {
            .Ok(value) => .Output(value),
            .Err(error) => .Early(error)
        }
    }

    // Throwable - enables `throw`
    public static func fromEarly(value: E) -> Result[T, E] {
        .Err(value)
    }

    // Returnable - enables `return value`
    public static func fromOutput(value: T) -> Result[T, E] {
        .Ok(value)
    }

    // Unwrapping
    public func unwrap() -> T {
        match self {
            .Ok(value) => value,
            .Err(error) => panic("called unwrap() on Err: " + error.description())
        }
    }

    public func unwrapOr(default: T) -> T {
        match self {
            .Ok(value) => value,
            .Err(_) => default
        }
    }

    public func unwrap(orElse defaultFn: (E) -> T) -> T {
        match self {
            .Ok(value) => value,
            .Err(error) => defaultFn(error)
        }
    }

    public func unwrapErr() -> E {
        match self {
            .Ok(value) => panic("called unwrapErr() on Ok"),
            .Err(error) => error
        }
    }

    // expect with custom message
    public func expect(message: String) -> T {
        match self {
            .Ok(value) => value,
            .Err(error) => panic(message + ": " + error.description())
        }
    }

    public func expectErr(message: String) -> E {
        match self {
            .Ok(_) => lang.panic(message),
            .Err(error) => error
        }
    }

    // Transformations
    public func map[U](transform: (T) -> U) -> Result[U, E] {
        match self {
            .Ok(value) => .Ok(transform(value)),
            .Err(error) => .Err(error)
        }
    }

    public func mapErr[F](transform: (E) -> F) -> Result[T, F] where F: Error {
        match self {
            .Ok(value) => .Ok(value),
            .Err(error) => .Err(transform(error))
        }
    }

    public func flatMap[U](transform: (T) -> Result[U, E]) -> Result[U, E] {
        match self {
            .Ok(value) => transform(value),
            .Err(error) => .Err(error)
        }
    }

    public func flatMapErr[F](transform: (E) -> Result[T, F]) -> Result[T, F] where F: Error {
        match self {
            .Ok(value) => .Ok(value),
            .Err(error) => transform(error)
        }
    }

    // Convert to Optional
    public func ok() -> Optional[T] {
        match self {
            .Ok(value) => .Some(value),
            .Err(_) => .None
        }
    }

    public func err() -> Optional[E] {
        match self {
            .Ok(_) => .None,
            .Err(error) => .Some(error)
        }
    }

    // Combinator operations
    // Note: 'and'/'or' are keywords, so we use 'andValue'/'orValue'
    public func andValue[U](other: Result[U, E]) -> Result[U, E] {
        match self {
            .Ok(_) => other,
            .Err(error) => .Err(error)
        }
    }

    public func andThen[U](transform: (T) -> Result[U, E]) -> Result[U, E] {
        self.flatMap(transform)
    }

    public func orValue(other: Result[T, E]) -> Result[T, E] {
        match self {
            .Ok(value) => .Ok(value),
            .Err(_) => other
        }
    }

    public func orElse[F](alternative: (E) -> Result[T, F]) -> Result[T, F] where F: Error {
        match self {
            .Ok(value) => .Ok(value),
            .Err(error) => alternative(error)
        }
    }

    // Transpose Optional inside Result
    public func transpose() -> Optional[Result[T, E]] where T: Optional {
        match self {
            .Ok(.Some(value)) => .Some(.Ok(value)),
            .Ok(.None) => .None,
            .Err(error) => .Some(.Err(error))
        }
    }

    // Iteration
    public func iter() -> ResultIterator[T, E] {
        ResultIterator(value: self)
    }
}

// Equatable when T and E are Equatable
extend Result[T, E]: Equatable where T: Equatable, E: Equatable {
    public func equals(other: Result[T, E]) -> Bool {
        match (self, other) {
            (.Ok(a), .Ok(b)) => a == b,
            (.Err(a), .Err(b)) => a == b,
            _ => false
        }
    }
}

// Functor implementation
extend Result[T, E]: Functor {
    type Inner = T
}

// Result iterator
public struct ResultIterator[T, E]: Iterator {
    type Item = T

    private var value: Optional[T]

    public init(value: Result[T, E]) {
        self.value = value.ok()
    }

    public mutating func next() -> Optional[T] {
        let result = self.value;
        self.value = .None;
        result
    }
}
