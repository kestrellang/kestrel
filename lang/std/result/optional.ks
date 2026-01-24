// Optional type - represents a value that may or may not be present

module std.result

import std.core.(Equatable, Formattable, Bool, ControlFlow, Tryable, FromResidual, ExpressibleByNullLiteral, Coalesce)
import std.text.(String)
// Note: Iterator import creates circular dependency - Iterator imports Optional
// import std.iter.(Iterator)

// Optional[T] - either Some(value) or None
@builtin(.OptionalEnum)
public enum Optional[T] {
    @builtin(.OptionalSomeCase)
    case Some(T)
    @builtin(.OptionalNoneCase)
    case None

    // Convenience constructors
    public static func some(value: T) -> Optional[T] {
        .Some(value)
    }

    public static func none() -> Optional[T] {
        .None
    }

    // Properties - using functions due to computed property parsing issues in enums
    public func isSome() -> Bool {
        match self {
            .Some(_) => true,
            .None => false
        }
    }

    public func isNone() -> Bool {
        match self {
            .Some(_) => false,
            .None => true
        }
    }

    // Unwrapping
    public func unwrap() -> T {
        match self {
            .Some(value) => value,
            .None => lang.panic("called unwrap() on None")
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

    // Combinator operations
    // Note: 'and'/'or' are keywords, so we use 'andValue'/'orValue'
    public func andValue[U](other: Optional[U]) -> Optional[U] {
        match self {
            .Some(_) => other,
            .None => .None
        }
    }

    public func andThen[U](transform: (T) -> Optional[U]) -> Optional[U] {
        match self {
            .Some(value) => transform(value),
            .None => .None
        }
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
    public mutating func take() -> Optional[T] {
        let result = self;
        self = .None;
        result
    }

    public mutating func replace(value: T) -> Optional[T] {
        let old = self;
        self = .Some(value);
        old
    }

    // Iteration
    public func iter() -> OptionalIterator[T] {
        OptionalIterator(self)
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

// Tryable for Optional (try? semantics)
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

// FromResidual for Optional - enables early return propagation
extend Optional[T]: FromResidual[()] {
    public static func fromResidual(residual: ()) -> Optional[T] {
        .None
    }
}

// Formattable when T is Formattable
extend Optional[T]: Formattable where T: Formattable {
    public func format() -> String {
        match self {
            .Some(value) => "Some(" + value.format() + ")",
            .None => "None"
        }
    }
}

// ExpressibleByNullLiteral - allows `null` to create Optional.None
extend Optional[T]: ExpressibleByNullLiteral {
    public init() {
        self = .None
    }
}

// Coalesce for Optional - enables ?? operator to unwrap with default
// Optional[T] ?? T -> T
extend Optional[T]: Coalesce[T] {
    type Coalesce.Output = T

    public func coalesce(default: () -> T) -> T {
        match self {
            .Some(value) => value,
            .None => default()
        }
    }
}

// Optional iterator - iterates 0 or 1 times
// Note: Iterator conformance added via extension in iter/iterator.ks to avoid circular import
public struct OptionalIterator[T] {
    type Item = T

    private var value: Optional[T]

    public init(value: Optional[T]) {
        self.value = value;
    }

    public mutating func next() -> Optional[T] {
        let result = self.value;
        self.value = .None;
        result
    }
}

// Type operator alias: T? desugars to OptionalTypeOperator[T] which is Optional[T]
@builtin(.OptionalTypeOperator)
public type OptionalTypeOperator[T] = Optional[T];
