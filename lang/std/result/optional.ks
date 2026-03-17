// Optional[T] - represents a value that may or may not be present

module std.result

import std.core.(Equatable, Comparable, Ordering, Hash, Hasher, Bool, ControlFlow, Tryable, FromResidual, FromValue, ExpressibleByNullLiteral, Coalesce, fatalError)
import std.text.(String, FormatOptions, Formattable)
import std.result.(Result)
import std.num.(Int64, UInt8)
import std.memory.(Slice, Pointer)
import std.iter.(Iterator)

/// Represents an optional value: either `Some(value)` or `None`.
///
/// Used for values that may be absent, providing a type-safe alternative to
/// null pointers. The compiler enforces handling of the None case.
///
/// Syntactic sugar:
///     T?           desugars to  Optional[T]
///     null         creates      Optional.None
///
/// Example:
///     func find(id: Int64) -> User? {
///         if let user = users.get(id) {
///             return user
///         }
///         return null
///     }
///
///     let user = find(id: 42)
///     match user {
///         case .Some(let u) => print(u.name)
///         case .None => print("Not found")
///     }
///
/// The `try` operator extracts values, returning early on None:
///     func process() -> Result? {
///         let a = try getA()  // returns None if getA() is None
///         let b = try getB()
///         return transform(a, b)
///     }
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
    ///
    /// Typically not needed - just return the value directly and it will
    /// be wrapped automatically.
    ///
    /// Example:
    ///     let opt = Optional.some(value: 42)  // Some(42)
    ///     let opt: Int64? = 42                // Same, preferred
    public static func some(value: T) -> Optional[T] {
        .Some(value)
    }

    /// Creates a None.
    ///
    /// Typically not needed - use `null` literal instead.
    ///
    /// Example:
    ///     let opt = Optional[Int64].none()  // None
    ///     let opt: Int64? = null            // Same, preferred
    public static func none() -> Optional[T] {
        .None
    }

    // ========================================================================
    // QUERY METHODS
    // ========================================================================

    /// Returns true if this is Some.
    ///
    /// Example:
    ///     Some(42).isSome()  // true
    ///     None.isSome()      // false
    public func isSome() -> Bool {
        match self {
            .Some(_) => true,
            .None => false
        }
    }

    /// Returns true if this is None.
    ///
    /// Example:
    ///     Some(42).isNone()  // false
    ///     None.isNone()      // true
    public func isNone() -> Bool {
        match self {
            .Some(_) => false,
            .None => true
        }
    }

    /// Returns true if this is Some and the value satisfies the predicate.
    ///
    /// Example:
    ///     Some(42).isSomeAnd({ it > 0 })   // true
    ///     Some(-1).isSomeAnd({ it > 0 })   // false
    ///     None.isSomeAnd({ it > 0 })       // false
    public func isSomeAnd(predicate: (T) -> Bool) -> Bool {
        match self {
            .Some(value) => predicate(value),
            .None => false
        }
    }

    // ========================================================================
    // UNWRAPPING
    // ========================================================================

    /// Returns the contained value, panicking if None.
    ///
    /// WARNING: Only use when you are certain the value is Some.
    /// Prefer `unwrapOr`, `unwrap(orElse:)`, or pattern matching.
    ///
    /// Example:
    ///     Some(42).unwrap()  // 42
    ///     None.unwrap()      // PANIC: called unwrap on None
    public func unwrap() -> T {
        match self {
            .Some(value) => value,
            .None => lang.panic("called unwrap() on None")
        }
    }

    /// Returns the contained value, panicking with a custom message if None.
    ///
    /// Use this for better error messages when a None is unexpected.
    ///
    /// Example:
    ///     let config = loadConfig().expect(message: "Config file required")
    ///     None.expect(message: "Should never happen")  // PANIC: Should never happen
    public func expect(message: String) -> T {
        match self {
            .Some(value) => value,
            .None => fatalError(message)
        }
    }

    /// Returns the contained value or the default if None.
    ///
    /// Note: The default is eagerly evaluated. Use `unwrap(orElse:)` for
    /// lazy evaluation when the default is expensive to compute.
    ///
    /// Example:
    ///     Some(42).unwrapOr(default: 0)  // 42
    ///     None.unwrapOr(default: 0)      // 0
    public func unwrapOr(default: T) -> T {
        match self {
            .Some(value) => value,
            .None => default
        }
    }

    /// Returns the contained value or calls the function if None.
    ///
    /// The function is only called if the Optional is None.
    ///
    /// Example:
    ///     Some(42).unwrap(orElse: || expensiveDefault())  // 42, no call
    ///     None.unwrap(orElse: || expensiveDefault())      // calls function
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
    ///
    /// Example:
    ///     Some(2).map({ it * 2 })        // Some(4)
    ///     None.map({ it * 2 })           // None
    ///     Some("hello").map({ it.len })  // Some(5)
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
    ///
    /// Example:
    ///     func parse(s: String) -> Int64? { ... }
    ///
    ///     Some("42").flatMap(parse)   // Some(42)
    ///     Some("abc").flatMap(parse)  // None (parse failed)
    ///     None.flatMap(parse)         // None
    public func flatMap[U](transform: (T) -> Optional[U]) -> Optional[U] {
        match self {
            .Some(value) => transform(value),
            .None => .None
        }
    }

    /// Flattens a nested Optional.
    ///
    /// Converts `Optional[Optional[T]]` to `Optional[T]`.
    ///
    /// Example:
    ///     Some(Some(42)).flatten()  // Some(42)
    ///     Some(None).flatten()      // None
    ///     None.flatten()            // None
    public func flatten[U]() -> Optional[U] where T = Optional[U] {
        match self {
            .Some(inner) => inner,
            .None => Optional[U].None
        }
    }

    /// Returns Some if predicate returns true, otherwise None.
    ///
    /// Example:
    ///     Some(4).filter({ it % 2 == 0 })  // Some(4)
    ///     Some(3).filter({ it % 2 == 0 })  // None
    ///     None.filter({ it % 2 == 0 })     // None
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

    /// Calls the function with the contained value for side effects.
    ///
    /// Returns self unchanged. Useful for logging or debugging in a chain.
    ///
    /// Example:
    ///     getUser(id)
    ///         .inspect({ print("Found: \{it.name}") })
    ///         .map({ it.email })
    public func inspect(fn: (T) -> ()) -> Optional[T] {
        match self {
            .Some(value) => {
                fn(value);
                .Some(value)
            },
            .None => .None
        }
    }

    // ========================================================================
    // COMBINATORS
    // ========================================================================

    /// Returns other if this is Some, otherwise None.
    ///
    /// Note: `other` is eagerly evaluated. Use `flatMap` for lazy evaluation.
    ///
    /// Example:
    ///     Some(1).then(other: Some("a"))  // Some("a")
    ///     Some(1).then(other: None)       // None
    ///     None.then(other: Some("a"))     // None
    public func then[U](other: Optional[U]) -> Optional[U] {
        match self {
            .Some(_) => other,
            .None => .None
        }
    }

    /// Returns this if Some, otherwise calls alternative.
    ///
    /// For the common case of providing a default value, prefer the `??` operator.
    /// Use `orElse` when you need to return another Optional (not unwrap).
    ///
    /// Example:
    ///     Some(1).orElse(|| Some(2))        // Some(1), no call
    ///     None.orElse(|| Some(2))           // Some(2)
    ///     None.orElse(|| loadFromCache())   // calls function
    ///
    ///     // For unwrapping with a default, prefer ??:
    ///     let value = optionalInt ?? 0
    public func orElse(alternative: () -> Optional[T]) -> Optional[T] {
        match self {
            .Some(value) => .Some(value),
            .None => alternative()
        }
    }

    /// Returns Some if exactly one of this or other is Some.
    ///
    /// Example:
    ///     Some(1).xor(other: None)     // Some(1)
    ///     None.xor(other: Some(2))     // Some(2)
    ///     Some(1).xor(other: Some(2))  // None
    ///     None.xor(other: None)        // None
    public func xor(other: Optional[T]) -> Optional[T] {
        match (self, other) {
            (.Some(value), .None) => .Some(value),
            (.None, .Some(value)) => .Some(value),
            _ => .None
        }
    }

    /// Combines two Optionals into an Optional of a tuple.
    ///
    /// Returns Some only if both are Some.
    ///
    /// Example:
    ///     Some(1).zip(with: Some("a"))  // Some((1, "a"))
    ///     Some(1).zip(with: None)       // None
    ///     None.zip(with: Some("a"))     // None
    public func zip[U](with other: Optional[U]) -> Optional[(T, U)] {
        match (self, other) {
            (.Some(a), .Some(b)) => .Some((a, b)),
            _ => .None
        }
    }

    // ========================================================================
    // CONVERSION TO RESULT
    // ========================================================================

    /// Converts to Result, using the provided error if None.
    ///
    /// Note: The error is eagerly evaluated. Use `okOrElse` for lazy evaluation.
    ///
    /// Example:
    ///     Some(42).okOr(error: "missing")  // Ok(42)
    ///     None.okOr(error: "missing")      // Err("missing")
    public func okOr[E](error: E) -> Result[T, E] {
        match self {
            .Some(value) => .Ok(value),
            .None => .Err(error)
        }
    }

    /// Converts to Result, calling the function to create an error if None.
    ///
    /// Example:
    ///     Some(42).okOrElse(|| NotFoundError())  // Ok(42), no call
    ///     None.okOrElse(|| NotFoundError())      // Err(NotFoundError())
    public func okOrElse[E](error: () -> E) -> Result[T, E] {
        match self {
            .Some(value) => .Ok(value),
            .None => .Err(error())
        }
    }

    // ========================================================================
    // MUTATING OPERATIONS
    // ========================================================================

    /// Takes the value out, leaving None in its place.
    ///
    /// Returns the original value (Some or None).
    ///
    /// Example:
    ///     var opt = Some(42)
    ///     opt.take()  // Some(42), opt is now None
    ///     opt.take()  // None, opt is still None
    public mutating func take() -> Optional[T] {
        let result = self;
        self = .None;
        result
    }

    /// Replaces the value, returning the old value.
    ///
    /// Example:
    ///     var opt = Some(1)
    ///     opt.replace(value: 2)  // Some(1), opt is now Some(2)
    ///
    ///     var none: Int64? = null
    ///     none.replace(value: 1)  // None, none is now Some(1)
    public mutating func replace(value: T) -> Optional[T] {
        let old = self;
        self = .Some(value);
        old
    }

    /// Takes the value if the predicate is satisfied.
    ///
    /// If Some and predicate returns true, takes the value leaving None.
    /// Otherwise returns None and leaves self unchanged.
    ///
    /// Example:
    ///     var opt = Some(42)
    ///     opt.takeIf({ it > 0 })   // Some(42), opt is now None
    ///
    ///     var opt2 = Some(42)
    ///     opt2.takeIf({ it < 0 })  // None, opt2 is still Some(42)
    public mutating func takeIf(predicate: (T) -> Bool) -> Optional[T] {
        match self {
            .Some(value) => {
                if predicate(value) {
                    self = .None;
                    .Some(value)
                } else {
                    .None
                }
            },
            .None => .None
        }
    }

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns an iterator over the contained value (0 or 1 elements).
    ///
    /// Useful for integrating with iterator-based APIs.
    ///
    /// Example:
    ///     for value in Some(42).iter() {
    ///         print(value)  // prints 42
    ///     }
    ///
    ///     for value in None.iter() {
    ///         print(value)  // never executes
    ///     }
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
    /// Two Optionals are equal if both are None, or both are Some with
    /// equal values.
    ///
    /// Example:
    ///     Some(1) == Some(1)  // true
    ///     Some(1) == Some(2)  // false
    ///     Some(1) == None     // false
    ///     None == None        // true
    public func equals(other: Optional[T]) -> Bool {
        match (self, other) {
            (.Some(a), .Some(b)) => a == b,
            (.None, .None) => true,
            _ => false
        }
    }

    /// Returns true if this is Some containing the given value.
    ///
    /// Example:
    ///     Some(42).contains(value: 42)  // true
    ///     Some(42).contains(value: 0)   // false
    ///     None.contains(value: 42)      // false
    public func contains(value: T) -> Bool {
        match self {
            .Some(inner) => inner == value,
            .None => false
        }
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - COMPARABLE
// ============================================================================

/// Extension for Optionals with comparable values.
///
/// None is considered less than any Some value.
extend Optional[T]: Comparable where T: Comparable {

    /// Compares two Optionals.
    ///
    /// Ordering: None < Some(x) for any x.
    /// When both are Some, compares the contained values.
    ///
    /// Example:
    ///     None < Some(1)      // true
    ///     Some(1) < Some(2)   // true
    ///     Some(2) < Some(1)   // false
    public func compare(other: Optional[T]) -> Ordering {
        match (self, other) {
            (.None, .None) => .Equal,
            (.None, .Some(_)) => .Less,
            (.Some(_), .None) => .Greater,
            (.Some(a), .Some(b)) => a.compare(b)
        }
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - HASH
// ============================================================================

/// Extension for Optionals with hashable values.
extend Optional[T]: Hash where T: Hash {

    /// Hashes this Optional into the given hasher.
    ///
    /// None has a distinct hash from any Some value.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        match self {
            .Some(value) => {
                // Write 1 to indicate Some
                let marker: Int64 = 1;
                hasher.write(Slice(pointer: Pointer(to: marker).asRaw().cast[UInt8](), count: Int64(intLiteral: 8)));
                value.hash(into: hasher)
            },
            .None => {
                // Write 0 to indicate None
                let marker: Int64 = 0;
                hasher.write(Slice(pointer: Pointer(to: marker).asRaw().cast[UInt8](), count: Int64(intLiteral: 8)))
            }
        }
    }
}

// ============================================================================
// EXTENSIONS - CLONE
// ============================================================================

/// Clone-like helper available for all Optionals.
extend Optional[T] {

    /// Creates a deep clone of the Optional.
    ///
    /// Example:
    ///     let opt = Some([1, 2, 3])
    ///     let copy = opt.clone()  // independent copy
    public func clone() -> Optional[T] {
        match self {
            .Some(value) => .Some(value),
            .None => .None
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
///
/// Example:
///     func process() -> Int64? {
///         let a = try getA()  // returns None early if None
///         let b = try getB()
///         return a + b
///     }
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
///
/// Example:
///     "\{Some(42)}"    // "Some(42)"
///     "\{None}"        // "None"
///     "\{Some(42):?}"  // "Optional.Some(42)"
extend Optional[T]: Formattable where T: Formattable {

    /// Formats this optional.
    ///
    /// Default format: "Some(value)" or "None".
    /// Debug format: "Optional.Some(value)" or "Optional.None".
    public func format(options: FormatOptions = FormatOptions.default()) -> String {
        match self {
            .Some(value) => "Some(" + value.format(options) + ")",
            .None => "None"
        }
    }
}

/// ExpressibleByNullLiteral - allows `null` to create Optional.None.
///
/// Example:
///     let opt: Int64? = null  // None
///     if condition { return null }
extend Optional[T]: ExpressibleByNullLiteral {
    public init() {
        self = .None
    }
}

/// Coalesce extension enabling the ?? operator.
///
/// Example:
///     let value = optionalInt ?? 0
///     let name = user?.name ?? "Anonymous"
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
///
/// Example:
///     let items: [Int64] = Some(42).iter().collect()  // [42]
///     let empty: [Int64] = None.iter().collect()      // []
public struct OptionalIterator[T]: Iterator {
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
///
/// This provides convenient syntax for optional types.
///
/// Example:
///     var name: String? = null       // same as Optional[String]
///     func find(id: Int64) -> User?  // returns Optional[User]
///     let nested: Int64?? = null     // Optional[Optional[Int64]]
@builtin(.OptionalTypeOperator)
public type OptionalTypeOperator[T] = Optional[T];
