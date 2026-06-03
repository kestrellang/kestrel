// Optional[T] - represents a value that may or may not be present

module std.result

import std.core.(Equatable, Comparable, Ordering, Hashable, Hasher, Bool, ControlFlow, Tryable, FromResidual, FromValue, ExpressibleByNullLiteral, Coalesce, fatalError)
import std.text.(String, StringBuilder, FormatOptions, Formattable)
import std.result.(Result)
import std.numeric.(Int64, UInt8)
import std.memory.(ArraySlice, Pointer)
import std.iter.(Iterator)

/// A type-safe stand-in for nullable references — either `Some(value)` or
/// `None`.
///
/// `T?` desugars to `Optional[T]`, and the `null` literal constructs
/// `.None` for any optional type. The compiler refuses to let you read the
/// inner value without handling the `None` case, which is the whole point
/// — there is no implicit unwrap. The `try` operator (and the `??`
/// coalescing operator) propagate `None` through call chains so you can
/// write linear code without nested `match` blocks.
///
/// # Examples
///
/// ```
/// func find(id: Int64) -> User? {
///     if let user = users.get(id) { return user };
///     null
/// }
///
/// match find(42) {
///     .Some(let u) => print(u.name),
///     .None        => print("Not found")
/// }
/// ```
///
/// ```
/// // `try` short-circuits on None
/// func combine() -> Int64? {
///     let a = try getA();   // returns None early if getA() is None
///     let b = try getB();
///     a + b
/// }
/// ```
///
/// # Representation
///
/// A two-case tagged union — one byte (or whatever the backend picks) of
/// discriminant plus the payload of `T`. The compiler will use a niche
/// when one is available (e.g. a non-zero pointer), so `Optional[Pointer]`
/// is the same size as `Pointer`.
@builtin(.OptionalEnum)
public enum Optional[T]: not Copyable {
    /// Wraps a present value of `T`.
    @builtin(.OptionalSomeCase)
    case Some(T)

    /// The absent state — same shape as a null reference, but checked at
    /// the type level.
    @builtin(.OptionalNoneCase)
    case None

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Wraps `value` in `.Some`. Rarely needed in practice — bare values
    /// are promoted via `FromValue`, and `let x: Int64? = 42` does the
    /// right thing.
    ///
    /// # Examples
    ///
    /// ```
    /// let opt = Optional.wrap(42);   // Some(42)
    /// let opt: Int64? = 42;                 // identical, preferred
    /// ```
    public static func wrap(value: T) -> Optional[T] {
        .Some(value)
    }

    /// Returns `.None`. Prefer the `null` literal — it works in any
    /// optional context without naming the type parameter.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = Optional[Int64].none();   // None
    /// let b: Int64? = null;             // identical, preferred
    /// ```
    public static func none() -> Optional[T] {
        .None
    }

    // ========================================================================
    // QUERY METHODS
    // ========================================================================

    /// True when this is `.Some`. Cheap discriminator-only check.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(42).isSome();   // true
    /// None.isSome();       // false
    /// ```
    public func isSome() -> Bool {
        match self {
            .Some(_) => true,
            .None => false
        }
    }

    /// True when this is `.None`. The complement of `isSome`.
    public func isNone() -> Bool {
        match self {
            .Some(_) => false,
            .None => true
        }
    }

    /// True when `.Some(value)` and `predicate(value)` returns `true`.
    /// `None` always answers `false` without invoking the predicate.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(42).isSomeAnd { it > 0 };    // true
    /// Some(-1).isSomeAnd { it > 0 };    // false
    /// None.isSomeAnd { it > 0 };        // false
    /// ```
    public func isSomeAnd(predicate: (T) -> Bool) -> Bool {
        match self {
            .Some(value) => predicate(value),
            .None => false
        }
    }

    // ========================================================================
    // UNWRAPPING
    // ========================================================================

    /// Returns the wrapped value, panicking if `None`. Reach for
    /// `unwrap(or:)`, the `??` operator, or pattern matching unless
    /// you can prove the value is `Some`.
    ///
    /// # Errors
    ///
    /// Panics with `"called unwrap() on None"` when invoked on `.None`.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(42).unwrap();   // 42
    /// None.unwrap();       // PANIC
    /// ```
    public func unwrap() -> T {
        match self {
            .Some(value) => value,
            .None => fatalError("called unwrap() on None")
        }
    }

    /// Like `unwrap`, but the panic carries `message` instead of the
    /// generic text. Use this where the absence of a value should crash
    /// loudly with context.
    ///
    /// # Errors
    ///
    /// Panics with `message` on `.None` via `fatalError`.
    ///
    /// # Examples
    ///
    /// ```
    /// let cfg = loadConfig().expect("Config file required");
    /// ```
    public func expect(message: String) -> T {
        match self {
            .Some(value) => value,
            .None => fatalError(message)
        }
    }

    /// Returns the wrapped value or `default` when `None`. `default` is
    /// always evaluated — use `unwrap(orElse:)` if computing it is
    /// expensive.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(42).unwrap(or: 0);   // 42
    /// None.unwrap(or: 0);       // 0
    /// ```
    public func unwrap(or default: T) -> T {
        match self {
            .Some(value) => value,
            .None => default
        }
    }

    /// Like `unwrap(or:)`, but `defaultFn` is only called on `None`. Use this
    /// when the default is expensive to compute or has side effects.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(42).unwrap(orElse: { expensiveDefault() });   // 42, no call
    /// None.unwrap(orElse: { expensiveDefault() });       // calls fn
    /// ```
    public func unwrap(orElse defaultFn: () -> T) -> T {
        match self {
            .Some(value) => value,
            .None => defaultFn()
        }
    }

    // ========================================================================
    // TRANSFORMATIONS
    // ========================================================================

    /// Functor map — applies `transform` to the wrapped value, leaving
    /// `None` untouched.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(2).map { it * 2 };           // Some(4)
    /// None.map { it * 2 };              // None
    /// Some("hello").map { it.len };     // Some(5)
    /// ```
    public func map[U](transform: (T) -> U) -> Optional[U] {
        match self {
            .Some(value) => .Some(transform(value)),
            .None => .None
        }
    }

    /// Monadic bind — apply a transform that itself returns an
    /// `Optional`, without nesting. Equivalent to `map(...).flatten()`.
    ///
    /// # Examples
    ///
    /// ```
    /// func parse(s: String) -> Int64? { ... }
    ///
    /// Some("42").flatMap(parse);    // Some(42)
    /// Some("abc").flatMap(parse);   // None  (parse failed)
    /// None.flatMap(parse);          // None
    /// ```
    public func flatMap[U](transform: (T) -> Optional[U]) -> Optional[U] {
        match self {
            .Some(value) => transform(value),
            .None => .None
        }
    }

    /// Collapses an `Optional[Optional[T]]` one level. Available only
    /// when `T` is itself an `Optional`.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(Some(42)).flatten();   // Some(42)
    /// Some(None).flatten();       // None
    /// None.flatten();             // None
    /// ```
    public func flatten[U]() -> Optional[U] where T = Optional[U] {
        match self {
            .Some(inner) => inner,
            .None => Optional[U].None
        }
    }

    /// Returns `Some(value)` when the predicate accepts the value, `None`
    /// otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(4).filter { it % 2 == 0 };   // Some(4)
    /// Some(3).filter { it % 2 == 0 };   // None
    /// None.filter { it % 2 == 0 };      // None
    /// ```
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

    /// Side-effecting tap — runs `fn` on the wrapped value (if any) and
    /// returns `self` unchanged. Useful for logging or assertions inside
    /// a chain.
    ///
    /// # Examples
    ///
    /// ```
    /// getUser(id)
    ///     .inspect { print("Found: \{it.name}") }
    ///     .map { it.email };
    /// ```
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

    /// Returns `other` when `self` is `Some`, otherwise `None`. `other` is
    /// evaluated eagerly — use `flatMap` for lazy chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(1).then(Some("a"));    // Some("a")
    /// Some(1).then(None);         // None
    /// None.then(Some("a"));       // None
    /// ```
    public func then[U](other: Optional[U]) -> Optional[U] {
        match self {
            .Some(_) => other,
            .None => .None
        }
    }

    /// Returns `self` when `Some`, otherwise the result of `alternative()`.
    /// For "use a default scalar" prefer the `??` operator; reach for
    /// `orElse` when the fallback itself is an `Optional`.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(1).orElse { Some(2) };        // Some(1), fn not called
    /// None.orElse { Some(2) };           // Some(2)
    /// None.orElse { loadFromCache() };   // calls fn
    ///
    /// // For unwrapping with a default, prefer ??:
    /// let value = optionalInt ?? 0;
    /// ```
    public func orElse(alternative: () -> Optional[T]) -> Optional[T] {
        match self {
            .Some(value) => .Some(value),
            .None => alternative()
        }
    }

    /// Exclusive-or of presence — returns the unique `Some` when exactly
    /// one of `self`/`other` is set, else `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(1).xor(None);       // Some(1)
    /// None.xor(Some(2));       // Some(2)
    /// Some(1).xor(Some(2));    // None
    /// None.xor(None);          // None
    /// ```
    public func xor(other: Optional[T]) -> Optional[T] {
        match (self, other) {
            (.Some(value), .None) => .Some(value),
            (.None, .Some(value)) => .Some(value),
            _ => .None
        }
    }

    /// Pairs two optionals into an optional tuple. `Some` only when both
    /// inputs are `Some`.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(1).zip(with: Some("a"));   // Some((1, "a"))
    /// Some(1).zip(with: None);        // None
    /// None.zip(with: Some("a"));      // None
    /// ```
    public func zip[U](with other: Optional[U]) -> Optional[(T, U)] {
        match (self, other) {
            (.Some(a), .Some(b)) => .Some((a, b)),
            _ => .None
        }
    }

    // ========================================================================
    // CONVERSION TO RESULT
    // ========================================================================

    /// Promotes to `Result`, supplying `error` for the `None` branch.
    /// `error` is eagerly evaluated — use `okOrElse` to defer it.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(42).okOr("missing");   // Ok(42)
    /// None.okOr("missing");       // Err("missing")
    /// ```
    public func okOr[E](error: E) -> Result[T, E] {
        match self {
            .Some(value) => .Ok(value),
            .None => .Err(error)
        }
    }

    /// Like `okOr`, but `error()` is only invoked on `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(42).okOrElse { NotFoundError() };   // Ok(42), fn not called
    /// None.okOrElse { NotFoundError() };       // Err(NotFoundError())
    /// ```
    public func okOrElse[E](error: () -> E) -> Result[T, E] {
        match self {
            .Some(value) => .Ok(value),
            .None => .Err(error())
        }
    }

    // ========================================================================
    // MUTATING OPERATIONS
    // ========================================================================

    /// Removes and returns the current value, leaving `self` as `None`.
    /// Idiomatic for "consume once" optional fields.
    ///
    /// # Examples
    ///
    /// ```
    /// var opt = Some(42);
    /// opt.take();   // Some(42); opt is now None
    /// opt.take();   // None;     opt is still None
    /// ```
    public mutating func take() -> Optional[T] {
        let result = self;
        self = .None;
        result
    }

    /// Stores `value` and returns whatever was there before. Mirror of
    /// `take` for the assignment direction.
    ///
    /// # Examples
    ///
    /// ```
    /// var opt = Some(1);
    /// opt.replace(2);    // Some(1); opt is now Some(2)
    ///
    /// var none: Int64? = null;
    /// none.replace(1);   // None;    none is now Some(1)
    /// ```
    public mutating func replace(value: T) -> Optional[T] {
        let old = self;
        self = .Some(value);
        old
    }

    /// Conditional `take` — empties `self` and returns the value only when
    /// `predicate(value)` accepts it. Otherwise leaves `self` untouched
    /// and returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// var opt = Some(42);
    /// opt.take(where: { it > 0 });    // Some(42); opt is now None
    ///
    /// var opt2 = Some(42);
    /// opt2.take(where: { it < 0 });   // None;     opt2 is still Some(42)
    /// ```
    public mutating func take(where predicate: (T) -> Bool) -> Optional[T] {
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

    /// Returns an `OptionalIterator` that yields one element if `Some` or
    /// zero elements if `None`. Lets an optional plug into any `for-in`
    /// or iterator-combinator pipeline.
    ///
    /// # Examples
    ///
    /// ```
    /// for value in Some(42).iter() {
    ///     print(value)   // prints 42
    /// };
    ///
    /// for value in None.iter() {
    ///     print(value)   // never executes
    /// };
    /// ```
    public func iter() -> OptionalIterator[T] {
        OptionalIterator(self)
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - EQUATABLE
// ============================================================================

/// `Optional` is move-only by default (`not Copyable`) so it can wrap a
/// non-Copyable payload (e.g. `File?`). It regains copy semantics only when the
/// wrapped type is itself Copyable — so `Int64?` is Copyable while `Array?`
/// stays move-only.
extend Optional[T]: Copyable where T: Copyable { }

/// Equatable when the inner type is — `None == None` is true, `Some(a) ==
/// Some(b)` defers to `T.isEqual`, and a present value is never equal to
/// `None`.
extend Optional[T]: Equatable where T: Equatable {

    /// Structural equality on the optional. Backs `==`.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(1) == Some(1);   // true
    /// Some(1) == Some(2);   // false
    /// Some(1) == None;      // false
    /// None == None;         // true
    /// ```
    public func isEqual(to other: Optional[T]) -> Bool {
        match (self, other) {
            (.Some(a), .Some(b)) => a == b,
            (.None, .None) => true,
            _ => false
        }
    }

    /// True when `self` is `Some` and the wrapped value equals `value`.
    /// Slightly cheaper than `== Some(value)` when you already have the
    /// bare value.
    ///
    /// # Examples
    ///
    /// ```
    /// Some(42).contains(42);   // true
    /// Some(42).contains(0);    // false
    /// None.contains(42);       // false
    /// ```
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

/// Comparable when the inner type is. The total order treats `None` as
/// less than every `Some`, so sorting `[Some(2), None, Some(1)]` gives
/// `[None, Some(1), Some(2)]`.
extend Optional[T]: Comparable where T: Comparable {

    /// Three-way compare. `None < Some(_)`; two `Some`s defer to the
    /// inner `compare`.
    ///
    /// # Examples
    ///
    /// ```
    /// None < Some(1);     // true
    /// Some(1) < Some(2);  // true
    /// Some(2) < Some(1);  // false
    /// ```
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

/// Hashable when the inner type is. The discriminant is mixed in first so
/// `None` and `Some(0)` hash to different values.
extend Optional[T]: Hashable where T: Hashable {

    /// Mixes a one-byte tag (`0` for `None`, `1` for `Some`) into the
    /// hasher, then defers to `T.hash` for the payload.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        match self {
            .Some(value) => {
                // Write 1 to indicate Some
                let marker: Int64 = 1;
                hasher.write(ArraySlice(pointer: Pointer(to: marker).asRaw().cast[UInt8](), count: 8));
                value.hash(into: hasher)
            },
            .None => {
                // Write 0 to indicate None
                let marker: Int64 = 0;
                hasher.write(ArraySlice(pointer: Pointer(to: marker).asRaw().cast[UInt8](), count: 8))
            }
        }
    }
}

// ============================================================================
// EXTENSIONS - CLONE
// ============================================================================

/// Clone helper available for every `Optional[T]` (the inner clone falls
/// out of value-semantics on `T`).
extend Optional[T] {

    /// Returns an independent copy. For value types this is a shallow
    /// copy of the payload; for COW types, the underlying buffer is
    /// shared until first mutation.
    ///
    /// # Examples
    ///
    /// ```
    /// let opt = Some([1, 2, 3]);
    /// let copy = opt.clone();   // independent copy
    /// ```
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

/// `Tryable` conformance — lets `try someOptional` extract the inner
/// value or short-circuit the enclosing function with `None`.
///
/// # Examples
///
/// ```
/// func process() -> Int64? {
///     let a = try getA();   // returns None early if getA() is None
///     let b = try getB();
///     a + b
/// }
/// ```
extend Optional[T]: Tryable {
    type Output = T
    type Residual = ()

    /// Drives `try` — `Continue(value)` for `Some`, `Break(())` for `None`.
    public consuming func tryExtract() -> ControlFlow[T, ()] {
        match self {
            .Some(value) => .Continue(value),
            .None => .Break(())
        }
    }
}

/// `ForceUnwrap` — drives the postfix `!` operator. `value!` returns the
/// wrapped value for `.Some`, or aborts via `fatalError` for `.None`.
extend Optional[T]: ForceUnwrap {
    type Output = T

    /// Returns the wrapped value, trapping on `.None`. Backs `value!`.
    public consuming func forceUnwrap() -> T {
        match self {
            .Some(value) => value,
            .None => fatalError("unwrapped a nil Optional")
        }
    }
}

/// `FromResidual[()]` — turns a `try`-propagated `()` residual back into
/// `.None` so chains of `try` returning optionals compose.
extend Optional[T]: FromResidual[()] {
    /// Builds `.None` from the residual produced by a `try` short-circuit.
    public static func fromResidual(residual: ()) -> Optional[T] {
        .None
    }
}

/// `FromValue[T]` — promotes a bare `T` to `Optional[T]` so
/// `let x: Int? = 5` works without explicit `.Some`.
extend Optional[T]: FromValue[T] {
    /// Wraps `value` in `.Some`. Called by the compiler at the promotion
    /// site, not usually by user code.
    public static func from(value: T) -> Optional[T] {
        .Some(value)
    }
}

/// `Formattable` when `T` is. Default rendering is `Some(value)` /
/// `None`; the debug specifier `:?` (handled by the formatter) prepends
/// `Optional.`.
///
/// # Examples
///
/// ```
/// "\{Some(42)}";     // "Some(42)"
/// "\{None}";         // "None"
/// "\{Some(42):?}";   // "Optional.Some(42)"
/// ```
extend Optional[T]: Formattable where T: Formattable {

    /// Renders `Some(...)` or `None`, forwarding `options` to the inner
    /// `format` for the payload.
    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        match self {
            .Some(value) => {
                writer.append("Some(");
                value.format(into: writer, options);
                writer.append(char: ')')
            },
            .None => writer.append("None")
        }
    }
}

/// `ExpressibleByNullLiteral` — makes the `null` literal yield `.None`
/// in any optional context.
///
/// # Examples
///
/// ```
/// let opt: Int64? = null;            // None
/// if condition { return null };
/// ```
extend Optional[T]: ExpressibleByNullLiteral {
    /// @name Null Literal
    /// Compiler-emitted bridge for the `null` literal. Always constructs
    /// `.None`.
    public init() {
        self = .None
    }
}

/// `Coalesce` — backs the `??` operator with lazy default evaluation.
///
/// # Examples
///
/// ```
/// let value = optionalInt ?? 0;
/// let name  = user?.name   ?? "Anonymous";
/// ```
extend Optional[T]: Coalesce[T] {
    type Coalesce.Output = T

    /// Returns the wrapped value or evaluates `default()`. The default is
    /// only invoked on `None`, which is what makes `??` cheap on the
    /// happy path.
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

/// Single-shot iterator yielding zero or one elements. Returned by
/// `Optional.iter()`.
///
/// # Examples
///
/// ```
/// let items: [Int64] = Some(42).iter().collect();   // [42]
/// let empty: [Int64] = None.iter().collect();       // []
/// ```
///
/// # Representation
///
/// One `Optional[T]` field. `next()` empties it on first call.
public struct OptionalIterator[T]: Iterator {
    type Item = T

    private var value: Optional[T]

    /// @name From Optional
    /// Builds an iterator that will yield the contents of `value` on its
    /// first `next()` call (or terminate immediately if `value` is `None`).
    public init(value: Optional[T]) {
        self.value = value;
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

/// Compiler hook — `T?` desugars to `Optional[T]` via this alias. End
/// users should write the sugar; this declaration exists so the operator
/// can resolve to a concrete type.
///
/// # Examples
///
/// ```
/// var name: String? = null;            // same as Optional[String]
/// func find(id: Int64) -> User?;       // returns Optional[User]
/// let nested: Int64?? = null;          // Optional[Optional[Int64]]
/// ```
@builtin(.OptionalTypeOperator)
public type OptionalTypeOperator[T] = Optional[T];
