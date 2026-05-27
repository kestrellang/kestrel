# std.result

## enum `Optional`

```kestrel
public enum Optional[T]
```

A type-safe stand-in for nullable references — either `Some(value)` or
`None`.

`T?` desugars to `Optional[T]`, and the `null` literal constructs
`.None` for any optional type. The compiler refuses to let you read the
inner value without handling the `None` case, which is the whole point
— there is no implicit unwrap. The `try` operator (and the `??`
coalescing operator) propagate `None` through call chains so you can
write linear code without nested `match` blocks.

### Examples

```
func find(id: Int64) -> User? {
    if let user = users.get(id) { return user };
    null
}

match find(42) {
    .Some(let u) => print(u.name),
    .None        => print("Not found")
}
```

```
// `try` short-circuits on None
func combine() -> Int64? {
    let a = try getA();   // returns None early if getA() is None
    let b = try getB();
    a + b
}
```

### Representation

A two-case tagged union — one byte (or whatever the backend picks) of
discriminant plus the payload of `T`. The compiler will use a niche
when one is available (e.g. a non-zero pointer), so `Optional[Pointer]`
is the same size as `Pointer`.

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
public func contains(T) -> Bool
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
public func expect(String) -> T
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
public func filter((T) -> Bool) -> Optional[T]
```

Returns `Some(value)` when the predicate accepts the value, `None`
otherwise.

##### Examples

```
Some(4).filter { it % 2 == 0 };   // Some(4)
Some(3).filter { it % 2 == 0 };   // None
None.filter { it % 2 == 0 };      // None
```

_Defined in `lang/std/result/optional.ks`._

#### function `flatMap`

```kestrel
public func flatMap[U]((T) -> Optional[U]) -> Optional[U]
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
public func flatten[U]() -> Optional[U] where T == Optional[U]
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
public func inspect((T) -> ()) -> Optional[T]
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
public func isSomeAnd((T) -> Bool) -> Bool
```

True when `.Some(value)` and `predicate(value)` returns `true`.
`None` always answers `false` without invoking the predicate.

##### Examples

```
Some(42).isSomeAnd { it > 0 };    // true
Some(-1).isSomeAnd { it > 0 };    // false
None.isSomeAnd { it > 0 };        // false
```

_Defined in `lang/std/result/optional.ks`._

#### function `iter`

```kestrel
public func iter() -> OptionalIterator[T]
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
public func map[U]((T) -> U) -> Optional[U]
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
public func okOr[E](E) -> Result[T, E]
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
public func okOrElse[E](() -> E) -> Result[T, E]
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
public func orElse(() -> Optional[T]) -> Optional[T]
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
public func then[U](Optional[U]) -> Optional[U]
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
public func unwrap() -> T
```

Returns the wrapped value, panicking if `None`. Reach for
`unwrapOr`, `unwrap(or:)`, the `??` operator, or pattern
matching unless you can prove the value is `Some`.

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
public func unwrap(or: () -> T) -> T
```

Like `unwrapOr`, but `defaultFn` is only called on `None`. Use this
when the default is expensive to compute or has side effects.

##### Examples

```
Some(42).unwrap(or: { expensiveDefault() });   // 42, no call
None.unwrap(or: { expensiveDefault() });       // calls fn
```

_Defined in `lang/std/result/optional.ks`._

#### function `unwrapOr`

```kestrel
public func unwrapOr(T) -> T
```

Returns the wrapped value or `default` when `None`. `default` is
always evaluated — use `unwrap(or:)` if computing it is
expensive.

##### Examples

```
Some(42).unwrapOr(0);   // 42
None.unwrapOr(0);       // 0
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
public func xor(Optional[T]) -> Optional[T]
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
public func zip[U](with: Optional[U]) -> Optional[(T, U)]
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

### Implements `Equatable`

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/protocols.ks`._

#### function `equal`

```kestrel
public func equal(to: Self) -> Bool
```

Bridges `Equal.equal(to:)` to `Equatable.isEqual(to:)`.

_Defined in `lang/std/core/protocols.ks`._

#### function `isEqual`

```kestrel
public func isEqual(to: Optional[T]) -> Bool
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

#### function `notEqual`

```kestrel
public func notEqual(to: Self) -> Bool
```

Default `!=`: delegates to `==` so there's a single source of truth.

_Defined in `lang/std/core/protocols.ks`._

### Implements `Comparable`

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/protocols.ks`._

#### function `compare`

```kestrel
public func compare(Optional[T]) -> Ordering
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

#### function `greaterThan`

```kestrel
public func greaterThan(Self) -> Bool
```

`>` derived from `compare`.

_Defined in `lang/std/core/protocols.ks`._

#### function `greaterThanOrEqual`

```kestrel
public func greaterThanOrEqual(Self) -> Bool
```

`>=` derived from `compare`.

_Defined in `lang/std/core/protocols.ks`._

#### function `isAtLeast`

```kestrel
public func isAtLeast(Self) -> Bool
```

`start..` lower-bound check, derived from `compare`.

_Defined in `lang/std/core/protocols.ks`._

#### function `isAtMost`

```kestrel
public func isAtMost(Self) -> Bool
```

`..=end` upper-bound check, derived from `compare`.

_Defined in `lang/std/core/protocols.ks`._

#### function `isBelow`

```kestrel
public func isBelow(Self) -> Bool
```

`..<end` upper-bound check, derived from `compare`.

_Defined in `lang/std/core/protocols.ks`._

#### function `lessThan`

```kestrel
public func lessThan(Self) -> Bool
```

`<` derived from `compare`.

_Defined in `lang/std/core/protocols.ks`._

#### function `lessThanOrEqual`

```kestrel
public func lessThanOrEqual(Self) -> Bool
```

`<=` derived from `compare`.

_Defined in `lang/std/core/protocols.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
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

### Implements `FromResidual`

#### function `fromResidual`

```kestrel
public static func fromResidual(()) -> Optional[T]
```

Builds `.None` from the residual produced by a `try` short-circuit.

_Defined in `lang/std/result/optional.ks`._

### Implements `FromValue`

#### function `from`

```kestrel
public static func from(T) -> Optional[T]
```

Wraps `value` in `.Some`. Called by the compiler at the promotion
site, not usually by user code.

_Defined in `lang/std/result/optional.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating StringBuilder, FormatOptions)
```

Renders `Some(...)` or `None`, forwarding `options` to the inner
`format` for the payload.

_Defined in `lang/std/result/optional.ks`._

#### function `formatted`

```kestrel
public func formatted(FormatOptions) -> String
```

Returns this value rendered as a `String`.

Convenience wrapper: creates a `StringBuilder`, calls
`format(into:)`, and returns the built string. Uses a distinct
name to avoid overload-resolution ambiguity with `format(into:)`.

_Defined in `lang/std/text/format.ks`._

### Implements `ExpressibleByNullLiteral`

#### initializer `Null Literal`

```kestrel
init()
```

Builds the absent/none instance.

_Defined in `lang/std/core/literals.ks`._

### Implements `Coalesce`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/coalesce.ks`._

#### function `coalesce`

```kestrel
public func coalesce(() -> T) -> T
```

Returns the wrapped value or evaluates `default()`. The default is
only invoked on `None`, which is what makes `??` cheap on the
happy path.

_Defined in `lang/std/result/optional.ks`._

## struct `OptionalIterator`

```kestrel
public struct OptionalIterator[T] { /* private fields */ }
```

Single-shot iterator yielding zero or one elements. Returned by
`Optional.iter()`.

### Examples

```
let items: [Int64] = Some(42).iter().collect();   // [42]
let empty: [Int64] = None.iter().collect();       // []
```

### Representation

One `Optional[T]` field. `next()` empties it on first call.

_Defined in `lang/std/result/optional.ks`._

### Members

#### initializer `From Optional`

```kestrel
public init(Optional[T])
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

#### typealias `TargetIterator`

```kestrel
type TargetIterator = Self
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `all`

```kestrel
public mutating func all(where: (Item) -> Bool) -> Bool
```

True if every element satisfies `predicate`. Stops at the first
failure. True for an empty iterator (vacuous truth).

##### Examples

```
[2, 4, 6].iter().all { it % 2 == 0 };   // true
[2, 3, 4].iter().all { it % 2 == 0 };   // false (stops at 3)
[].iter().all { false };                // true (empty)
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `any`

```kestrel
public mutating func any(where: (Item) -> Bool) -> Bool
```

True if any element satisfies `predicate`. Stops at the first
match. False for an empty iterator.

##### Examples

```
[1, 2, 3, 4].iter().any { it > 3 };    // true (stops at 4)
[1, 2, 3].iter().any { it > 10 };      // false
[].iter().any { true };                // false
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `chain`

```kestrel
public func chain[Other](Other) -> ChainIterator[Self, Other] where Other: Iterator, Other.Item == Item
```

Yields all of `self`, then all of `other`. Both must produce the
same `Item` type.

##### Examples

```
[1, 2].iter().chain([3, 4].iter()).collect();   // [1, 2, 3, 4]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `collect`

```kestrel
public consuming func collect() -> Array[Item]
```

Drains the iterator into an `Array[Item]`. Eager and `O(n)`. Use
at the end of an adapter chain to materialise the result.

##### Examples

```
[1, 2, 3].iter().filter { it > 1 }.collect();   // [2, 3]
(1..5).iter().map { it * it }.collect();        // [1, 4, 9, 16]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `compactMap`

```kestrel
public func compactMap[T]() -> FilterMapIterator[Self, T] where Item == Optional[T]
```

Drops `None`s and unwraps `Some`s — the identity-transform special
case of `filterMap`. Available when the iterator already yields
optionals.

##### Examples

```
let xs: [Int64?] = [.Some(1), .None, .Some(2), .None, .Some(3)];
xs.iter().compactMap().collect();   // [1, 2, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `contains`

```kestrel
public mutating func contains(Item) -> Bool
```

True if any element equals `element`. Short-circuits.

##### Examples

```
[1, 2, 3].iter().contains(2);   // true
[1, 2, 3].iter().contains(5);   // false
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `count`

```kestrel
public consuming func count() -> Int64
```

Counts the elements by walking the whole iterator. `O(n)` — for
types that already know their length, prefer
`ExactSizeIterator.remaining`.

##### Examples

```
[1, 2, 3, 4, 5].iter().filter { it % 2 == 0 }.count();   // 2
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `cycle`

```kestrel
public func cycle() -> CycleIterator[Self]
```

Restarts iteration from the beginning whenever the inner iterator
is exhausted, producing an infinite sequence. Always combine with
`take` (or another short-circuiting consumer) — otherwise the
result is unbounded.

##### Examples

```
[1, 2, 3].iter().cycle().take(7).collect();
// [1, 2, 3, 1, 2, 3, 1]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `enumerate`

```kestrel
public func enumerate() -> EnumerateIterator[Self]
```

Pairs each element with its zero-based position.

##### Examples

```
for (i, item) in arr.iter().enumerate() {
    print("Index \{i}: \{item}")
};
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `filter`

```kestrel
public func filter(where: (Item) -> Bool) -> FilterIterator[Self]
```

Yields only elements where `predicate` returns `true`. Lazy —
elements are tested as they're pulled.

##### Examples

```
[1, 2, 3, 4, 5].iter().filter { it % 2 == 0 }.collect();   // [2, 4]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `filterMap`

```kestrel
public func filterMap[U](as: (Item) -> U?) -> FilterMapIterator[Self, U]
```

Combined map + filter — `transform` returns `Optional[U]`; `None`
values are skipped. Use over `map(...).filter(...)` when the
transform itself decides whether the element belongs.

##### Examples

```
["1", "two", "3"].iter()
    .filterMap { Int64.parse(it) }
    .collect();   // [1, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `first`

```kestrel
public mutating func first(where: (Item) -> Bool) -> Item?
```

First element matching `predicate`, or `None`. Stops at the first
match.

##### Examples

```
[1, 2, 3, 4, 5].iter().first { it > 3 };   // Some(4)
[1, 2, 3].iter().first { it > 10 };        // None
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `firstIndex`

```kestrel
public mutating func firstIndex(where: (Item) -> Bool) -> Int64?
```

Index of the first element matching `predicate`, or `None`.

##### Examples

```
["a", "b", "c"].iter().firstIndex(where: { it == "b" });   // Some(1)
[1, 2, 3].iter().firstIndex(where: { it > 10 });           // None
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `flatMap`

```kestrel
public func flatMap[U](as: (Item) -> U) -> FlatMapIterator[Self, U] where U: Iterator
```

Maps each element to an iterator and concatenates the results.
The monadic bind for iterators.

##### Examples

```
[[1, 2], [3, 4], [5]].iter()
    .flatMap { it.iter() }
    .collect();   // [1, 2, 3, 4, 5]
```

```
// Conditional expand — drop odd, double even
[1, 2, 3].iter()
    .flatMap { if it % 2 == 0 { [it, it].iter() } else { [].iter() } }
    .collect();   // [2, 2]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `flatten`

```kestrel
public func flatten() -> FlattenIterator[Self]
```

Concatenates the inner iterators into one flat stream. Each inner
iterator is fully drained before moving to the next. The
already-have-iterators counterpart of `flatMap`.

##### Examples

```
let nested = [[1, 2], [3, 4], [5]].iter().map { it.iter() };
nested.flatten().collect();   // [1, 2, 3, 4, 5]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `fold`

```kestrel
public consuming func fold[Acc](from: Acc, by: (Acc, Item) -> Acc) -> Acc
```

Left fold — start at `initial` and walk left to right, applying
`combine(acc, element)`. Returns `initial` for an empty iterator.

##### Examples

```
[1, 2, 3, 4].iter().fold(from: 0) { (acc, x) in acc + x };   // 10
[1, 2, 3].iter().fold(from: 1) { (acc, x) in acc * x };      // 6
[].iter().fold(from: 42) { (acc, x) in acc + x };            // 42
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `forEach`

```kestrel
public consuming func forEach((Item) -> ())
```

Calls `action` on every element, discarding return values. Use
`tryForEach` if you need to short-circuit on failure.

##### Examples

```
[1, 2, 3].iter().forEach { print(it) };
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `fuse`

```kestrel
public func fuse() -> FusedIterator[Self]
```

Locks `None` once seen — protects against iterators that aren't
fused (i.e. that may produce more elements after returning `None`
once). After the first `None`, this adapter returns `None`
forever.

_Defined in `lang/std/iter/iterator.ks`._

#### function `inspect`

```kestrel
public func inspect((Item) -> ()) -> InspectIterator[Self]
```

Calls `inspector` on each element as it flows through, leaving
the value otherwise untouched. Useful for logging or
instrumenting an adapter chain mid-pipeline.

##### Examples

```
[1, 2, 3].iter()
    .inspect { print("before filter: \{it}") }
    .filter { it > 1 }
    .inspect { print("after filter: \{it}") }
    .collect();
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `intersperse`

```kestrel
public func intersperse(with: Item) -> IntersperseIterator[Self]
```

Inserts `separator` between consecutive elements. Empty inputs
stay empty; single-element inputs get no separator.

##### Examples

```
[1, 2, 3].iter().intersperse(with: 0).collect();
// [1, 0, 2, 0, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `intersperseWith`

```kestrel
public func intersperseWith(with: () -> Item) -> IntersperseWithIterator[Self]
```

Like `intersperse`, but builds each separator on demand by calling
`separator()`. Use when the separator is expensive or needs to
vary by call.

##### Examples

```
var counter = 0;
[1, 2, 3].iter()
    .intersperseWith { counter += 1; counter * 10 }
    .collect();   // [1, 10, 2, 20, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `isSorted`

```kestrel
public consuming func isSorted() -> Bool
```

True if elements come out in ascending order. True for empty or
single-element iterators (vacuous). Short-circuits on the first
out-of-order pair.

##### Examples

```
[1, 2, 3, 4, 5].iter().isSorted();   // true
[1, 3, 2, 4, 5].iter().isSorted();   // false
[1, 1, 2, 2, 3].iter().isSorted();   // true (equal allowed)
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `isSortedDescending`

```kestrel
public consuming func isSortedDescending() -> Bool
```

True if elements come out in descending order. Mirror of
`isSorted`.

_Defined in `lang/std/iter/iterator.ks`._

#### function `iter`

```kestrel
func iter() -> Self
```

Returns `self`. The blanket conformance pivot — iterators *are*
iterables.

_Defined in `lang/std/iter/iterator.ks`._

#### function `last`

```kestrel
public consuming func last() -> Item?
```

Last element, or `None` if empty. Consumes the entire iterator —
`O(n)` even for sequences whose last element is cheap to address
directly.

_Defined in `lang/std/iter/iterator.ks`._

#### function `map`

```kestrel
public func map[U](as: (Item) -> U) -> MapIterator[Self, U]
```

Applies `transform` to each element. Lazy — the function only
fires when the downstream pulls a value.

##### Examples

```
[1, 2, 3].iter().map { it * 2 }.collect();         // [2, 4, 6]
["hi", "yo"].iter().map { it.count }.collect();    // [2, 2]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `max`

```kestrel
public consuming func max() -> Item?
```

Largest element, or `None` for an empty iterator. Ties go to the
first occurrence.

_Defined in `lang/std/iter/iterator.ks`._

#### function `min`

```kestrel
public consuming func min() -> Item?
```

Smallest element, or `None` for an empty iterator. Ties go to the
first occurrence.

##### Examples

```
[3, 1, 4, 1, 5].iter().min();   // Some(1)
[].iter().min();                // None
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `next`

```kestrel
public mutating func next() -> Optional[T]
```

Returns and clears the stored value, then returns `None` forever.
`O(1)` and allocation-free.

_Defined in `lang/std/result/optional.ks`._

#### function `nth`

```kestrel
public mutating func nth(Int64) -> Item?
```

Returns the element at index `n` (zero-based), consuming
everything up to and including it. `None` if `n` is past the end.

##### Examples

```
[10, 20, 30, 40].iter().nth(2);   // Some(30)
[10, 20].iter().nth(5);           // None
[10, 20, 30].iter().nth(0);       // Some(10)
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `peekable`

```kestrel
public func peekable() -> PeekableIterator[Self]
```

Wraps `self` so you can look at the next element without
consuming it.

##### Examples

```
var it = [1, 2, 3].iter().peekable();
it.peek();   // Some(1) — no consumption
it.peek();   // Some(1) — still
it.next();   // Some(1) — now consumed
it.peek();   // Some(2)
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `product`

```kestrel
public consuming func product() -> Item
```

Product of every element. Returns `Item.one` for an empty
iterator.

##### Examples

```
[1, 2, 3, 4, 5].iter().product();   // 120
(1..=5).iter().product();           // 120  (5!)
[].iter().product();                // 1
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `reduce`

```kestrel
public consuming func reduce(by: (Item, Item) -> Item) -> Item?
```

Like `fold`, but seeds the accumulator with the first element
instead of taking an explicit `initial`. Returns `None` for an
empty iterator.

##### Examples

```
[1, 2, 3, 4].iter().reduce { (a, b) in a + b };   // Some(10)
[5].iter().reduce { (a, b) in a + b };            // Some(5)
[].iter().reduce { (a, b) in a + b };             // None
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `scan`

```kestrel
public func scan[Acc](from: Acc, by: (Acc, Item) -> Acc) -> ScanIterator[Self, Acc]
```

Like `fold`, but yields each intermediate accumulator value
instead of just the final one. Useful for prefix sums, running
products, and any "carry state along" pattern.

##### Examples

```
// Running sum
[1, 2, 3, 4].iter()
    .scan(from: 0) { (acc, x) in acc + x }
    .collect();   // [1, 3, 6, 10]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `skip`

```kestrel
public func skip(Int64) -> SkipIterator[Self]
```

Drops the first `count` elements, then yields the rest.

##### Examples

```
[1, 2, 3, 4, 5].iter().skip(2).collect();   // [3, 4, 5]
[1, 2].iter().skip(10).collect();           // []
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `skipWhile`

```kestrel
public func skipWhile(where: (Item) -> Bool) -> SkipWhileIterator[Self]
```

Drops elements while `predicate` is `true`, then yields *every*
remaining element (including ones that would also satisfy the
predicate). Mirror of `takeWhile`.

##### Examples

```
[1, 2, 3, 4, 1, 2].iter()
    .skipWhile { it < 3 }
    .collect();   // [3, 4, 1, 2]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `sorted`

```kestrel
public consuming func sorted() -> Array[Item]
```

Collects into an `Array[Item]`, sorted ascending. Eager and
`O(n log n)` — calls `Array.sort(by:)` after `collect()`.

##### Examples

```
[3, 1, 4, 1, 5].iter().sorted();                       // [1, 1, 3, 4, 5]
[3, 1, 2].iter().filter { it > 1 }.sorted();          // [2, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `stepBy`

```kestrel
public func stepBy(Int64) -> StepByIterator[Self]
```

Yields every `n`-th element, starting at the first. `n == 0` is
undefined (the adapter will spin forever).

##### Examples

```
[0, 1, 2, 3, 4, 5, 6].iter().stepBy(2).collect();   // [0, 2, 4, 6]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `sum`

```kestrel
public consuming func sum() -> Item
```

Sum of every element. Returns `Item.zero` for an empty iterator.

##### Examples

```
[1, 2, 3, 4, 5].iter().sum();    // 15
[1.5, 2.5, 3.0].iter().sum();    // 7.0
[].iter().sum();                 // 0
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `take`

```kestrel
public func take(Int64) -> TakeIterator[Self]
```

Yields at most the first `count` elements; stops early even if
more are available.

##### Examples

```
[1, 2, 3, 4, 5].iter().take(3).collect();   // [1, 2, 3]
[1, 2].iter().take(10).collect();           // [1, 2]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `takeWhile`

```kestrel
public func takeWhile(where: (Item) -> Bool) -> TakeWhileIterator[Self]
```

Yields elements until `predicate` first returns `false`, then
stops. The "first failing" element is *not* yielded.

##### Examples

```
[1, 2, 3, 4, 1, 2].iter()
    .takeWhile { it < 4 }
    .collect();   // [1, 2, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `tryFold`

```kestrel
public mutating func tryFold[Acc, E](from: Acc, by: (Acc, Item) -> Result[Acc, E]) -> Result[Acc, E]
```

Fold with early exit on `Err`. The combine returns `Result`; the
first `Err` halts iteration and is returned. If everything
succeeds, returns `Ok(final accumulator)`.

##### Examples

```
// Stop the moment a parse fails
["1", "2", "3"].iter()
    .tryFold(from: 0) { (acc, s) in
        match Int64.parse(s) {
            .Some(n) => .Ok(acc + n),
            .None    => .Err("parse error")
        }
    };   // Ok(6)

["1", "bad", "3"].iter()
    .tryFold(from: 0) { (acc, s) in
        match Int64.parse(s) {
            .Some(n) => .Ok(acc + n),
            .None    => .Err("parse error")
        }
    };   // Err("parse error")
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `tryForEach`

```kestrel
public mutating func tryForEach[E]((Item) -> Result[(), E]) -> Result[(), E]
```

`forEach` with early exit on `Err`. Mirror of `tryFold` for the
"do something with each element" shape.

##### Examples

```
files.iter().tryForEach { (path) in
    File.delete(path)   // Result[(), IoError]
};   // stops on first failure
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `unzip`

```kestrel
public consuming func unzip[A, B]() -> (Array[A], Array[B]) where Item == (A, B)
```

Splits an iterator of pairs into two parallel arrays. Inverse of
`zip`.

##### Examples

```
let pairs = [(1, "a"), (2, "b"), (3, "c")];
let (nums, strs) = pairs.iter().unzip();
// nums = [1, 2, 3], strs = ["a", "b", "c"]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `zip`

```kestrel
public func zip[Other](Other) -> ZipIterator[Self, Other] where Other: Iterator
```

Pairs elements from `self` and `other`. Stops as soon as either
side runs out.

##### Examples

```
let names = ["Alice", "Bob", "Charlie"];
let ages  = [30, 25, 35];
names.iter().zip(ages.iter()).collect();
// [("Alice", 30), ("Bob", 25), ("Charlie", 35)]
```

_Defined in `lang/std/iter/iterator.ks`._

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
public enum Result[T, E]
```

The fallible-operation enum: either `Ok(value)` or `Err(error)`. The
project's exception-free error story.

`T throws E` desugars to `Result[T, E]`, and the `try` operator
short-circuits on `Err` so failure propagation reads like normal
straight-line code. The compiler refuses to let you read the success
value without first handling the error case.

`Result` composes with `Optional` via `ok()` / `err()`, and with the
`?` operator via `Tryable`. Pick `Result` when callers should be able
to inspect *why* something failed; pick `Optional` when "absent" is the
only failure mode.

### Examples

```
func parseAndDouble(s: String) -> Int64 throws ParseError {
    let n = try Int64.parse(s).okOr(ParseError());
    n * 2
}

match parseAndDouble("21") {
    .Ok(let v)  => print("got \{v}"),
    .Err(let e) => print("failed: \{e}")
}
```

### Representation

A two-case tagged union — discriminant plus the larger of `T` / `E`.
Niche optimisation applies the same way it does to `Optional`.

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
public func andThen[U]((T) -> Result[U, E]) -> Result[U, E]
```

Alias for `flatMap` — chains a fallible step onto an `Ok` branch.
Reads more naturally in long pipelines (`parseInput().andThen(validate).andThen(persist)`).

_Defined in `lang/std/result/result.ks`._

#### function `andValue`

```kestrel
public func andValue[U](Result[U, E]) -> Result[U, E]
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
public func err() -> Optional[E]
```

Discards the success value, returning `Some(error)` for `.Err` and
`None` for `.Ok`. Mirror of `ok()`.

_Defined in `lang/std/result/result.ks`._

#### function `flatMap`

```kestrel
public func flatMap[U]((T) -> Result[U, E]) -> Result[U, E]
```

Monadic bind on the success branch — apply a transform that itself
returns a `Result`, without nesting.

_Defined in `lang/std/result/result.ks`._

#### function `flatMapErr`

```kestrel
public func flatMapErr[F]((E) -> Result[T, F]) -> Result[T, F]
```

Monadic bind on the error branch — apply a recovery function that
itself returns a `Result`, without nesting. Mirror of `flatMap`.

_Defined in `lang/std/result/result.ks`._

#### function `isErr`

```kestrel
public func isErr() -> Bool
```

True when this is `.Err`. Complement of `isOk`.

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

#### function `iter`

```kestrel
public func iter() -> ResultIterator[T, E]
```

Returns a `ResultIterator` yielding the success value (one element
for `.Ok`, zero for `.Err`). Lets a `Result` plug into iterator
pipelines that only care about the happy path.

_Defined in `lang/std/result/result.ks`._

#### function `map`

```kestrel
public func map[U]((T) -> U) -> Result[U, E]
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
public func mapErr[F]((E) -> F) -> Result[T, F]
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
public func ok() -> Optional[T]
```

Discards the error, returning `Some(value)` for `.Ok` and `None`
for `.Err`.

_Defined in `lang/std/result/result.ks`._

#### function `orElse`

```kestrel
public func orElse[F]((E) -> Result[T, F]) -> Result[T, F]
```

Returns `self` when `Ok`, otherwise calls `alternative(error)`.
Use this for recovery logic that depends on which error occurred —
e.g. retrying on a transient error but bubbling a permanent one.

_Defined in `lang/std/result/result.ks`._

#### function `orValue`

```kestrel
public func orValue(Result[T, E]) -> Result[T, E]
```

Returns `self` when `Ok`, otherwise returns `other`. Named
`orValue` because `or` is a reserved keyword.

_Defined in `lang/std/result/result.ks`._

#### function `unwrap`

```kestrel
public func unwrap() -> T
```

Returns the success value, panicking if `Err`. Use `unwrapOr`,
`unwrap(or:)`, or pattern matching unless you can prove the
result is `Ok`.

##### Errors

Panics with `"called unwrap() on Err"` when invoked on `.Err`.

_Defined in `lang/std/result/result.ks`._

#### function `unwrap`

```kestrel
public func unwrap(or: (E) -> T) -> T
```

Like `unwrapOr`, but `defaultFn` receives the error value and is
only invoked on `Err`. Useful when the recovery value depends on
what went wrong.

_Defined in `lang/std/result/result.ks`._

#### function `unwrapErr`

```kestrel
public func unwrapErr() -> E
```

Returns the error value, panicking if `Ok`. Mostly used in tests
to assert that a call failed.

##### Errors

Panics with `"called unwrapErr() on Ok"` when invoked on `.Ok`.

_Defined in `lang/std/result/result.ks`._

#### function `unwrapOr`

```kestrel
public func unwrapOr(T) -> T
```

Returns the success value or `default` on `Err`. `default` is
always evaluated — use `unwrap(or:)` if computing it is
expensive or depends on the error.

_Defined in `lang/std/result/result.ks`._

### Implements `Tryable`

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
`.Err`. Defined inline because `Tryable` is declared in the enum's
conformance list above.

_Defined in `lang/std/result/result.ks`._

### Implements `FromResidual`

#### function `fromResidual`

```kestrel
public static func fromResidual(E) -> Result[T, E]
```

Builds `.Err(residual)` from the residual produced by a `try`
short-circuit.

_Defined in `lang/std/result/result.ks`._

### Implements `FromValue`

#### function `from`

```kestrel
public static func from(T) -> Result[T, E]
```

Wraps `value` in `.Ok`. Called by the compiler at the promotion
site, not usually by user code.

_Defined in `lang/std/result/result.ks`._

### Implements `Equatable`

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/protocols.ks`._

#### function `equal`

```kestrel
public func equal(to: Self) -> Bool
```

Bridges `Equal.equal(to:)` to `Equatable.isEqual(to:)`.

_Defined in `lang/std/core/protocols.ks`._

#### function `isEqual`

```kestrel
public func isEqual(to: Result[T, E]) -> Bool
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

#### function `notEqual`

```kestrel
public func notEqual(to: Self) -> Bool
```

Default `!=`: delegates to `==` so there's a single source of truth.

_Defined in `lang/std/core/protocols.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating StringBuilder, FormatOptions)
```

Renders `Ok(...)` or `Err(...)`, forwarding `options` to the inner
`format` for the payload.

_Defined in `lang/std/result/result.ks`._

#### function `formatted`

```kestrel
public func formatted(FormatOptions) -> String
```

Returns this value rendered as a `String`.

Convenience wrapper: creates a `StringBuilder`, calls
`format(into:)`, and returns the built string. Uses a distinct
name to avoid overload-resolution ambiguity with `format(into:)`.

_Defined in `lang/std/text/format.ks`._

## struct `ResultIterator`

```kestrel
public struct ResultIterator[T, E] { /* private fields */ }
```

Single-shot iterator yielding zero or one elements (the `Ok` value).
Returned by `Result.iter()`. Errors are silently skipped — use
`mapErr` / `match` if you need them.

### Representation

Stores the success value in an `Optional[T]` field; `next()` empties
it on first call.

_Defined in `lang/std/result/result.ks`._

### Members

#### initializer `From Result`

```kestrel
public init(Result[T, E])
```

Builds an iterator from a `Result`, projecting `.Ok` to a single
element and `.Err` to an empty stream.

_Defined in `lang/std/result/result.ks`._

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

