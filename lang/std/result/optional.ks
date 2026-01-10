// Optional type

module std.result

import std.core.(Equatable)
import std.ops.(ExpressibleByNilLiteral, Nil)
import std.iter.(Iterator, Functor)

public enum Optional[T]: ExpressibleByNilLiteral {
    case Some(T)
    case None

    // ExpressibleByNilLiteral
    public init(nilLiteral value: Nil) {
        self = .None
    }

    // Convenience constructors
    public static func some(value: T) -> Optional[T] {
        .Some(value)
    }

    public static func none() -> Optional[T] {
        .None
    }

    // Properties
    public var isSome: Bool {
        match self {
            .Some(_) => true,
            .None => false
        }
    }

    public var isNone: Bool {
        match self {
            .Some(_) => false,
            .None => true
        }
    }

    // Unwrapping
    public func unwrap() -> T {
        match self {
            .Some(value) => value,
            .None => panic("called unwrap() on None")
        }
    }

    public func unwrapOr(default: T) -> T {
        match self {
            .Some(value) => value,
            .None => default
        }
    }

    public func unwrap(orElse defaultFn: () -> T) -> T {
        match self {
            .Some(value) => value,
            .None => defaultFn()
        }
    }

    // expect with custom message
    public func expect(message: String) -> T {
        match self {
            .Some(value) => value,
            .None => panic(message)
        }
    }

    // Transformations
    public func map[U](transform: (T) -> U) -> Optional[U] {
        match self {
            .Some(value) => .Some(transform(value)),
            .None => .None
        }
    }

    public func flatMap[U](transform: (T) -> Optional[U]) -> Optional[U] {
        match self {
            .Some(value) => transform(value),
            .None => .None
        }
    }

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

    // Convert to Result
    public func ok[E](otherwise error: E) -> Result[T, E] where E: Error {
        match self {
            .Some(value) => .Ok(value),
            .None => .Err(error)
        }
    }

    public func okOrElse[E](errorFn: () -> E) -> Result[T, E] where E: Error {
        match self {
            .Some(value) => .Ok(value),
            .None => .Err(errorFn())
        }
    }

    // Combinator operations
    // Note: 'and'/'or' are keywords, so we use 'andValue'/'orValue'
    public func andValue[U](other: Optional[U]) -> Optional[U] {
        match self {
            .Some(_) => other,
            .None => .None
        }
    }

    public func andThen[U](transform: (T) -> Optional[U]) -> Optional[U] {
        self.flatMap(transform)
    }

    public func orValue(other: Optional[T]) -> Optional[T] {
        match self {
            .Some(value) => .Some(value),
            .None => other
        }
    }

    public func orElse(alternative: () -> Optional[T]) -> Optional[T] {
        match self {
            .Some(value) => .Some(value),
            .None => alternative()
        }
    }

    public func xor(other: Optional[T]) -> Optional[T] {
        match (self, other) {
            (.Some(value), .None) => .Some(value),
            (.None, .Some(value)) => .Some(value),
            _ => .None
        }
    }

    // Take and replace
    public func take() -> Optional[T] {
        let result = self;
        self = .None;
        result
    }

    public func replace(value: T) -> Optional[T] {
        let old = self;
        self = .Some(value);
        old
    }

    // Iteration
    public func iter() -> OptionalIterator[T] {
        OptionalIterator(value: self)
    }
}

// Equatable when T is Equatable
extend Optional[T]: Equatable where T: Equatable {
    public func equals(other: Optional[T]) -> Bool {
        match (self, other) {
            (.Some(a), .Some(b)) => a == b,
            (.None, .None) => true,
            _ => false
        }
    }
}

// Functor implementation
extend Optional[T]: Functor {
    type Inner = T
}

// Tryable for Optional (try? semantics)
extend Optional[T]: Tryable[T, Nil] {
    public func tryExtract() -> Residual[T, Nil] {
        match self {
            .Some(value) => .Output(value),
            .None => .Early(nil)
        }
    }
}

// Optional iterator
public struct OptionalIterator[T]: Iterator {
    type Item = T

    private var value: Optional[T]

    public init(value: Optional[T]) {
        self.value = value;
    }

    public func next() -> Optional[T] {
        let result = self.value;
        self.value = .None;
        result
    }
}

// Type alias for sugar: T? is Optional[T]
// This is handled by the compiler
