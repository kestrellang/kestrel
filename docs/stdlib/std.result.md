# std.result

## enum `Optional`

```kestrel
public enum Optional[T]: not Copyable where T: not Static
```

_Defined in `lang/std/result/optional.ks`._

### Members

#### case `None`

```kestrel
case None
```

The absent state — same shape as a null reference, but checked at
the type level.

_Defined in `lang/std/result/optional.ks`._

#### initializer `Null Literal`

```kestrel
public init()
```

Compiler-emitted bridge for the `null` literal. Always constructs
`.None`.

_Defined in `lang/std/result/optional.ks`._

#### case `Some`

```kestrel
case Some(T)
```

Wraps a present value of `T`.

_Defined in `lang/std/result/optional.ks`._

#### function `asRef`

```kestrel
public func asRef() -> Optional[&T]
```

Borrows the contained value without consuming this optional.

_Defined in `lang/std/result/optional.ks`._

#### function `clone`

```kestrel
public func clone() -> Optional[T]
```

Returns an independent copy. For value types this is a shallow
copy of the payload; for COW types, the underlying buffer is
shared until first mutation.

##### Examples

```
let opt = Some([1, 2, 3]);
let copy = opt.clone();   // independent copy
```

_Defined in `lang/std/result/optional.ks`._

#### function `contains`

```kestrel
public func contains(T) -> Bool where T: Equatable
```

True when `self` is `Some` and the wrapped value equals `value`.
Slightly cheaper than `== Some(value)` when you already have the
bare value.

##### Examples

```
Some(42).contains(42);   // true
Some(42).contains(0);    // false
None.contains(42);       // false
```

_Defined in `lang/std/result/optional.ks`._

#### function `expect`

```kestrel
public consuming func expect(String) -> T
```

Like `unwrap`, but the panic carries `message` instead of the
generic text. Use this where the absence of a value should crash
loudly with context.

##### Errors

Panics with `message` on `.None` via `fatalError`.

##### Examples

```
let cfg = loadConfig().expect("Config file required");
```

_Defined in `lang/std/result/optional.ks`._

#### function `filter`

```kestrel
public consuming func filter(where: (T) -> Bool) -> Optional[T]
```

Returns `Some(value)` when the predicate accepts the value, `None`
otherwise.

##### Examples

```
Some(4).filter(where: { it % 2 == 0 });   // Some(4)
Some(3).filter(where: { it % 2 == 0 });   // None
None.filter(where: { it % 2 == 0 });      // None
```

_Defined in `lang/std/result/optional.ks`._

#### function `flatMap`

```kestrel
public consuming func flatMap[U]((T) -> Optional[U]) -> Optional[U]
```

Monadic bind — apply a transform that itself returns an
`Optional`, without nesting. Equivalent to `map(...).flatten()`.

##### Examples

```
func parse(s: String) -> Int64? { ... }

Some("42").flatMap(parse);    // Some(42)
Some("abc").flatMap(parse);   // None  (parse failed)
None.flatMap(parse);          // None
```

_Defined in `lang/std/result/optional.ks`._

#### function `flatten`

```kestrel
public consuming func flatten[U]() -> Optional[U] where T == Optional[U]
```

Collapses an `Optional[Optional[T]]` one level. Available only
when `T` is itself an `Optional`.

##### Examples

```
Some(Some(42)).flatten();   // Some(42)
Some(None).flatten();       // None
None.flatten();             // None
```

_Defined in `lang/std/result/optional.ks`._

#### function `inspect`

```kestrel
public consuming func inspect((T) -> ()) -> Optional[T]
```

Side-effecting tap — runs `fn` on the wrapped value (if any) and
returns `self` unchanged. Useful for logging or assertions inside
a chain.

##### Examples

```
getUser(id)
    .inspect { print("Found: \{it.name}") }
    .map { it.email };
```

_Defined in `lang/std/result/optional.ks`._

#### function `isNone`

```kestrel
public func isNone() -> Bool
```

True when this is `.None`. The complement of `isSome`.

_Defined in `lang/std/result/optional.ks`._

#### function `isSome`

```kestrel
public func isSome() -> Bool
```

True when this is `.Some`. Cheap discriminator-only check.

##### Examples

```
Some(42).isSome();   // true
None.isSome();       // false
```

_Defined in `lang/std/result/optional.ks`._

#### function `isSomeAnd`

```kestrel
public func isSomeAnd(where: (T) -> Bool) -> Bool
```

True when `.Some(value)` and `predicate(value)` returns `true`.
`None` always answers `false` without invoking the predicate.

##### Examples

```
Some(42).isSomeAnd(where: { it > 0 });    // true
Some(-1).isSomeAnd(where: { it > 0 });    // false
None.isSomeAnd(where: { it > 0 });        // false
```

_Defined in `lang/std/result/optional.ks`._

#### function `iter`

```kestrel
public consuming func iter() -> OptionalIterator[T]
```

Returns an `OptionalIterator` that yields one element if `Some` or
zero elements if `None`. Lets an optional plug into any `for-in`
or iterator-combinator pipeline.

##### Examples

```
for value in Some(42).iter() {
    print(value)   // prints 42
};

for value in None.iter() {
    print(value)   // never executes
};
```

_Defined in `lang/std/result/optional.ks`._

#### function `map`

```kestrel
public consuming func map[U]((T) -> U) -> Optional[U]
```

Functor map — applies `transform` to the wrapped value, leaving
`None` untouched.

##### Examples

```
Some(2).map { it * 2 };           // Some(4)
None.map { it * 2 };              // None
Some("hello").map { it.len };     // Some(5)
```

_Defined in `lang/std/result/optional.ks`._

#### function `none`

```kestrel
public static func none() -> Optional[T]
```

Returns `.None`. Prefer the `null` literal — it works in any
optional context without naming the type parameter.

##### Examples

```
let a = Optional[Int64].none();   // None
let b: Int64? = null;             // identical, preferred
```

_Defined in `lang/std/result/optional.ks`._

#### function `okOr`

```kestrel
public consuming func okOr[E](consuming E) -> Result[T, E]
```

Promotes to `Result`, supplying `error` for the `None` branch.
`error` is eagerly evaluated — use `okOrElse` to defer it.

##### Examples

```
Some(42).okOr("missing");   // Ok(42)
None.okOr("missing");       // Err("missing")
```

_Defined in `lang/std/result/optional.ks`._

#### function `okOrElse`

```kestrel
public consuming func okOrElse[E](() -> E) -> Result[T, E]
```

Like `okOr`, but `error()` is only invoked on `None`.

##### Examples

```
Some(42).okOrElse { NotFoundError() };   // Ok(42), fn not called
None.okOrElse { NotFoundError() };       // Err(NotFoundError())
```

_Defined in `lang/std/result/optional.ks`._

#### function `orElse`

```kestrel
public consuming func orElse(() -> Optional[T]) -> Optional[T]
```

Returns `self` when `Some`, otherwise the result of `alternative()`.
For "use a default scalar" prefer the `??` operator; reach for
`orElse` when the fallback itself is an `Optional`.

##### Examples

```
Some(1).orElse { Some(2) };        // Some(1), fn not called
None.orElse { Some(2) };           // Some(2)
None.orElse { loadFromCache() };   // calls fn

// For unwrapping with a default, prefer ??:
let value = optionalInt ?? 0;
```

_Defined in `lang/std/result/optional.ks`._

#### function `replace`

```kestrel
public mutating func replace(T) -> Optional[T]
```

Stores `value` and returns whatever was there before. Mirror of
`take` for the assignment direction.

##### Examples

```
var opt = Some(1);
opt.replace(2);    // Some(1); opt is now Some(2)

var none: Int64? = null;
none.replace(1);   // None;    none is now Some(1)
```

_Defined in `lang/std/result/optional.ks`._

#### function `take`

```kestrel
public mutating func take() -> Optional[T]
```

Removes and returns the current value, leaving `self` as `None`.
Idiomatic for "consume once" optional fields.

##### Examples

```
var opt = Some(42);
opt.take();   // Some(42); opt is now None
opt.take();   // None;     opt is still None
```

_Defined in `lang/std/result/optional.ks`._

#### function `take`

```kestrel
public mutating func take(where: (T) -> Bool) -> Optional[T]
```

Conditional `take` — empties `self` and returns the value only when
`predicate(value)` accepts it. Otherwise leaves `self` untouched
and returns `None`.

##### Examples

```
var opt = Some(42);
opt.take(where: { it > 0 });    // Some(42); opt is now None

var opt2 = Some(42);
opt2.take(where: { it < 0 });   // None;     opt2 is still Some(42)
```

_Defined in `lang/std/result/optional.ks`._

#### function `then`

```kestrel
public consuming func then[U](consuming Optional[U]) -> Optional[U]
```

Returns `other` when `self` is `Some`, otherwise `None`. `other` is
evaluated eagerly — use `flatMap` for lazy chaining.

##### Examples

```
Some(1).then(Some("a"));    // Some("a")
Some(1).then(None);         // None
None.then(Some("a"));       // None
```

_Defined in `lang/std/result/optional.ks`._

#### function `unwrap`

```kestrel
public consuming func unwrap() -> T
```

Returns the wrapped value, panicking if `None`. Reach for
`unwrap(or:)`, the `??` operator, or pattern matching unless
you can prove the value is `Some`.

##### Errors

Panics with `"called unwrap() on None"` when invoked on `.None`.

##### Examples

```
Some(42).unwrap();   // 42
None.unwrap();       // PANIC
```

_Defined in `lang/std/result/optional.ks`._

#### function `unwrap`

```kestrel
public consuming func unwrap(or: consuming T) -> T
```

Returns the wrapped value or `default` when `None`. `default` is
always evaluated — use `unwrap(orElse:)` if computing it is
expensive.

##### Examples

```
Some(42).unwrap(or: 0);   // 42
None.unwrap(or: 0);       // 0
```

_Defined in `lang/std/result/optional.ks`._

#### function `unwrap`

```kestrel
public consuming func unwrap(orElse: () -> T) -> T
```

Like `unwrap(or:)`, but `defaultFn` is only called on `None`. Use this
when the default is expensive to compute or has side effects.

##### Examples

```
Some(42).unwrap(orElse: { expensiveDefault() });   // 42, no call
None.unwrap(orElse: { expensiveDefault() });       // calls fn
```

_Defined in `lang/std/result/optional.ks`._

#### function `wrap`

```kestrel
public static func wrap(T) -> Optional[T]
```

Wraps `value` in `.Some`. Rarely needed in practice — bare values
are promoted via `FromValue`, and `let x: Int64? = 42` does the
right thing.

##### Examples

```
let opt = Optional.wrap(42);   // Some(42)
let opt: Int64? = 42;                 // identical, preferred
```

_Defined in `lang/std/result/optional.ks`._

#### function `xor`

```kestrel
public consuming func xor(consuming Optional[T]) -> Optional[T]
```

Exclusive-or of presence — returns the unique `Some` when exactly
one of `self`/`other` is set, else `None`.

##### Examples

```
Some(1).xor(None);       // Some(1)
None.xor(Some(2));       // Some(2)
Some(1).xor(Some(2));    // None
None.xor(None);          // None
```

_Defined in `lang/std/result/optional.ks`._

#### function `zip`

```kestrel
public consuming func zip[U](with: consuming Optional[U]) -> Optional[(T, U)]
```

Pairs two optionals into an optional tuple. `Some` only when both
inputs are `Some`.

##### Examples

```
Some(1).zip(with: Some("a"));   // Some((1, "a"))
Some(1).zip(with: None);        // None
None.zip(with: Some("a"));      // None
```

_Defined in `lang/std/result/optional.ks`._

### Implements `Copyable`

_Conditional: `where T: Copyable`._

### Implements `Equatable`

_Conditional: `where T: Equatable`._

#### function `isEqual`

```kestrel
public func isEqual(to: Optional[T]) -> Bool where T: Equatable
```

Structural equality on the optional. Backs `==`.

##### Examples

```
Some(1) == Some(1);   // true
Some(1) == Some(2);   // false
Some(1) == None;      // false
None == None;         // true
```

_Defined in `lang/std/result/optional.ks`._

### Implements `Comparable`

_Conditional: `where T: Comparable`._

#### function `compare`

```kestrel
public func compare(Optional[T]) -> Ordering where T: Comparable
```

Three-way compare. `None < Some(_)`; two `Some`s defer to the
inner `compare`.

##### Examples

```
None < Some(1);     // true
Some(1) < Some(2);  // true
Some(2) < Some(1);  // false
```

_Defined in `lang/std/result/optional.ks`._

### Implements `Hashable`

_Conditional: `where T: Hashable`._

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher, T: Hashable
```

Mixes a one-byte tag (`0` for `None`, `1` for `Some`) into the
hasher, then defers to `T.hash` for the payload.

_Defined in `lang/std/result/optional.ks`._

### Implements `Tryable`

#### typealias `Output`

```kestrel
type Output = T
```

_Defined in `lang/std/result/optional.ks`._

#### typealias `Output`

```kestrel
type Output = T
```

_Defined in `lang/std/result/optional.ks`._

#### typealias `Output`

```kestrel
type Output = T
```

_Defined in `lang/std/result/optional.ks`._

#### typealias `Residual`

```kestrel
type Residual = ()
```

_Defined in `lang/std/result/optional.ks`._

#### function `tryExtract`

```kestrel
public consuming func tryExtract() -> ControlFlow[T, ()]
```

Drives `try` — `Continue(value)` for `Some`, `Break(())` for `None`.

_Defined in `lang/std/result/optional.ks`._

### Implements `ForceUnwrap`

#### function `forceUnwrap`

```kestrel
public consuming func forceUnwrap() -> T
```

Returns the wrapped value, trapping on `.None`. Backs `value!`.

_Defined in `lang/std/result/optional.ks`._

### Implements `FromResidual`

#### function `fromResidual`

```kestrel
public static func fromResidual(consuming ()) -> Optional[T]
```

Builds `.None` from the residual produced by a `try` short-circuit.

_Defined in `lang/std/result/optional.ks`._

### Implements `FromValue`

#### function `from`

```kestrel
public static func from(consuming T) -> Optional[T]
```

Wraps `value` in `.Some`. Called by the compiler at the promotion
site, not usually by user code. `consuming` so `value` is moved into
`.Some` (no clone-and-leak of a borrowed original).

_Defined in `lang/std/result/optional.ks`._

### Implements `Formattable`

_Conditional: `where T: Formattable`._

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions) where T: Formattable
```

Renders `Some(...)` or `None`, forwarding `options` to the inner
`format` for the payload.

_Defined in `lang/std/result/optional.ks`._

### Implements `ExpressibleByNullLiteral`

### Implements `Coalesce`

#### function `coalesce`

```kestrel
public consuming func coalesce(() -> T) -> T
```

Returns the wrapped value or evaluates `default()`. The default is
only invoked on `None`, which is what makes `??` cheap on the
happy path.

_Defined in `lang/std/result/optional.ks`._

## struct `OptionalIterator`

```kestrel
public struct OptionalIterator[T] where T: not Static { /* private fields */ }
```

_Defined in `lang/std/result/optional.ks`._

### Members

#### initializer `From Optional`

```kestrel
public init(consuming Optional[T])
```

Builds an iterator that will yield the contents of `value` on its
first `next()` call (or terminate immediately if `value` is `None`).

_Defined in `lang/std/result/optional.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/result/optional.ks`._

#### function `next`

```kestrel
public mutating func next() -> Optional[T]
```

Returns and clears the stored value, then returns `None` forever.
`O(1)` and allocation-free.

_Defined in `lang/std/result/optional.ks`._

## typealias `OptionalTypeOperator`

```kestrel
public type OptionalTypeOperator[T] = Optional[T]
```

Compiler hook — `T?` desugars to `Optional[T]` via this alias. End
users should write the sugar; this declaration exists so the operator
can resolve to a concrete type.

### Examples

```
var name: String? = null;            // same as Optional[String]
func find(id: Int64) -> User?;       // returns Optional[User]
let nested: Int64?? = null;          // Optional[Optional[Int64]]
```

_Defined in `lang/std/result/optional.ks`._

## enum `Result`

```kestrel
public enum Result[T, E]: not Copyable where T: not Static
```

_Defined in `lang/std/result/result.ks`._

### Members

#### case `Err`

```kestrel
case Err(E)
```

The failure branch — wraps an `E`.

_Defined in `lang/std/result/result.ks`._

#### case `Ok`

```kestrel
case Ok(T)
```

The success branch — wraps a `T`.

_Defined in `lang/std/result/result.ks`._

#### function `andThen`

```kestrel
public consuming func andThen[U]((T) -> Result[U, E]) -> Result[U, E]
```

Alias for `flatMap` — chains a fallible step onto an `Ok` branch.
Reads more naturally in long pipelines (`parseInput().andThen(validate).andThen(persist)`).

_Defined in `lang/std/result/result.ks`._

#### function `andValue`

```kestrel
public consuming func andValue[U](consuming Result[U, E]) -> Result[U, E]
```

Returns `other` when `self` is `Ok`, otherwise propagates the
existing `Err`. Named `andValue` (not `and`) because `and` is a
reserved keyword.

_Defined in `lang/std/result/result.ks`._

#### function `err`

```kestrel
public static func err(E) -> Result[T, E]
```

Wraps `error` in `.Err`. Useful when constructing a `Result` from
a known error in non-promotion contexts.

_Defined in `lang/std/result/result.ks`._

#### function `err`

```kestrel
public consuming func err() -> Optional[E]
```

Discards the success value, returning `Some(error)` for `.Err` and
`None` for `.Ok`. Mirror of `ok()`.

_Defined in `lang/std/result/result.ks`._

#### function `errRef`

```kestrel
public func errRef() -> Optional[&E]
```

Borrows the error value without consuming this result.

_Defined in `lang/std/result/result.ks`._

#### function `expect`

```kestrel
public consuming func expect(String) -> T
```

Like `unwrap`, but the panic carries `message` instead of the
generic text. Mirror of `Optional.expect(message:)` — use where a
failure should crash loudly with context.

##### Errors

Panics with `message` on `.Err` via `fatalError`.

##### Examples

```
let cfg = loadConfig().expect("Config file required");
```

_Defined in `lang/std/result/result.ks`._

#### function `flatMap`

```kestrel
public consuming func flatMap[U]((T) -> Result[U, E]) -> Result[U, E]
```

Monadic bind on the success branch — apply a transform that itself
returns a `Result`, without nesting.

_Defined in `lang/std/result/result.ks`._

#### function `flatMapErr`

```kestrel
public consuming func flatMapErr[F]((E) -> Result[T, F]) -> Result[T, F]
```

Monadic bind on the error branch — apply a recovery function that
itself returns a `Result`, without nesting. Mirror of `flatMap`.

_Defined in `lang/std/result/result.ks`._

#### function `flatten`

```kestrel
public consuming func flatten[U]() -> Result[U, E] where T == Result[U, E]
```

Collapses a `Result[Result[T, E], E]` one level. Available only
when the success payload is itself a `Result` with the same error
type. Mirror of `Optional.flatten()`.

##### Examples

```
Ok(Ok(42)).flatten();        // Ok(42)
Ok(Err("inner")).flatten();  // Err("inner")
Err("outer").flatten();      // Err("outer")
```

_Defined in `lang/std/result/result.ks`._

#### function `inspect`

```kestrel
public consuming func inspect((T) -> ()) -> Result[T, E]
```

Side-effecting tap on the success branch — runs `fn` on the `Ok`
value (if any) and returns `self` unchanged. Useful for logging or
assertions inside a chain. Mirror of `Optional.inspect(fn:)`.

##### Examples

```
parsePort(input)
    .inspect { print("port = \{it}") }
    .map { it + 1 };
```

_Defined in `lang/std/result/result.ks`._

#### function `inspectErr`

```kestrel
public consuming func inspectErr((E) -> ()) -> Result[T, E]
```

Side-effecting tap on the error branch — runs `fn` on the `Err`
value (if any) and returns `self` unchanged. Mirror of `inspect`
for the failure path.

##### Examples

```
parsePort(input)
    .inspectErr { print("failed: \{it}") }
    .mapErr { AppError.Parse(it) };
```

_Defined in `lang/std/result/result.ks`._

#### function `isErr`

```kestrel
public func isErr() -> Bool
```

True when this is `.Err`. Complement of `isOk`.

_Defined in `lang/std/result/result.ks`._

#### function `isErrAnd`

```kestrel
public func isErrAnd(where: (E) -> Bool) -> Bool
```

True when `.Err(error)` and `predicate(error)` returns `true`.
`.Ok` always answers `false` without invoking the predicate.
Complement of `isOkAnd(where:)`.

##### Examples

```
Err(404).isErrAnd(where: { it == 404 });   // true
Err(500).isErrAnd(where: { it == 404 });   // false
Ok(1).isErrAnd(where: { it == 404 });      // false
```

_Defined in `lang/std/result/result.ks`._

#### function `isOk`

```kestrel
public func isOk() -> Bool
```

True when this is `.Ok`. Cheap discriminator-only check.

##### Examples

```
Ok(42).isOk();          // true
Err("oops").isOk();     // false
```

_Defined in `lang/std/result/result.ks`._

#### function `isOkAnd`

```kestrel
public func isOkAnd(where: (T) -> Bool) -> Bool
```

True when `.Ok(value)` and `predicate(value)` returns `true`.
`.Err` always answers `false` without invoking the predicate.
Mirror of `Optional.isSomeAnd(where:)`.

##### Examples

```
Ok(42).isOkAnd(where: { it > 0 });      // true
Ok(-1).isOkAnd(where: { it > 0 });      // false
Err("x").isOkAnd(where: { it > 0 });    // false
```

_Defined in `lang/std/result/result.ks`._

#### function `iter`

```kestrel
public consuming func iter() -> ResultIterator[T, E]
```

Returns a `ResultIterator` yielding the success value (one element
for `.Ok`, zero for `.Err`). Lets a `Result` plug into iterator
pipelines that only care about the happy path.

_Defined in `lang/std/result/result.ks`._

#### function `map`

```kestrel
public consuming func map[U]((T) -> U) -> Result[U, E]
```

Functor map on the success branch. `.Err` passes through unchanged.

##### Examples

```
Ok(2).map { it * 2 };          // Ok(4)
Err("oops").map { it * 2 };    // Err("oops")
```

_Defined in `lang/std/result/result.ks`._

#### function `mapErr`

```kestrel
public consuming func mapErr[F]((E) -> F) -> Result[T, F]
```

Functor map on the error branch — typically used to widen a
specific error type into a more general one.

##### Examples

```
parse(s).mapErr { AppError.Parse(it) };
```

_Defined in `lang/std/result/result.ks`._

#### function `ok`

```kestrel
public static func ok(T) -> Result[T, E]
```

Wraps `value` in `.Ok`. Rarely needed in practice — `FromValue`
promotes bare values where the context expects a `Result`.

_Defined in `lang/std/result/result.ks`._

#### function `ok`

```kestrel
public consuming func ok() -> Optional[T]
```

Discards the error, returning `Some(value)` for `.Ok` and `None`
for `.Err`.

_Defined in `lang/std/result/result.ks`._

#### function `okRef`

```kestrel
public func okRef() -> Optional[&T]
```

Borrows the success value without consuming this result.

_Defined in `lang/std/result/result.ks`._

#### function `orElse`

```kestrel
public consuming func orElse[F]((E) -> Result[T, F]) -> Result[T, F]
```

Returns `self` when `Ok`, otherwise calls `alternative(error)`.
Use this for recovery logic that depends on which error occurred —
e.g. retrying on a transient error but bubbling a permanent one.

_Defined in `lang/std/result/result.ks`._

#### function `orValue`

```kestrel
public consuming func orValue(consuming Result[T, E]) -> Result[T, E]
```

Returns `self` when `Ok`, otherwise returns `other`. Named
`orValue` because `or` is a reserved keyword.

_Defined in `lang/std/result/result.ks`._

#### function `unwrap`

```kestrel
public consuming func unwrap() -> T
```

Returns the success value, panicking if `Err`. Use
`unwrap(or:)` or pattern matching unless you can prove the
result is `Ok`.

##### Errors

Panics with `"called unwrap() on Err"` when invoked on `.Err`.

_Defined in `lang/std/result/result.ks`._

#### function `unwrap`

```kestrel
public consuming func unwrap(or: consuming T) -> T
```

Returns the success value or `default` on `Err`. `default` is
always evaluated — use `unwrap(orElse:)` if computing it is
expensive or depends on the error.

_Defined in `lang/std/result/result.ks`._

#### function `unwrap`

```kestrel
public consuming func unwrap(orElse: (E) -> T) -> T
```

Like `unwrap(or:)`, but `defaultFn` receives the error value and is
only invoked on `Err`. Useful when the recovery value depends on
what went wrong.

_Defined in `lang/std/result/result.ks`._

#### function `unwrapErr`

```kestrel
public consuming func unwrapErr() -> E
```

Returns the error value, panicking if `Ok`. Mostly used in tests
to assert that a call failed.

##### Errors

Panics with `"called unwrapErr() on Ok"` when invoked on `.Ok`.

_Defined in `lang/std/result/result.ks`._

### Implements `Tryable`

#### typealias `Output`

```kestrel
type Output = T
```

_Defined in `lang/std/result/result.ks`._

#### typealias `Output`

```kestrel
type Output = T
```

_Defined in `lang/std/result/result.ks`._

#### typealias `Residual`

```kestrel
type Residual = E
```

_Defined in `lang/std/result/result.ks`._

#### function `tryExtract`

```kestrel
public consuming func tryExtract() -> ControlFlow[T, E]
```

Drives `try` — `Continue(value)` for `.Ok`, `Break(error)` for
`.Err`.

_Defined in `lang/std/result/result.ks`._

### Implements `Exitable`

_Conditional: `where T: Exitable, E: Formattable`._

#### function `report`

```kestrel
consuming func report() -> ExitCode where T: Exitable, E: Formattable
```

_Defined in `lang/std/os/exitable.ks`._

### Implements `Copyable`

_Conditional: `where T: Copyable, E: Copyable`._

### Implements `FromResidual`

#### function `fromResidual`

```kestrel
public static func fromResidual(consuming E) -> Result[T, E]
```

Builds `.Err(residual)` from the residual produced by a `try`
short-circuit.

_Defined in `lang/std/result/result.ks`._

### Implements `FromValue`

#### function `from`

```kestrel
public static func from(consuming T) -> Result[T, E]
```

Wraps `value` in `.Ok`. Called by the compiler at the promotion
site, not usually by user code. `consuming` so `value` is moved into
`.Ok` (no clone-and-leak of a borrowed original).

_Defined in `lang/std/result/result.ks`._

### Implements `Coalesce`

#### function `coalesce`

```kestrel
public consuming func coalesce(() -> T) -> T
```

Returns the `.Ok` value or evaluates `default()`. The default is
only invoked on `.Err`; the error value is dropped.

_Defined in `lang/std/result/result.ks`._

### Implements `Equatable`

_Conditional: `where T: Equatable, E: Equatable`._

#### function `isEqual`

```kestrel
public func isEqual(to: Result[T, E]) -> Bool where T: Equatable, E: Equatable
```

Structural equality on the result. Backs `==`.

##### Examples

```
Ok(1)       == Ok(1);        // true
Ok(1)       == Ok(2);        // false
Err("x")    == Err("x");     // true
Ok(1)       == Err("x");     // false
```

_Defined in `lang/std/result/result.ks`._

### Implements `Formattable`

_Conditional: `where T: Formattable, E: Formattable`._

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions) where T: Formattable, E: Formattable
```

Renders `Ok(...)` or `Err(...)`, forwarding `options` to the inner
`format` for the payload.

_Defined in `lang/std/result/result.ks`._

## struct `ResultIterator`

```kestrel
public struct ResultIterator[T, E] where T: not Static { /* private fields */ }
```

_Defined in `lang/std/result/result.ks`._

### Members

#### initializer `From Result`

```kestrel
public init(consuming Result[T, E])
```

Builds an iterator from a `Result`, projecting `.Ok` to a single
element and `.Err` to an empty stream.

_Defined in `lang/std/result/result.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/result/result.ks`._

#### function `next`

```kestrel
public mutating func next() -> Optional[T]
```

Returns and clears the stored value, then returns `None` forever.
`O(1)` and allocation-free.

_Defined in `lang/std/result/result.ks`._

## typealias `ResultTypeOperator`

```kestrel
public type ResultTypeOperator[T, E] = Result[T, E]
```

Compiler hook — `T throws E` desugars to `Result[T, E]` via this
alias. Write the sugar in user code; this exists so the operator can
resolve to a concrete type.

_Defined in `lang/std/result/result.ks`._

