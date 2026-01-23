// Result type - represents either success (Ok) or failure (Err)

module std.result

import std.core.(Equatable, Formattable, Bool, ControlFlow, Tryable, FromResidual, Returnable)
import std.text.(String)
import std.result.(Optional)

public enum Result[T, E]: Tryable, Returnable[T] {
    case Ok(T)
    case Err(E)

    // Tryable - associated types
    type Output = T
    type Early = E

    // Convenience constructors
    public static func ok(value: T) -> Result[T, E] {
        .Ok(value)
    }

    public static func err(error: E) -> Result[T, E] {
        .Err(error)
    }

    // Properties - using functions due to computed property parsing issues in enums
    public func isOk() -> Bool {
        match self {
            .Ok(_) => true,
            .Err(_) => false
        }
    }

    public func isErr() -> Bool {
        match self {
            .Ok(_) => false,
            .Err(_) => true
        }
    }

    // Tryable - enables `try`
    public func tryExtract() -> ControlFlow[T, E] {
        match self {
            .Ok(value) => .Continue(value),
            .Err(error) => .Break(error)
        }
    }

    // Returnable - enables `return value`
    public static func fromOutput(value: T) -> Result[T, E] {
        .Ok(value)
    }

    // Unwrapping
    public func unwrap() -> T {
        match self {
            .Ok(value) => value,
            .Err(_) => lang.panic("called unwrap() on Err")
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
            .Ok(_) => lang.panic("called unwrapErr() on Ok"),
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

    public func mapErr[F](transform: (E) -> F) -> Result[T, F] {
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

    public func flatMapErr[F](transform: (E) -> Result[T, F]) -> Result[T, F] {
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
        match self {
            .Ok(value) => transform(value),
            .Err(error) => .Err(error)
        }
    }

    public func orValue(other: Result[T, E]) -> Result[T, E] {
        match self {
            .Ok(value) => .Ok(value),
            .Err(_) => other
        }
    }

    public func orElse[F](alternative: (E) -> Result[T, F]) -> Result[T, F] {
        match self {
            .Ok(value) => .Ok(value),
            .Err(error) => alternative(error)
        }
    }

    // Iteration
    public func iter() -> ResultIterator[T, E] {
        ResultIterator(self)
    }
}

// FromResidual conformance - enables early return propagation
extend Result[T, E]: FromResidual[E] {
    public static func fromResidual(residual: E) -> Result[T, E] {
        .Err(residual)
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

// Formattable when T and E are Formattable
extend Result[T, E]: Formattable where T: Formattable, E: Formattable {
    public func format() -> String {
        match self {
            .Ok(value) => "Ok(" + value.format() + ")",
            .Err(error) => "Err(" + error.format() + ")"
        }
    }
}

// Result iterator - iterates 0 or 1 times (only Ok values)
public struct ResultIterator[T, E] {
    type Item = T

    private var value: Optional[T]

    public init(result: Result[T, E]) {
        self.value = result.ok();
    }

    public mutating func next() -> Optional[T] {
        let result = self.value;
        self.value = .None;
        result
    }
}

// Type operator alias: T throws E desugars to ResultTypeOperator[T, E] which is Result[T, E]
@builtin(.ResultTypeOperator)
public type ResultTypeOperator[T, E] = Result[T, E];
