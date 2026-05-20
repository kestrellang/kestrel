# std.core

## protocol `AddAssign`

```kestrel
public protocol AddAssign[Other = Self]
```

Raw protocol backing the `+=` operator.

In-place mutation lets conforming types avoid the temporary that a
`self = self + other` rewrite would produce — important for collections
(e.g. `Array += other`) and other types where the binary `+` would copy.

_Defined in `lang/std/core/assign.ks`._

### Members

#### function `addAssign`

```kestrel
mutating func addAssign(Other)
```

Mutates `self` to `self + other`.

_Defined in `lang/std/core/assign.ks`._

## protocol `Addable`

```kestrel
public protocol Addable[Other = Self]
```

Raw protocol backing the `+` operator.

`Output` may differ from `Self` and `Other` — this is what allows mixed-type
arithmetic (e.g. `Vector + Scalar -> Vector`) without losing precision.
The associated `zero` value gives sums (and `Iterator.sum`) a starting
point and is the additive identity by definition.

### Examples

```
2 + 3            // 5
Int64.zero       // 0
```

_Defined in `lang/std/core/arithmetic.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `add`

```kestrel
func add(Other) -> Output
```

Returns `self + other`.

_Defined in `lang/std/core/arithmetic.ks`._

#### field `zero`

```kestrel
static var zero: Self { get }
```

The additive identity — a value `z` such that `x + z == x` for all `x`.

_Defined in `lang/std/core/arithmetic.ks`._

## protocol `And`

```kestrel
public protocol And[Other = Self]
```

Raw protocol backing the `and` keyword operator.

The `other` operand is a thunk so that conformers can short-circuit:
the right-hand side must not be evaluated when `self` is falsy. The
stdlib implementations on `Bool` and the optional types all honour
this; user implementations should too.

### Examples

```
true and false        // false
true and { true }     // true (closure form, mostly internal)
```

_Defined in `lang/std/core/logical.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/logical.ks`._

#### function `logicalAnd`

```kestrel
func logicalAnd(() -> Other) -> Output
```

Returns `self and other()`. The closure runs only if needed.

_Defined in `lang/std/core/logical.ks`._

## typealias `ArrayLiteralType`

```kestrel
public type ArrayLiteralType[T] = std.collections.Array[T]
```

_Defined in `lang/std/core/literals.ks`._

## protocol `ArrayMatchable`

```kestrel
public protocol ArrayMatchable
```

Protocol enabling array patterns (`[a, b]`, `[a, ..rest]`,
`[a, .., z]`, `[a, ..rest, z]`).

The compiler routes match-arm element access through `matchGet` and
rest-binding through `matchSlice` — they take `Int64` bounds the
compiler has already verified. A conformer may assume `0 <= index <
matchLength()` and `0 <= from <= to <= matchLength()` and skip its
own bounds checks; the conformance is unsafe to satisfy if those
invariants don't hold. `Array[T]` and `ArraySlice[T]` are the canonical
conformers.

_Defined in `lang/std/core/protocols.ks`._

### Members

#### typealias `Element`

```kestrel
type Element
```

_Defined in `lang/std/core/protocols.ks`._

#### function `matchGet`

```kestrel
func matchGet(Int64) -> Element
```

Returns the element at `index`. Caller (the compiler) guarantees
`0 <= index < matchLength()` — implementations may skip bounds checks.

_Defined in `lang/std/core/protocols.ks`._

#### function `matchLength`

```kestrel
func matchLength() -> Int64
```

Total number of elements available to match.

_Defined in `lang/std/core/protocols.ks`._

#### function `matchSlice`

```kestrel
func matchSlice(Int64, Int64) -> ArraySlice[Element]
```

Returns the slice `[from, to)`. Caller guarantees
`0 <= from <= to <= matchLength()`.

_Defined in `lang/std/core/protocols.ks`._

## protocol `BitwiseAnd`

```kestrel
public protocol BitwiseAnd[Other = Self]
```

Raw protocol backing the `&` operator.

Implemented by every integer width; `Output` is `Self` for the standard
integer types but may differ for SIMD or bitset wrappers.

### Examples

```
0b1100 & 0b1010   // 0b1000
```

_Defined in `lang/std/core/bitwise.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseAnd`

```kestrel
func bitwiseAnd(Other) -> Output
```

Returns `self & other`.

_Defined in `lang/std/core/bitwise.ks`._

## protocol `BitwiseAndAssign`

```kestrel
public protocol BitwiseAndAssign[Other = Self]
```

Raw protocol backing the `&=` operator.

_Defined in `lang/std/core/assign.ks`._

### Members

#### function `bitwiseAndAssign`

```kestrel
mutating func bitwiseAndAssign(Other)
```

Mutates `self` to `self & other`.

_Defined in `lang/std/core/assign.ks`._

## protocol `BitwiseNot`

```kestrel
public protocol BitwiseNot
```

Raw protocol backing the unary `~` operator.

_Defined in `lang/std/core/bitwise.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseNot`

```kestrel
func bitwiseNot() -> Output
```

Returns `~self` — every bit flipped.

_Defined in `lang/std/core/bitwise.ks`._

## protocol `BitwiseOr`

```kestrel
public protocol BitwiseOr[Other = Self]
```

Raw protocol backing the `|` operator.

_Defined in `lang/std/core/bitwise.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseOr`

```kestrel
func bitwiseOr(Other) -> Output
```

Returns `self | other`.

_Defined in `lang/std/core/bitwise.ks`._

## protocol `BitwiseOrAssign`

```kestrel
public protocol BitwiseOrAssign[Other = Self]
```

Raw protocol backing the `|=` operator.

_Defined in `lang/std/core/assign.ks`._

### Members

#### function `bitwiseOrAssign`

```kestrel
mutating func bitwiseOrAssign(Other)
```

Mutates `self` to `self | other`.

_Defined in `lang/std/core/assign.ks`._

## protocol `BitwiseXor`

```kestrel
public protocol BitwiseXor[Other = Self]
```

Raw protocol backing the `^` operator.

_Defined in `lang/std/core/bitwise.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `bitwiseXor`

```kestrel
func bitwiseXor(Other) -> Output
```

Returns `self ^ other`.

_Defined in `lang/std/core/bitwise.ks`._

## protocol `BitwiseXorAssign`

```kestrel
public protocol BitwiseXorAssign[Other = Self]
```

Raw protocol backing the `^=` operator.

_Defined in `lang/std/core/assign.ks`._

### Members

#### function `bitwiseXorAssign`

```kestrel
mutating func bitwiseXorAssign(Other)
```

Mutates `self` to `self ^ other`.

_Defined in `lang/std/core/assign.ks`._

## struct `Bool`

```kestrel
public struct Bool { /* private fields */ }
```

Two-state truth value with `true` and `false` as its only inhabitants.

`Bool` is the canonical conformer of every logical, conditional, and
equality protocol in `std.core`: equality, matching, hashing, formatting,
`and`/`or`/`not`, plus FFI compatibility for crossing the C boundary as
a single byte. Custom types rarely need to wrap `Bool`; conform to the
individual protocols (e.g. `BooleanConditional`) instead.

### Examples

```
let alive = true;
if alive { greet() }

let votes = [true, false, true];
let yesCount = votes.iter().filter { it }.count();   // 2
```

### Representation

Wraps a single `lang.i1`. The runtime promotes to a byte at FFI
boundaries (`FFISafe` conformance).

_Defined in `lang/std/core/bool.ks`._

### Members

#### initializer `Bool Literal`

```kestrel
public init(boolLiteral: lang.i1)
```

Builds a `Bool` from the primitive `lang.i1` produced by a literal.

_Defined in `lang/std/core/bool.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Bool) -> Bool
```

Returns `true` if both bits agree. Drives `==` for `Bool`.

_Defined in `lang/std/core/bool.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(Bool) -> Bool
```

Pattern-match form of `isEqual`: `case true =>` and `case false =>`
dispatch through here.

_Defined in `lang/std/core/bool.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating StringBuilder, FormatOptions)
```

Renders as `"true"` / `"false"`. With `options.debug`, wraps as
`"Bool(true)"` / `"Bool(false)"` for diagnostic dumps.

##### Examples

```
true.format()                                       // "true"
false.format(FormatOptions.debug())                 // "Bool(false)"
```

_Defined in `lang/std/core/bool.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Feeds a single `0` or `1` byte into `hasher`. Compatible with how the
stdlib hashes other primitives — equal `Bool`s always hash equal.

_Defined in `lang/std/core/bool.ks`._

### Implements `And`

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/bool.ks`._

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/bool.ks`._

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/bool.ks`._

#### function `logicalAnd`

```kestrel
public func logicalAnd(() -> Bool) -> Bool
```

Short-circuiting `and`: `other` runs only when `self` is `true`.
The closure form is what the `and` keyword lowers into; users
typically write `a and b` rather than calling this directly.

_Defined in `lang/std/core/bool.ks`._

### Implements `Or`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/logical.ks`._

#### function `logicalOr`

```kestrel
public func logicalOr(() -> Bool) -> Bool
```

Short-circuiting `or`: `other` runs only when `self` is `false`.

_Defined in `lang/std/core/bool.ks`._

### Implements `Not`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/logical.ks`._

#### function `logicalNot`

```kestrel
public func logicalNot() -> Bool
```

Bit-flip; `not true == false`.

_Defined in `lang/std/core/bool.ks`._

### Implements `ExpressibleByBoolLiteral`

#### initializer `Bool Literal`

```kestrel
init(boolLiteral: lang.i1)
```

Builds an instance from a boolean literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `BooleanConditional`

#### function `boolValue`

```kestrel
public func boolValue() -> lang.i1
```

Returns the wrapped `lang.i1` so `if`/`while` can branch on it
without a redundant `Bool` round-trip.

_Defined in `lang/std/core/bool.ks`._

## protocol `BooleanConditional`

```kestrel
public protocol BooleanConditional
```

Protocol for types that may appear directly in `if`, `while`, and other
boolean contexts.

`Bool` is the canonical conformer. The method returns the primitive
`lang.i1` rather than `Bool` to avoid a circular dependency between the
conditional lowering and `Bool` itself.

_Defined in `lang/std/core/logical.ks`._

### Members

#### function `boolValue`

```kestrel
func boolValue() -> lang.i1
```

Returns the underlying truth value as a primitive `i1`.

_Defined in `lang/std/core/logical.ks`._

## typealias `BooleanLiteralType`

```kestrel
public type BooleanLiteralType = Bool
```

Default type for boolean literals (`let b = true` → `Bool`).

_Defined in `lang/std/core/literals.ks`._

## typealias `CharLiteralType`

```kestrel
public type CharLiteralType = Char
```

Default type for character literals (`let c = 'a'` → `Char`).

_Defined in `lang/std/core/literals.ks`._

## protocol `Cloneable`

```kestrel
public protocol Cloneable
```

Protocol for types that need custom logic when duplicated.

`Cloneable` extends `Copyable` so that cloneable values can flow through
generic code that asks only for `Copyable`. The compiler invokes
`clone()` automatically wherever a `Cloneable` value would otherwise be
implicitly copied (assignment, argument pass, return). The implementation
decides how deep the copy goes — `RcBox`, for example, only bumps the
refcount.

### Examples

```
let a = RcBox(value: 1);
let b = a;            // implicit clone() — refcount bumps to 2
let c = a.clone();    // explicit clone — refcount bumps to 3
```

_Defined in `lang/std/core/copy.ks`._

### Members

#### function `clone`

```kestrel
func clone() -> Self
```

Returns a copy of `self`. Conformers define the depth and any side
effects (e.g. refcount adjustments).

_Defined in `lang/std/core/copy.ks`._

## struct `ClosedRange`

```kestrel
public struct ClosedRange[T] where T: Steppable, T: Comparable { /* private fields */ }
```

Closed range `[start, end]` — produced by the `..=` operator. Both
endpoints are included in iteration.

### Examples

```
for i in 0..=3 { print(i) }   // 0, 1, 2, 3
(0..=10).contains(10)         // true (vs Range, which excludes the upper)
```

### Representation

Two values: `start` and `end`. No heap allocation.

_Defined in `lang/std/core/range.ks`._

### Members

#### initializer `From Bounds`

```kestrel
public init(T, T)
```

Builds the closed range `[start, end]`.

_Defined in `lang/std/core/range.ks`._

#### function `contains`

```kestrel
public func contains(T) -> Bool
```

Returns `true` iff `start <= value <= end`.

_Defined in `lang/std/core/range.ks`._

#### field `end`

```kestrel
public var end: T
```

Upper bound — included.

_Defined in `lang/std/core/range.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` when `start > end` (no values are produced).

_Defined in `lang/std/core/range.ks`._

#### field `start`

```kestrel
public var start: T
```

Lower bound — included.

_Defined in `lang/std/core/range.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: ClosedRange[T]) -> Bool
```

Equal when both bounds match.

_Defined in `lang/std/core/range.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/core/range.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = ClosedRangeIterator[T]
```

_Defined in `lang/std/core/range.ks`._

#### function `iter`

```kestrel
public func iter() -> ClosedRangeIterator[T]
```

Returns a fresh iterator over the range.

_Defined in `lang/std/core/range.ks`._

### Implements `SeqIndex`

#### typealias `SeqOutput`

```kestrel
type SeqOutput = ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeq`

```kestrel
public func readSeq(from: ArraySlice[T]) -> ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeqChecked`

```kestrel
public func readSeqChecked(from: ArraySlice[T]) -> ArraySlice[T]?
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeqUnchecked`

```kestrel
public func readSeqUnchecked(from: ArraySlice[T]) -> ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `writeSeq`

```kestrel
public func writeSeq(to: ArraySlice[T], with: ArraySlice[T])
```

_Defined in `lang/std/collections/slice.ks`._

#### function `writeSeqUnchecked`

```kestrel
public func writeSeqUnchecked(to: ArraySlice[T], with: ArraySlice[T])
```

_Defined in `lang/std/collections/slice.ks`._

### Implements `SeqRange`

#### function `resolve`

```kestrel
public func resolve(Int64) -> Range[Int64]
```

_Defined in `lang/std/collections/slice.ks`._

### Implements `BytesIndex`

#### typealias `BytesYield`

```kestrel
type BytesYield = BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytes`

```kestrel
public func readBytes(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesChecked`

```kestrel
public func readBytesChecked(from: BytesView) -> BytesView?
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesUnchecked`

```kestrel
public func readBytesUnchecked(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesClampable`

#### typealias `BytesClampedYield`

```kestrel
type BytesClampedYield = BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesClamped`

```kestrel
public func readBytesClamped(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesWrappable`

#### typealias `BytesWrappedYield`

```kestrel
type BytesWrappedYield = BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesWrapped`

```kestrel
public func readBytesWrapped(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesSubstringIndex`

#### function `readBytesSubstring`

```kestrel
public func readBytesSubstring(from: BytesView) -> String
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsIndex`

#### typealias `CharsYield`

```kestrel
type CharsYield = CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readChars`

```kestrel
public func readChars(from: CharsView) -> CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsChecked`

```kestrel
public func readCharsChecked(from: CharsView) -> CharsView?
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsClampable`

#### typealias `CharsClampedYield`

```kestrel
type CharsClampedYield = CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsClamped`

```kestrel
public func readCharsClamped(from: CharsView) -> CharsView
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsWrappable`

#### typealias `CharsWrappedYield`

```kestrel
type CharsWrappedYield = CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsWrapped`

```kestrel
public func readCharsWrapped(from: CharsView) -> CharsView
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsSubstringIndex`

#### function `readCharsSubstring`

```kestrel
public func readCharsSubstring(from: CharsView) -> String
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesIndex`

#### typealias `GraphemesYield`

```kestrel
type GraphemesYield = GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemes`

```kestrel
public func readGraphemes(from: GraphemesView) -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesChecked`

```kestrel
public func readGraphemesChecked(from: GraphemesView) -> GraphemesView?
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesClampable`

#### typealias `GraphemesClampedYield`

```kestrel
type GraphemesClampedYield = GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesClamped`

```kestrel
public func readGraphemesClamped(from: GraphemesView) -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesWrappable`

#### typealias `GraphemesWrappedYield`

```kestrel
type GraphemesWrappedYield = GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesWrapped`

```kestrel
public func readGraphemesWrapped(from: GraphemesView) -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesSubstringIndex`

#### function `readGraphemesSubstring`

```kestrel
public func readGraphemesSubstring(from: GraphemesView) -> String
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesIndex`

#### typealias `LinesYield`

```kestrel
type LinesYield = LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLines`

```kestrel
public func readLines(from: LinesView) -> LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesChecked`

```kestrel
public func readLinesChecked(from: LinesView) -> LinesView?
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesClampable`

#### typealias `LinesClampedYield`

```kestrel
type LinesClampedYield = LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesClamped`

```kestrel
public func readLinesClamped(from: LinesView) -> LinesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesWrappable`

#### typealias `LinesWrappedYield`

```kestrel
type LinesWrappedYield = LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesWrapped`

```kestrel
public func readLinesWrapped(from: LinesView) -> LinesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesSubstringIndex`

#### function `readLinesSubstring`

```kestrel
public func readLinesSubstring(from: LinesView) -> String
```

_Defined in `lang/std/text/views.ks`._

## protocol `ClosedRangeConstructible`

```kestrel
public protocol ClosedRangeConstructible[Other = Self]
```

Raw protocol backing the closed `..=` operator (`start..=end`).

_Defined in `lang/std/core/range.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `inclusiveRange`

```kestrel
func inclusiveRange(to: Other) -> Output
```

Builds the closed range `[self, end]`.

_Defined in `lang/std/core/range.ks`._

## struct `ClosedRangeIterator`

```kestrel
public struct ClosedRangeIterator[T] where T: Steppable, T: Comparable { /* private fields */ }
```

Iterator over a `ClosedRange[T]`. Differs from `RangeIterator` in
that it yields `end` and uses an extra `finished` bit so it can
terminate after emitting the upper bound.

### Representation

`current`, `end`, and a one-bit `finished` flag.

_Defined in `lang/std/core/range.ks`._

### Members

#### initializer `From Bounds`

```kestrel
public init(current: T, end: T, finished: Bool)
```

Builds an iterator yielding `current` through `end` inclusive.
Pass `finished: true` to construct an already-exhausted iterator.

_Defined in `lang/std/core/range.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/core/range.ks`._

#### function `next`

```kestrel
public mutating func next() -> T?
```

Yields the next value, or `.None` when past `end`.

_Defined in `lang/std/core/range.ks`._

## protocol `Coalesce`

```kestrel
public protocol Coalesce[Default]
```

Raw protocol backing the `??` operator.

Implemented by `Optional[T]` (with `Default = T`, `Output = T`) and by
`Result[T, E]` (with `Default = T`, `Output = T`). The operand is a
thunk so the default expression is only evaluated when needed — this
matters when the default has side effects or is expensive to compute.

### Examples

```
let name: String? = .None;
name ?? "anonymous"           // "anonymous"

let cached: String? = .Some("hi");
cached ?? expensiveLookup()   // "hi" — expensiveLookup() not called
```

_Defined in `lang/std/core/coalesce.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/coalesce.ks`._

#### function `coalesce`

```kestrel
func coalesce(() -> Default) -> Output
```

Returns the contained value, or the result of `default()` if absent.

_Defined in `lang/std/core/coalesce.ks`._

## protocol `Comparable`

```kestrel
public protocol Comparable
```

Protocol for types with a total ordering.

Conformers implement a single `compare(other:) -> Ordering`; the
blanket extension below derives `<`, `<=`, `>`, `>=`, and `!=` (the
last shadowing the `Equatable` default since it can be cheaper via
`compare`). `Comparable` extends `Equatable`, so equal values and a
`compare` returning `.Equal` must agree.

### Examples

```
public struct Version: Comparable {
    public var major: Int64
    public var minor: Int64
    public func isEqual(to other: Version) -> Bool {
        self.major == other.major and self.minor == other.minor
    }
    public func compare(other: Version) -> Ordering {
        self.major.compare(other.major)
            .then(self.minor.compare(other.minor))
    }
}
```

_Defined in `lang/std/core/protocols.ks`._

### Members

#### function `compare`

```kestrel
func compare(Self) -> Ordering
```

Returns the ordering of `self` relative to `other`. Must be a
total order — for any `a`, `b`, `c` exactly one of `Less`,
`Equal`, `Greater` holds, and the order is transitive.

_Defined in `lang/std/core/protocols.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
func isEqual(to: Self) -> Bool
```

Returns `true` iff `self` and `other` are considered equal. Should
be reflexive, symmetric, and transitive — `Hashable` requires equal
values to hash equal, so don't drift from those laws.

_Defined in `lang/std/core/protocols.ks`._

### Implements `Less`

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/protocols.ks`._

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/protocols.ks`._

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/protocols.ks`._

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/protocols.ks`._

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/protocols.ks`._

#### function `lessThan`

```kestrel
public func lessThan(Self) -> Bool
```

`<` derived from `compare`.

_Defined in `lang/std/core/protocols.ks`._

### Implements `LessOrEqual`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/comparison.ks`._

#### function `lessThanOrEqual`

```kestrel
public func lessThanOrEqual(Self) -> Bool
```

`<=` derived from `compare`.

_Defined in `lang/std/core/protocols.ks`._

### Implements `Greater`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/comparison.ks`._

#### function `greaterThan`

```kestrel
public func greaterThan(Self) -> Bool
```

`>` derived from `compare`.

_Defined in `lang/std/core/protocols.ks`._

### Implements `GreaterOrEqual`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/comparison.ks`._

#### function `greaterThanOrEqual`

```kestrel
public func greaterThanOrEqual(Self) -> Bool
```

`>=` derived from `compare`.

_Defined in `lang/std/core/protocols.ks`._

### Implements `NotEqual`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/comparison.ks`._

#### function `isNotEqual`

```kestrel
public func isNotEqual(to: Self) -> Bool
```

`!=` derived from `compare`. Shadows the `Equatable` default with
a single dispatch.

_Defined in `lang/std/core/protocols.ks`._

### Implements `RangeMatchable`

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

## enum `ControlFlow`

```kestrel
public enum ControlFlow[C, B]
```

The two-state result of a `tryExtract()` call: keep going with a value, or
short-circuit out of the current function with an early-return payload.

Conceptually `Either`-shaped, but the names are deliberately
control-flow flavoured because that is what the compiler does with
them — `Continue` flows to the next instruction, `Break` lowers into a
branch back to the function's epilogue via `FromResidual`.

_Defined in `lang/std/core/error.ks`._

### Members

#### case `Break`

```kestrel
case Break(B)
```

Residual-return flow — carries the residual to propagate via `FromResidual`.

_Defined in `lang/std/core/error.ks`._

#### case `Continue`

```kestrel
case Continue(C)
```

Normal flow — carries the value to use as the operator result.

_Defined in `lang/std/core/error.ks`._

## protocol `Convertible`

```kestrel
public protocol Convertible[From]
```

Protocol for explicit type conversions via `init(from:)`.

Conform when you want callers to write `Target(from: source)`. Most
numeric types do this for every other numeric width (see
`lang/std/num/integer.ks.template`). Conformances should be lossless or
document their loss behavior; for fallible conversions prefer a separate
`Result`-returning function.

### Examples

```
let i: Int64 = 42;
let u = UInt32(from: i);   // explicit narrowing conversion
```

_Defined in `lang/std/core/convertible.ks`._

### Members

#### initializer `From Source`

```kestrel
init(from: From)
```

Creates an instance from `value`.

_Defined in `lang/std/core/convertible.ks`._

## protocol `Copyable`

```kestrel
public protocol Copyable
```

Marker protocol for types whose values are duplicated by a plain bitwise
copy of their storage.

All built-in scalars and most plain value structs conform implicitly — the
compiler synthesises the conformance unless the type explicitly opts out
with `not Copyable`. Opt out for types that own a resource (a heap
allocation, a file handle) where bitwise duplication would alias the
resource and break ownership.

_Defined in `lang/std/core/copy.ks`._

## protocol `Defaultable`

```kestrel
public protocol Defaultable
```

Protocol for types with a meaningful zero/default value.

`Defaultable` is what `T()` resolves to when no other init is
chosen. Conform when there's an obvious default: `0` for numbers,
`""` for strings, the empty collection for containers. Don't
conform just to satisfy a generic bound — the absence of a default
is information.

_Defined in `lang/std/core/protocols.ks`._

### Members

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

## typealias `DictionaryLiteralType`

```kestrel
public type DictionaryLiteralType[K, V] = std.collections.Dictionary[K, V, std.collections.DefaultHasher]
```

_Defined in `lang/std/core/literals.ks`._

## protocol `DivideAssign`

```kestrel
public protocol DivideAssign[Other = Self]
```

Raw protocol backing the `/=` operator.

_Defined in `lang/std/core/assign.ks`._

### Members

#### function `divideAssign`

```kestrel
mutating func divideAssign(Other)
```

Mutates `self` to `self / other`.

_Defined in `lang/std/core/assign.ks`._

## protocol `Divisible`

```kestrel
public protocol Divisible[Other = Self]
```

Raw protocol backing the `/` operator.

Division by zero is not modelled at the protocol level; conforming types
document their own behavior (integer types panic, floats produce `inf`/`nan`).

_Defined in `lang/std/core/arithmetic.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `divide`

```kestrel
func divide(Other) -> Output
```

Returns `self / other`.

_Defined in `lang/std/core/arithmetic.ks`._

## protocol `Equal`

```kestrel
public protocol Equal[Other = Self]
```

Raw protocol backing the `==` operator.

Most user code should conform to `Equatable` instead, which conforms to
`Equal[Self]` automatically with `Output = Bool`. Implement `Equal` directly
only when you need a non-Bool result (e.g. lifting equality into a vector
type that returns a mask).

### Examples

```
1 == 1   // true
"a" == "b"  // false
```

_Defined in `lang/std/core/comparison.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/comparison.ks`._

#### function `isEqual`

```kestrel
func isEqual(to: Other) -> Output
```

Returns the equality result as `Output` — typically `Bool`.

_Defined in `lang/std/core/comparison.ks`._

## protocol `Equatable`

```kestrel
public protocol Equatable
```

Protocol for types whose values can be compared for equality.

`Equatable` is the semantic counterpart to the raw `Equal[Self]`
operator protocol: conformers implement `isEqual` returning `Bool`, and a
blanket extension below derives both `==` and `!=`. Most types should
reach for `Equatable` rather than `Equal` directly — the `Bool`
associated-type binding is wired up automatically.

### Examples

```
public struct Point: Equatable {
    public var x: Int64
    public var y: Int64
    public func isEqual(to other: Point) -> Bool {
        self.x == other.x and self.y == other.y
    }
}

Point(x: 1, y: 2) == Point(x: 1, y: 2)   // true
```

_Defined in `lang/std/core/protocols.ks`._

### Implements `Equal`

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/protocols.ks`._

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/protocols.ks`._

#### function `isEqual`

```kestrel
func isEqual(to: Self) -> Bool
```

Returns `true` iff `self` and `other` are considered equal. Should
be reflexive, symmetric, and transitive — `Hashable` requires equal
values to hash equal, so don't drift from those laws.

_Defined in `lang/std/core/protocols.ks`._

### Implements `NotEqual`

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/comparison.ks`._

#### function `isNotEqual`

```kestrel
public func isNotEqual(to: Self) -> Bool
```

Default `!=` derived from `isEqual`.

_Defined in `lang/std/core/protocols.ks`._

## protocol `ExpressibleByArrayLiteral`

```kestrel
public protocol ExpressibleByArrayLiteral
```

User-facing protocol for array-literal lowering.

Provides a `LiteralSlice` view over the literal's contents so the
implementation can iterate or copy without juggling raw pointers.

_Defined in `lang/std/core/literals.ks`._

### Members

#### initializer `Array Literal`

```kestrel
init(arrayLiteral: LiteralSlice[Element])
```

Builds an instance from a literal slice of elements.

_Defined in `lang/std/core/literals.ks`._

### Implements `_ExpressibleByArrayLiteral`

#### typealias `Element`

```kestrel
type Element
```

_Defined in `lang/std/core/literals.ks`._

#### initializer `Literal Bridge`

```kestrel
init(_arrayLiteralPointer: consuming lang.ptr[Element], _arrayLiteralCount: consuming lang.i64)
```

Compiler-emitted init taking a raw pointer and count.

Both params are `consuming`: the compiler hands ownership of the
stack buffer's address (and the count) over to the implementation,
which stores them in its own storage. This convention is what the
MIR lowering's structural predicate looks for — implementations
that deviate will be silently skipped during literal lowering.

_Defined in `lang/std/core/literals.ks`._

## protocol `ExpressibleByBoolLiteral`

```kestrel
public protocol ExpressibleByBoolLiteral
```

Protocol for types that accept a `true`/`false` literal.

The init takes a primitive `lang.i1` rather than `Bool` because `Bool`
itself conforms — the literal lowering needs a representation that does
not depend on the type being constructed.

_Defined in `lang/std/core/literals.ks`._

### Members

#### initializer `Bool Literal`

```kestrel
init(boolLiteral: lang.i1)
```

Builds an instance from a boolean literal.

_Defined in `lang/std/core/literals.ks`._

## protocol `ExpressibleByCharLiteral`

```kestrel
public protocol ExpressibleByCharLiteral
```

Protocol for types that accept a character literal (`'a'`).

_Defined in `lang/std/core/literals.ks`._

### Members

#### initializer `Char Literal`

```kestrel
init(charLiteral: lang.i32)
```

Builds an instance from a character literal.

_Defined in `lang/std/core/literals.ks`._

## protocol `ExpressibleByDictionaryLiteral`

```kestrel
public protocol ExpressibleByDictionaryLiteral
```

User-facing protocol for dictionary-literal lowering. Mirrors
`ExpressibleByArrayLiteral` but for key-value pairs.

_Defined in `lang/std/core/literals.ks`._

### Members

#### initializer `Dictionary Literal`

```kestrel
init(dictionaryLiteral: LiteralSlice[(Key, Value)])
```

Builds an instance from a literal slice of key-value pairs.

_Defined in `lang/std/core/literals.ks`._

### Implements `_ExpressibleByDictionaryLiteral`

#### typealias `Key`

```kestrel
type Key
```

_Defined in `lang/std/core/literals.ks`._

#### initializer `Literal Bridge`

```kestrel
init(consuming lang.ptr[(Key, Value)], consuming lang.i64)
```

Compiler-emitted init taking a raw `(Key, Value)` pointer and count.

Both params are `consuming` for the same reason as the array
bridge: the compiler hands ownership of the stack buffer to the
implementation. MIR lowering matches on the unwrapped param
shape, so an impl that deviates from this convention will be
skipped during literal lowering.

_Defined in `lang/std/core/literals.ks`._

#### typealias `Value`

```kestrel
type Value
```

_Defined in `lang/std/core/literals.ks`._

## protocol `ExpressibleByFloatLiteral`

```kestrel
public protocol ExpressibleByFloatLiteral
```

Protocol for types that accept a floating-point literal (e.g. `3.14`).

_Defined in `lang/std/core/literals.ks`._

### Members

#### initializer `Float Literal`

```kestrel
init(floatLiteral: lang.f64)
```

Builds an instance from a floating-point literal.

_Defined in `lang/std/core/literals.ks`._

## protocol `ExpressibleByIntLiteral`

```kestrel
public protocol ExpressibleByIntLiteral
```

Protocol for types that accept an integer literal (e.g. `42`, `0xff`).

All the standard integer widths conform; types outside `std.numeric` (for
example a `BigInt` or a fixed-point number) can also conform to opt in
to the literal syntax.

_Defined in `lang/std/core/literals.ks`._

### Members

#### initializer `Int Literal`

```kestrel
init(intLiteral: lang.i64)
```

Builds an instance from an integer literal.

_Defined in `lang/std/core/literals.ks`._

## protocol `ExpressibleByNullLiteral`

```kestrel
public protocol ExpressibleByNullLiteral
```

Protocol for types that accept the `null` literal.

`Optional[T]` is the canonical conformer; it produces `.None`. Types
that wrap an optional or have a meaningful "absent" state may also
conform.

_Defined in `lang/std/core/literals.ks`._

### Members

#### initializer `Null Literal`

```kestrel
init()
```

Builds the absent/none instance.

_Defined in `lang/std/core/literals.ks`._

## protocol `ExpressibleByStringLiteral`

```kestrel
public protocol ExpressibleByStringLiteral
```

Protocol for types that accept a string literal (`"…"`).

The init receives a raw pointer and byte length so that string literal
lowering does not require the target type to already exist in stdlib form.

_Defined in `lang/std/core/literals.ks`._

### Members

#### initializer `String Literal`

```kestrel
init(stringLiteral: lang.ptr[lang.i8], lang.i64)
```

Builds an instance from a string literal.

_Defined in `lang/std/core/literals.ks`._

## typealias `FloatLiteralType`

```kestrel
public type FloatLiteralType = Float64
```

Default type for float literals (`let x = 1.0` → `Float64`).

_Defined in `lang/std/core/literals.ks`._

## protocol `FromResidual`

```kestrel
public protocol FromResidual[Residual]
```

Protocol that lets a return type absorb a `try`-propagated residual.

Implement when your error/optional type should be reachable via `try`
from another type with a different residual. For example, `Result[T, E]`
implements `FromResidual[E]` so that `try someResult` inside a function
returning `Result[T, E]` rebuilds the failure.

_Defined in `lang/std/core/error.ks`._

### Members

#### function `fromResidual`

```kestrel
static func fromResidual(Residual) -> Self
```

Builds an instance carrying `residual` as its failure payload.

_Defined in `lang/std/core/error.ks`._

## protocol `Greater`

```kestrel
public protocol Greater[Other = Self]
```

Raw protocol backing the `>` operator. See `Less` for guidance.

_Defined in `lang/std/core/comparison.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/comparison.ks`._

#### function `greaterThan`

```kestrel
func greaterThan(Other) -> Output
```

Returns the greater-than result as `Output` — typically `Bool`.

_Defined in `lang/std/core/comparison.ks`._

## protocol `GreaterOrEqual`

```kestrel
public protocol GreaterOrEqual[Other = Self]
```

Raw protocol backing the `>=` operator. See `Less` for guidance.

_Defined in `lang/std/core/comparison.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/comparison.ks`._

#### function `greaterThanOrEqual`

```kestrel
func greaterThanOrEqual(Other) -> Output
```

Returns the greater-than-or-equal result as `Output` — typically `Bool`.

_Defined in `lang/std/core/comparison.ks`._

## protocol `Hashable`

```kestrel
public protocol Hashable
```

Protocol for types whose values can be hashed.

`Hashable` extends `Equatable`: the contract is that `a == b` implies
`a.hash(into:)` and `b.hash(into:)` feed the same bytes to the hasher.
Violating this breaks `Set` and `Dictionary` — equal lookups won't
land on the equal stored value. The hasher is generic so the same
hash impl works across hashing algorithms (SipHash, FxHash, etc.).

### Examples

```
public struct Tag: Hashable {
    public var name: String
    public func isEqual(to other: Tag) -> Bool { self.name == other.name }
    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.name.hash(into: hasher)
    }
}
```

_Defined in `lang/std/core/protocols.ks`._

### Members

#### function `hash`

```kestrel
func hash[H](into: mutating H) where H: Hasher
```

Feeds this value's bytes into `hasher`. Must be deterministic
across calls and consistent with `isEqual`.

_Defined in `lang/std/core/protocols.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
func isEqual(to: Self) -> Bool
```

Returns `true` iff `self` and `other` are considered equal. Should
be reflexive, symmetric, and transitive — `Hashable` requires equal
values to hash equal, so don't drift from those laws.

_Defined in `lang/std/core/protocols.ks`._

## protocol `Hasher`

```kestrel
public protocol Hasher
```

Protocol for hash algorithm implementations consumed by `Hashable`.

The contract is the same as Rust / Swift: `Hashable`-conforming types
`write` their bytes into the hasher; the hasher accumulates state
and emits a `UInt64` digest on `finish()`. Used by `Set`,
`Dictionary`, and any structure that wants stable hashes.

_Defined in `lang/std/core/protocols.ks`._

### Members

#### function `finish`

```kestrel
mutating func finish() -> UInt64
```

Returns the finalised hash. After calling `finish` the hasher's
state is unspecified — don't reuse it.

_Defined in `lang/std/core/protocols.ks`._

#### function `write`

```kestrel
mutating func write(ArraySlice[UInt8])
```

Mixes `bytes` into the running hash state.

_Defined in `lang/std/core/protocols.ks`._

## typealias `IntegerLiteralType`

```kestrel
public type IntegerLiteralType = Int64
```

Default type for integer literals (`let x = 1` → `Int64`).

_Defined in `lang/std/core/literals.ks`._

## protocol `LeftShift`

```kestrel
public protocol LeftShift[Other = Int64]
```

Raw protocol backing the `<<` operator.

`Other` defaults to `Int64` — the standard shift count type. Conforming
types may narrow this to a different count where appropriate.

### Errors

Standard integer types panic on out-of-range shift counts (see the
`shiftLeft` documentation on the integer types).

_Defined in `lang/std/core/bitwise.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftLeft`

```kestrel
func shiftLeft(by: Other) -> Output
```

Returns `self << count`.

_Defined in `lang/std/core/bitwise.ks`._

## protocol `LeftShiftAssign`

```kestrel
public protocol LeftShiftAssign[Other]
```

Raw protocol backing the `<<=` operator.

_Defined in `lang/std/core/assign.ks`._

### Members

#### function `shiftLeftAssign`

```kestrel
mutating func shiftLeftAssign(by: Other)
```

Mutates `self` to `self << count`.

_Defined in `lang/std/core/assign.ks`._

## protocol `Less`

```kestrel
public protocol Less[Other = Self]
```

Raw protocol backing the `<` operator.

`Comparable` derives `Less`, `LessOrEqual`, `Greater`, `GreaterOrEqual` from
a single `compare()` method, so prefer conforming to `Comparable` for
totally-ordered types.

_Defined in `lang/std/core/comparison.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/comparison.ks`._

#### function `lessThan`

```kestrel
func lessThan(Other) -> Output
```

Returns the less-than result as `Output` — typically `Bool`.

_Defined in `lang/std/core/comparison.ks`._

## protocol `LessOrEqual`

```kestrel
public protocol LessOrEqual[Other = Self]
```

Raw protocol backing the `<=` operator. See `Less` for guidance.

_Defined in `lang/std/core/comparison.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/comparison.ks`._

#### function `lessThanOrEqual`

```kestrel
func lessThanOrEqual(Other) -> Output
```

Returns the less-than-or-equal result as `Output` — typically `Bool`.

_Defined in `lang/std/core/comparison.ks`._

## protocol `Matchable`

```kestrel
public protocol Matchable
```

Protocol enabling `match` against custom types via the `case` pattern.

Conformers decide what "matches" means — for `Bool` and the integer
types it is straight equality; for ranges it is containment. The
compiler lowers `case <pattern> =>` to a `matches` call.

_Defined in `lang/std/core/protocols.ks`._

### Members

#### function `matches`

```kestrel
func matches(Self) -> Bool
```

Returns `true` if `other` matches the receiver.

_Defined in `lang/std/core/protocols.ks`._

## protocol `Modulo`

```kestrel
public protocol Modulo[Other = Self]
```

Raw protocol backing the `%` operator.

For integers this is the remainder of truncated division, with the sign of
the dividend. Use `floorMod` (defined on integer types) when you want
Euclidean / floor-style remainder semantics.

_Defined in `lang/std/core/arithmetic.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `modulo`

```kestrel
func modulo(Other) -> Output
```

Returns `self % other`.

_Defined in `lang/std/core/arithmetic.ks`._

## protocol `ModuloAssign`

```kestrel
public protocol ModuloAssign[Other = Self]
```

Raw protocol backing the `%=` operator.

_Defined in `lang/std/core/assign.ks`._

### Members

#### function `modAssign`

```kestrel
mutating func modAssign(Other)
```

Mutates `self` to `self % other`.

_Defined in `lang/std/core/assign.ks`._

## protocol `Multipliable`

```kestrel
public protocol Multipliable[Other = Self]
```

Raw protocol backing the `*` operator.

The associated `one` value is the multiplicative identity, used as the
starting accumulator for products and powers.

### Examples

```
6 * 7         // 42
Int64.one     // 1
```

_Defined in `lang/std/core/arithmetic.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `multiply`

```kestrel
func multiply(Other) -> Output
```

Returns `self * other`.

_Defined in `lang/std/core/arithmetic.ks`._

#### field `one`

```kestrel
static var one: Self { get }
```

The multiplicative identity — a value `o` such that `x * o == x` for all `x`.

_Defined in `lang/std/core/arithmetic.ks`._

## protocol `MultiplyAssign`

```kestrel
public protocol MultiplyAssign[Other = Self]
```

Raw protocol backing the `*=` operator.

_Defined in `lang/std/core/assign.ks`._

### Members

#### function `multiplyAssign`

```kestrel
mutating func multiplyAssign(Other)
```

Mutates `self` to `self * other`.

_Defined in `lang/std/core/assign.ks`._

## protocol `Negatable`

```kestrel
public protocol Negatable
```

Raw protocol backing the unary `-` operator.

On signed two's-complement integers, negating the minimum value overflows
(e.g. `-Int8.minValue == Int8.minValue`); the operator wraps. Use
`checkedNegate` if overflow needs to surface.

_Defined in `lang/std/core/arithmetic.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `negate`

```kestrel
func negate() -> Output
```

Returns `-self`.

_Defined in `lang/std/core/arithmetic.ks`._

## protocol `Not`

```kestrel
public protocol Not
```

Raw protocol backing the `not` keyword operator.

_Defined in `lang/std/core/logical.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/logical.ks`._

#### function `logicalNot`

```kestrel
func logicalNot() -> Output
```

Returns `not self`.

_Defined in `lang/std/core/logical.ks`._

## protocol `NotEqual`

```kestrel
public protocol NotEqual[Other = Self]
```

Raw protocol backing the `!=` operator.

`Equatable` provides a default `isNotEqual` derived from `isEqual`, so
conforming to `Equatable` is enough for both `==` and `!=`.

_Defined in `lang/std/core/comparison.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/comparison.ks`._

#### function `isNotEqual`

```kestrel
func isNotEqual(to: Other) -> Output
```

Returns the inequality result as `Output` — typically `Bool`.

_Defined in `lang/std/core/comparison.ks`._

## typealias `NullLiteralType`

```kestrel
public type NullLiteralType[T] = std.result.Optional[T]
```

Default type for null literals (`let x = null` → `Optional[T]`).

_Defined in `lang/std/core/literals.ks`._

## protocol `Or`

```kestrel
public protocol Or[Other = Self]
```

Raw protocol backing the `or` keyword operator.

As with `And`, `other` is a thunk so the right-hand side can be skipped
when `self` already determines the result.

_Defined in `lang/std/core/logical.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/logical.ks`._

#### function `logicalOr`

```kestrel
func logicalOr(() -> Other) -> Output
```

Returns `self or other()`. The closure runs only if needed.

_Defined in `lang/std/core/logical.ks`._

## enum `Ordering`

```kestrel
public enum Ordering
```

The three-valued result of a `Comparable.compare()` call.

`Ordering` is the lingua franca for comparison: types implementing
`Comparable` define a single `compare` returning this enum, and the
stdlib derives `<`, `<=`, `>`, `>=` on top. The `then` / `thenWith`
helpers make it easy to chain comparisons over multiple fields without
nested `if`s.

### Examples

```
let cmp = a.compare(b);
match cmp {
    .Less => "ascending",
    .Equal => "tied",
    .Greater => "descending"
}

// Chain field comparisons: by lastName, then firstName.
a.lastName.compare(b.lastName)
    .then(a.firstName.compare(b.firstName))
```

### Representation

A plain three-state enum with no payload — lowers to a small integer tag.

_Defined in `lang/std/core/ordering.ks`._

### Members

#### case `Equal`

```kestrel
case Equal
```

The two values compared equal.

_Defined in `lang/std/core/ordering.ks`._

#### case `Greater`

```kestrel
case Greater
```

The receiver compared greater than the argument.

_Defined in `lang/std/core/ordering.ks`._

#### case `Less`

```kestrel
case Less
```

The receiver compared less than the argument.

_Defined in `lang/std/core/ordering.ks`._

#### function `reverse`

```kestrel
public func reverse() -> Ordering
```

Swaps `Less` and `Greater`; leaves `Equal` alone. Useful for sorting
in reverse without writing a second comparator.

##### Examples

```
Ordering.Less.reverse()     // .Greater
Ordering.Equal.reverse()    // .Equal
```

_Defined in `lang/std/core/ordering.ks`._

#### function `then`

```kestrel
public func then(Ordering) -> Ordering
```

Tie-breaker chain: returns `self` if it is non-`Equal`, otherwise
`other`. The eager form — both arguments are evaluated.

##### Examples

```
Ordering.Equal.then(.Less)     // .Less
Ordering.Greater.then(.Less)   // .Greater (self wins)
```

_Defined in `lang/std/core/ordering.ks`._

#### function `thenWith`

```kestrel
public func thenWith(() -> Ordering) -> Ordering
```

Lazy variant of `then` — `compare` runs only when `self` is `Equal`.
Prefer this when computing the secondary comparison is expensive.

_Defined in `lang/std/core/ordering.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Ordering) -> Bool
```

Equality on the orderings themselves: same variant ⇒ equal.

_Defined in `lang/std/core/ordering.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating StringBuilder, FormatOptions)
```

Renders as `"Less"`, `"Equal"`, or `"Greater"`. With `debug` set,
prefixes with the type name (`"Ordering.Less"`).

_Defined in `lang/std/core/ordering.ks`._

## struct `Range`

```kestrel
public struct Range[T] where T: Steppable, T: Comparable { /* private fields */ }
```

Half-open range `[start, end)` — produced by the `..<` operator.

`Range` is `Iterable`, so `for x in 0..<10 { … }` works directly.
`T` must be `Steppable` (defines `successor()`) and `Comparable` (so
the iterator knows when to stop). Empty ranges (`start >= end`) yield
nothing.

### Examples

```
for i in 0..<3 { print(i) }   // 0, 1, 2
(0..<10).contains(5)          // true
(0..<0).isEmpty()             // true
```

### Representation

Two values: `start` and `end`. No heap allocation.

_Defined in `lang/std/core/range.ks`._

### Members

#### initializer `From Bounds`

```kestrel
public init(T, T)
```

Builds the range `[start, end)`.

_Defined in `lang/std/core/range.ks`._

#### function `contains`

```kestrel
public func contains(T) -> Bool
```

Returns `true` iff `start <= value < end`.

_Defined in `lang/std/core/range.ks`._

#### field `end`

```kestrel
public var end: T
```

Upper bound — excluded from the range.

_Defined in `lang/std/core/range.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` when `start >= end` (no values are produced).

_Defined in `lang/std/core/range.ks`._

#### field `start`

```kestrel
public var start: T
```

Lower bound — included in the range.

_Defined in `lang/std/core/range.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Range[T]) -> Bool
```

Equal when both bounds match. Useful for range-keyed lookups and
tests, not a structural property of the iteration order.

_Defined in `lang/std/core/range.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/core/range.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = RangeIterator[T]
```

_Defined in `lang/std/core/range.ks`._

#### function `iter`

```kestrel
public func iter() -> RangeIterator[T]
```

Returns a fresh iterator over the range. Multiple calls produce
independent iterators — `Range` is value-typed.

_Defined in `lang/std/core/range.ks`._

### Implements `SeqIndex`

#### typealias `SeqOutput`

```kestrel
type SeqOutput = ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeq`

```kestrel
public func readSeq(from: ArraySlice[T]) -> ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeqChecked`

```kestrel
public func readSeqChecked(from: ArraySlice[T]) -> ArraySlice[T]?
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeqUnchecked`

```kestrel
public func readSeqUnchecked(from: ArraySlice[T]) -> ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `writeSeq`

```kestrel
public func writeSeq(to: ArraySlice[T], with: ArraySlice[T])
```

_Defined in `lang/std/collections/slice.ks`._

#### function `writeSeqUnchecked`

```kestrel
public func writeSeqUnchecked(to: ArraySlice[T], with: ArraySlice[T])
```

_Defined in `lang/std/collections/slice.ks`._

### Implements `SeqClampable`

#### typealias `SeqClampedOutput`

```kestrel
type SeqClampedOutput = ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeqClamped`

```kestrel
public func readSeqClamped(from: ArraySlice[T]) -> ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `writeSeqClamped`

```kestrel
public func writeSeqClamped(to: ArraySlice[T], with: ArraySlice[T])
```

_Defined in `lang/std/collections/slice.ks`._

### Implements `SeqRange`

#### function `resolve`

```kestrel
public func resolve(Int64) -> Range[Int64]
```

_Defined in `lang/std/collections/slice.ks`._

### Implements `BytesIndex`

#### typealias `BytesYield`

```kestrel
type BytesYield = BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytes`

```kestrel
public func readBytes(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesChecked`

```kestrel
public func readBytesChecked(from: BytesView) -> BytesView?
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesUnchecked`

```kestrel
public func readBytesUnchecked(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesClampable`

#### typealias `BytesClampedYield`

```kestrel
type BytesClampedYield = BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesClamped`

```kestrel
public func readBytesClamped(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesWrappable`

#### typealias `BytesWrappedYield`

```kestrel
type BytesWrappedYield = BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesWrapped`

```kestrel
public func readBytesWrapped(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesSubstringIndex`

#### function `readBytesSubstring`

```kestrel
public func readBytesSubstring(from: BytesView) -> String
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsIndex`

#### typealias `CharsYield`

```kestrel
type CharsYield = CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readChars`

```kestrel
public func readChars(from: CharsView) -> CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsChecked`

```kestrel
public func readCharsChecked(from: CharsView) -> CharsView?
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsClampable`

#### typealias `CharsClampedYield`

```kestrel
type CharsClampedYield = CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsClamped`

```kestrel
public func readCharsClamped(from: CharsView) -> CharsView
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsWrappable`

#### typealias `CharsWrappedYield`

```kestrel
type CharsWrappedYield = CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsWrapped`

```kestrel
public func readCharsWrapped(from: CharsView) -> CharsView
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsSubstringIndex`

#### function `readCharsSubstring`

```kestrel
public func readCharsSubstring(from: CharsView) -> String
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesIndex`

#### typealias `GraphemesYield`

```kestrel
type GraphemesYield = GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemes`

```kestrel
public func readGraphemes(from: GraphemesView) -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesChecked`

```kestrel
public func readGraphemesChecked(from: GraphemesView) -> GraphemesView?
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesClampable`

#### typealias `GraphemesClampedYield`

```kestrel
type GraphemesClampedYield = GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesClamped`

```kestrel
public func readGraphemesClamped(from: GraphemesView) -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesWrappable`

#### typealias `GraphemesWrappedYield`

```kestrel
type GraphemesWrappedYield = GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesWrapped`

```kestrel
public func readGraphemesWrapped(from: GraphemesView) -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesSubstringIndex`

#### function `readGraphemesSubstring`

```kestrel
public func readGraphemesSubstring(from: GraphemesView) -> String
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesIndex`

#### typealias `LinesYield`

```kestrel
type LinesYield = LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLines`

```kestrel
public func readLines(from: LinesView) -> LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesChecked`

```kestrel
public func readLinesChecked(from: LinesView) -> LinesView?
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesClampable`

#### typealias `LinesClampedYield`

```kestrel
type LinesClampedYield = LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesClamped`

```kestrel
public func readLinesClamped(from: LinesView) -> LinesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesWrappable`

#### typealias `LinesWrappedYield`

```kestrel
type LinesWrappedYield = LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesWrapped`

```kestrel
public func readLinesWrapped(from: LinesView) -> LinesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesSubstringIndex`

#### function `readLinesSubstring`

```kestrel
public func readLinesSubstring(from: LinesView) -> String
```

_Defined in `lang/std/text/views.ks`._

## protocol `RangeConstructible`

```kestrel
public protocol RangeConstructible[Other = Self]
```

Raw protocol backing the half-open `..<` operator (`start..<end`).

`Output` is the range type produced — usually `Range[Self]`, but
custom types may produce their own range flavor (e.g. a date range).

_Defined in `lang/std/core/range.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `exclusiveRange`

```kestrel
func exclusiveRange(to: Other) -> Output
```

Builds the half-open range `[self, end)`.

_Defined in `lang/std/core/range.ks`._

## struct `RangeFrom`

```kestrel
public struct RangeFrom[T] where T: Steppable, T: Comparable { /* private fields */ }
```

Partial range `[start, +∞)` — produced by the postfix `..` operator.

`RangeFrom` is `Iterable` and produces an infinite iterator. Use
`break` to terminate iteration.

### Examples

```
for i in 0.. {
    if i >= 5 { break; }
    print(i)
}
(10..).contains(42)   // true
```

### Representation

Single value: `start`. No heap allocation.

_Defined in `lang/std/core/range.ks`._

### Members

#### initializer `From Start`

```kestrel
public init(T)
```

_Defined in `lang/std/core/range.ks`._

#### function `contains`

```kestrel
public func contains(T) -> Bool
```

Returns `true` iff `value >= start`.

_Defined in `lang/std/core/range.ks`._

#### field `start`

```kestrel
public var start: T
```

Lower bound — included in the range.

_Defined in `lang/std/core/range.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: RangeFrom[T]) -> Bool
```

Structural equality.

_Defined in `lang/std/core/range.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/core/range.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = RangeFromIterator[T]
```

_Defined in `lang/std/core/range.ks`._

#### function `iter`

```kestrel
public func iter() -> RangeFromIterator[T]
```

Returns a fresh infinite iterator starting at `start`.

_Defined in `lang/std/core/range.ks`._

### Implements `SeqIndex`

#### typealias `SeqOutput`

```kestrel
type SeqOutput = ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeq`

```kestrel
public func readSeq(from: ArraySlice[T]) -> ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeqChecked`

```kestrel
public func readSeqChecked(from: ArraySlice[T]) -> ArraySlice[T]?
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeqUnchecked`

```kestrel
public func readSeqUnchecked(from: ArraySlice[T]) -> ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `writeSeq`

```kestrel
public func writeSeq(to: ArraySlice[T], with: ArraySlice[T])
```

_Defined in `lang/std/collections/slice.ks`._

#### function `writeSeqUnchecked`

```kestrel
public func writeSeqUnchecked(to: ArraySlice[T], with: ArraySlice[T])
```

_Defined in `lang/std/collections/slice.ks`._

### Implements `SeqRange`

#### function `resolve`

```kestrel
public func resolve(Int64) -> Range[Int64]
```

_Defined in `lang/std/collections/slice.ks`._

### Implements `BytesIndex`

#### typealias `BytesYield`

```kestrel
type BytesYield = BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytes`

```kestrel
public func readBytes(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesChecked`

```kestrel
public func readBytesChecked(from: BytesView) -> BytesView?
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesUnchecked`

```kestrel
public func readBytesUnchecked(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesClampable`

#### typealias `BytesClampedYield`

```kestrel
type BytesClampedYield = BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesClamped`

```kestrel
public func readBytesClamped(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesWrappable`

#### typealias `BytesWrappedYield`

```kestrel
type BytesWrappedYield = BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesWrapped`

```kestrel
public func readBytesWrapped(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesSubstringIndex`

#### function `readBytesSubstring`

```kestrel
public func readBytesSubstring(from: BytesView) -> String
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsIndex`

#### typealias `CharsYield`

```kestrel
type CharsYield = CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readChars`

```kestrel
public func readChars(from: CharsView) -> CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsChecked`

```kestrel
public func readCharsChecked(from: CharsView) -> CharsView?
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsClampable`

#### typealias `CharsClampedYield`

```kestrel
type CharsClampedYield = CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsClamped`

```kestrel
public func readCharsClamped(from: CharsView) -> CharsView
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsWrappable`

#### typealias `CharsWrappedYield`

```kestrel
type CharsWrappedYield = CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsWrapped`

```kestrel
public func readCharsWrapped(from: CharsView) -> CharsView
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsSubstringIndex`

#### function `readCharsSubstring`

```kestrel
public func readCharsSubstring(from: CharsView) -> String
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesIndex`

#### typealias `GraphemesYield`

```kestrel
type GraphemesYield = GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemes`

```kestrel
public func readGraphemes(from: GraphemesView) -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesChecked`

```kestrel
public func readGraphemesChecked(from: GraphemesView) -> GraphemesView?
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesClampable`

#### typealias `GraphemesClampedYield`

```kestrel
type GraphemesClampedYield = GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesClamped`

```kestrel
public func readGraphemesClamped(from: GraphemesView) -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesWrappable`

#### typealias `GraphemesWrappedYield`

```kestrel
type GraphemesWrappedYield = GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesWrapped`

```kestrel
public func readGraphemesWrapped(from: GraphemesView) -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesSubstringIndex`

#### function `readGraphemesSubstring`

```kestrel
public func readGraphemesSubstring(from: GraphemesView) -> String
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesIndex`

#### typealias `LinesYield`

```kestrel
type LinesYield = LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLines`

```kestrel
public func readLines(from: LinesView) -> LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesChecked`

```kestrel
public func readLinesChecked(from: LinesView) -> LinesView?
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesClampable`

#### typealias `LinesClampedYield`

```kestrel
type LinesClampedYield = LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesClamped`

```kestrel
public func readLinesClamped(from: LinesView) -> LinesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesWrappable`

#### typealias `LinesWrappedYield`

```kestrel
type LinesWrappedYield = LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesWrapped`

```kestrel
public func readLinesWrapped(from: LinesView) -> LinesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesSubstringIndex`

#### function `readLinesSubstring`

```kestrel
public func readLinesSubstring(from: LinesView) -> String
```

_Defined in `lang/std/text/views.ks`._

## protocol `RangeFromConstructible`

```kestrel
public protocol RangeFromConstructible
```

Protocol backing the postfix `..` operator (`start..`).

`Output` is the range type produced — usually `RangeFrom[Self]`.

_Defined in `lang/std/core/range.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `rangeFrom`

```kestrel
func rangeFrom() -> Output
```

Builds the partial range `[self, +∞)`.

_Defined in `lang/std/core/range.ks`._

## struct `RangeFromIterator`

```kestrel
public struct RangeFromIterator[T] where T: Steppable, T: Comparable { /* private fields */ }
```

Iterator over a `RangeFrom[T]`. Yields successive values via
`Steppable.successor()` with no upper bound — callers must `break`.

### Representation

Single value: `current` (next yield).

_Defined in `lang/std/core/range.ks`._

### Members

#### initializer `From Start`

```kestrel
public init(current: T)
```

_Defined in `lang/std/core/range.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/core/range.ks`._

#### function `next`

```kestrel
public mutating func next() -> T?
```

Yields the next value. Never returns `.None` — infinite iterator.

_Defined in `lang/std/core/range.ks`._

## struct `RangeIterator`

```kestrel
public struct RangeIterator[T] where T: Steppable, T: Comparable { /* private fields */ }
```

Iterator over a half-open `Range[T]`. Yields successive values via
`Steppable.successor()` until reaching (but not including) `end`.

### Representation

Two values: `current` (next yield) and `end` (sentinel).

_Defined in `lang/std/core/range.ks`._

### Members

#### initializer `From Bounds`

```kestrel
public init(current: T, end: T)
```

Builds an iterator that yields `current`, `current.successor()`, …
stopping before `end`.

_Defined in `lang/std/core/range.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/core/range.ks`._

#### function `next`

```kestrel
public mutating func next() -> T?
```

Yields the next value, or `.None` when exhausted.

_Defined in `lang/std/core/range.ks`._

## protocol `RangeMatchable`

```kestrel
public protocol RangeMatchable[Bound = Self]
```

Protocol enabling range patterns (`start..=end`, `..<end`, `start..`).

Split into three primitive comparisons rather than a single
"is in range" call so the compiler can lower partial ranges (e.g.
`..<10`) without synthesising a stand-in upper bound. The `Bound`
parameter lets a value be matched against bounds of a different type —
e.g. an `Int64` against `Char` bounds.

_Defined in `lang/std/core/protocols.ks`._

### Members

#### function `isAtLeast`

```kestrel
func isAtLeast(Bound) -> Bool
```

Returns `true` when `self >= bound`. Powers `start..` patterns.

_Defined in `lang/std/core/protocols.ks`._

#### function `isAtMost`

```kestrel
func isAtMost(Bound) -> Bool
```

Returns `true` when `self <= bound`. Powers `..=end` patterns.

_Defined in `lang/std/core/protocols.ks`._

#### function `isBelow`

```kestrel
func isBelow(Bound) -> Bool
```

Returns `true` when `self < bound`. Powers `..<end` patterns.

_Defined in `lang/std/core/protocols.ks`._

## struct `RangeThrough`

```kestrel
public struct RangeThrough[T] where T: Comparable { /* private fields */ }
```

Partial range `(-∞, end]` — produced by the prefix `..=` operator.

Not `Iterable` — there is no start to iterate from.

### Examples

```
(..=10).contains(10)   // true
(..=10).contains(11)   // false
```

### Representation

Single value: `end`. No heap allocation.

_Defined in `lang/std/core/range.ks`._

### Members

#### initializer `From End`

```kestrel
public init(T)
```

_Defined in `lang/std/core/range.ks`._

#### function `contains`

```kestrel
public func contains(T) -> Bool
```

Returns `true` iff `value <= end`.

_Defined in `lang/std/core/range.ks`._

#### field `end`

```kestrel
public var end: T
```

Upper bound — included in the range.

_Defined in `lang/std/core/range.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: RangeThrough[T]) -> Bool
```

Structural equality.

_Defined in `lang/std/core/range.ks`._

### Implements `SeqIndex`

#### typealias `SeqOutput`

```kestrel
type SeqOutput = ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeq`

```kestrel
public func readSeq(from: ArraySlice[T]) -> ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeqChecked`

```kestrel
public func readSeqChecked(from: ArraySlice[T]) -> ArraySlice[T]?
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeqUnchecked`

```kestrel
public func readSeqUnchecked(from: ArraySlice[T]) -> ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `writeSeq`

```kestrel
public func writeSeq(to: ArraySlice[T], with: ArraySlice[T])
```

_Defined in `lang/std/collections/slice.ks`._

#### function `writeSeqUnchecked`

```kestrel
public func writeSeqUnchecked(to: ArraySlice[T], with: ArraySlice[T])
```

_Defined in `lang/std/collections/slice.ks`._

### Implements `SeqRange`

#### function `resolve`

```kestrel
public func resolve(Int64) -> Range[Int64]
```

_Defined in `lang/std/collections/slice.ks`._

### Implements `BytesIndex`

#### typealias `BytesYield`

```kestrel
type BytesYield = BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytes`

```kestrel
public func readBytes(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesChecked`

```kestrel
public func readBytesChecked(from: BytesView) -> BytesView?
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesUnchecked`

```kestrel
public func readBytesUnchecked(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesClampable`

#### typealias `BytesClampedYield`

```kestrel
type BytesClampedYield = BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesClamped`

```kestrel
public func readBytesClamped(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesWrappable`

#### typealias `BytesWrappedYield`

```kestrel
type BytesWrappedYield = BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesWrapped`

```kestrel
public func readBytesWrapped(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesSubstringIndex`

#### function `readBytesSubstring`

```kestrel
public func readBytesSubstring(from: BytesView) -> String
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsIndex`

#### typealias `CharsYield`

```kestrel
type CharsYield = CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readChars`

```kestrel
public func readChars(from: CharsView) -> CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsChecked`

```kestrel
public func readCharsChecked(from: CharsView) -> CharsView?
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsClampable`

#### typealias `CharsClampedYield`

```kestrel
type CharsClampedYield = CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsClamped`

```kestrel
public func readCharsClamped(from: CharsView) -> CharsView
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsWrappable`

#### typealias `CharsWrappedYield`

```kestrel
type CharsWrappedYield = CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsWrapped`

```kestrel
public func readCharsWrapped(from: CharsView) -> CharsView
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsSubstringIndex`

#### function `readCharsSubstring`

```kestrel
public func readCharsSubstring(from: CharsView) -> String
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesIndex`

#### typealias `GraphemesYield`

```kestrel
type GraphemesYield = GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemes`

```kestrel
public func readGraphemes(from: GraphemesView) -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesChecked`

```kestrel
public func readGraphemesChecked(from: GraphemesView) -> GraphemesView?
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesClampable`

#### typealias `GraphemesClampedYield`

```kestrel
type GraphemesClampedYield = GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesClamped`

```kestrel
public func readGraphemesClamped(from: GraphemesView) -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesWrappable`

#### typealias `GraphemesWrappedYield`

```kestrel
type GraphemesWrappedYield = GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesWrapped`

```kestrel
public func readGraphemesWrapped(from: GraphemesView) -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesSubstringIndex`

#### function `readGraphemesSubstring`

```kestrel
public func readGraphemesSubstring(from: GraphemesView) -> String
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesIndex`

#### typealias `LinesYield`

```kestrel
type LinesYield = LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLines`

```kestrel
public func readLines(from: LinesView) -> LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesChecked`

```kestrel
public func readLinesChecked(from: LinesView) -> LinesView?
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesClampable`

#### typealias `LinesClampedYield`

```kestrel
type LinesClampedYield = LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesClamped`

```kestrel
public func readLinesClamped(from: LinesView) -> LinesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesWrappable`

#### typealias `LinesWrappedYield`

```kestrel
type LinesWrappedYield = LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesWrapped`

```kestrel
public func readLinesWrapped(from: LinesView) -> LinesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesSubstringIndex`

#### function `readLinesSubstring`

```kestrel
public func readLinesSubstring(from: LinesView) -> String
```

_Defined in `lang/std/text/views.ks`._

## protocol `RangeThroughConstructible`

```kestrel
public protocol RangeThroughConstructible
```

Protocol backing the prefix `..=` operator (`..=end`).

`Output` is the range type produced — usually `RangeThrough[Self]`.

_Defined in `lang/std/core/range.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `rangeThrough`

```kestrel
func rangeThrough() -> Output
```

Builds the partial range `(-∞, self]`.

_Defined in `lang/std/core/range.ks`._

## struct `RangeUpTo`

```kestrel
public struct RangeUpTo[T] where T: Comparable { /* private fields */ }
```

Partial range `(-∞, end)` — produced by the prefix `..<` operator.

Not `Iterable` — there is no start to iterate from.

### Examples

```
(..<10).contains(5)    // true
(..<10).contains(10)   // false
```

### Representation

Single value: `end`. No heap allocation.

_Defined in `lang/std/core/range.ks`._

### Members

#### initializer `From End`

```kestrel
public init(T)
```

_Defined in `lang/std/core/range.ks`._

#### function `contains`

```kestrel
public func contains(T) -> Bool
```

Returns `true` iff `value < end`.

_Defined in `lang/std/core/range.ks`._

#### field `end`

```kestrel
public var end: T
```

Upper bound — excluded from the range.

_Defined in `lang/std/core/range.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: RangeUpTo[T]) -> Bool
```

Structural equality.

_Defined in `lang/std/core/range.ks`._

### Implements `SeqIndex`

#### typealias `SeqOutput`

```kestrel
type SeqOutput = ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeq`

```kestrel
public func readSeq(from: ArraySlice[T]) -> ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeqChecked`

```kestrel
public func readSeqChecked(from: ArraySlice[T]) -> ArraySlice[T]?
```

_Defined in `lang/std/collections/slice.ks`._

#### function `readSeqUnchecked`

```kestrel
public func readSeqUnchecked(from: ArraySlice[T]) -> ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `writeSeq`

```kestrel
public func writeSeq(to: ArraySlice[T], with: ArraySlice[T])
```

_Defined in `lang/std/collections/slice.ks`._

#### function `writeSeqUnchecked`

```kestrel
public func writeSeqUnchecked(to: ArraySlice[T], with: ArraySlice[T])
```

_Defined in `lang/std/collections/slice.ks`._

### Implements `SeqRange`

#### function `resolve`

```kestrel
public func resolve(Int64) -> Range[Int64]
```

_Defined in `lang/std/collections/slice.ks`._

### Implements `BytesIndex`

#### typealias `BytesYield`

```kestrel
type BytesYield = BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytes`

```kestrel
public func readBytes(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesChecked`

```kestrel
public func readBytesChecked(from: BytesView) -> BytesView?
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesUnchecked`

```kestrel
public func readBytesUnchecked(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesClampable`

#### typealias `BytesClampedYield`

```kestrel
type BytesClampedYield = BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesClamped`

```kestrel
public func readBytesClamped(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesWrappable`

#### typealias `BytesWrappedYield`

```kestrel
type BytesWrappedYield = BytesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesWrapped`

```kestrel
public func readBytesWrapped(from: BytesView) -> BytesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `BytesSubstringIndex`

#### function `readBytesSubstring`

```kestrel
public func readBytesSubstring(from: BytesView) -> String
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsIndex`

#### typealias `CharsYield`

```kestrel
type CharsYield = CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readChars`

```kestrel
public func readChars(from: CharsView) -> CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsChecked`

```kestrel
public func readCharsChecked(from: CharsView) -> CharsView?
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsClampable`

#### typealias `CharsClampedYield`

```kestrel
type CharsClampedYield = CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsClamped`

```kestrel
public func readCharsClamped(from: CharsView) -> CharsView
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsWrappable`

#### typealias `CharsWrappedYield`

```kestrel
type CharsWrappedYield = CharsView
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsWrapped`

```kestrel
public func readCharsWrapped(from: CharsView) -> CharsView
```

_Defined in `lang/std/text/views.ks`._

### Implements `CharsSubstringIndex`

#### function `readCharsSubstring`

```kestrel
public func readCharsSubstring(from: CharsView) -> String
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesIndex`

#### typealias `GraphemesYield`

```kestrel
type GraphemesYield = GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemes`

```kestrel
public func readGraphemes(from: GraphemesView) -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesChecked`

```kestrel
public func readGraphemesChecked(from: GraphemesView) -> GraphemesView?
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesClampable`

#### typealias `GraphemesClampedYield`

```kestrel
type GraphemesClampedYield = GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesClamped`

```kestrel
public func readGraphemesClamped(from: GraphemesView) -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesWrappable`

#### typealias `GraphemesWrappedYield`

```kestrel
type GraphemesWrappedYield = GraphemesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readGraphemesWrapped`

```kestrel
public func readGraphemesWrapped(from: GraphemesView) -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `GraphemesSubstringIndex`

#### function `readGraphemesSubstring`

```kestrel
public func readGraphemesSubstring(from: GraphemesView) -> String
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesIndex`

#### typealias `LinesYield`

```kestrel
type LinesYield = LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLines`

```kestrel
public func readLines(from: LinesView) -> LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesChecked`

```kestrel
public func readLinesChecked(from: LinesView) -> LinesView?
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesClampable`

#### typealias `LinesClampedYield`

```kestrel
type LinesClampedYield = LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesClamped`

```kestrel
public func readLinesClamped(from: LinesView) -> LinesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesWrappable`

#### typealias `LinesWrappedYield`

```kestrel
type LinesWrappedYield = LinesView
```

_Defined in `lang/std/text/views.ks`._

#### function `readLinesWrapped`

```kestrel
public func readLinesWrapped(from: LinesView) -> LinesView
```

_Defined in `lang/std/text/views.ks`._

### Implements `LinesSubstringIndex`

#### function `readLinesSubstring`

```kestrel
public func readLinesSubstring(from: LinesView) -> String
```

_Defined in `lang/std/text/views.ks`._

## protocol `RangeUpToConstructible`

```kestrel
public protocol RangeUpToConstructible
```

Protocol backing the prefix `..<` operator (`..<end`).

`Output` is the range type produced — usually `RangeUpTo[Self]`.

_Defined in `lang/std/core/range.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/range.ks`._

#### function `rangeUpTo`

```kestrel
func rangeUpTo() -> Output
```

Builds the partial range `(-∞, self)`.

_Defined in `lang/std/core/range.ks`._

## protocol `RightShift`

```kestrel
public protocol RightShift[Other = Int64]
```

Raw protocol backing the `>>` operator.

Behaviour for signed types is arithmetic shift (sign-preserving); unsigned
types use logical shift. The `Other` default mirrors `LeftShift`.

_Defined in `lang/std/core/bitwise.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/bitwise.ks`._

#### function `shiftRight`

```kestrel
func shiftRight(by: Other) -> Output
```

Returns `self >> count`.

_Defined in `lang/std/core/bitwise.ks`._

## protocol `RightShiftAssign`

```kestrel
public protocol RightShiftAssign[Other]
```

Raw protocol backing the `>>=` operator.

_Defined in `lang/std/core/assign.ks`._

### Members

#### function `shiftRightAssign`

```kestrel
mutating func shiftRightAssign(by: Other)
```

Mutates `self` to `self >> count`.

_Defined in `lang/std/core/assign.ks`._

## typealias `StringLiteralType`

```kestrel
public type StringLiteralType = String
```

Default type for string literals (`let s = "hi"` → `String`).

_Defined in `lang/std/core/literals.ks`._

## protocol `SubtractAssign`

```kestrel
public protocol SubtractAssign[Other = Self]
```

Raw protocol backing the `-=` operator.

_Defined in `lang/std/core/assign.ks`._

### Members

#### function `subtractAssign`

```kestrel
mutating func subtractAssign(Other)
```

Mutates `self` to `self - other`.

_Defined in `lang/std/core/assign.ks`._

## protocol `Subtractable`

```kestrel
public protocol Subtractable[Other = Self]
```

Raw protocol backing the `-` binary operator.

_Defined in `lang/std/core/arithmetic.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

_Defined in `lang/std/core/arithmetic.ks`._

#### function `subtract`

```kestrel
func subtract(Other) -> Output
```

Returns `self - other`.

_Defined in `lang/std/core/arithmetic.ks`._

## protocol `Tryable`

```kestrel
public protocol Tryable
```

Protocol enabling the `try expr` operator.

`Output` is the success value the operator yields; `Residual` is the
"residual" — typically an `Err` variant, a `None`, or a typed error —
that gets propagated. The compiler lowers `try x` to roughly
`match x.tryExtract() { .Continue(v) => v, .Break(r) => return Self.fromResidual(r) }`,
which is why the enclosing function's return type must conform to
`FromResidual[Residual]`.

### Examples

```
// Optional and Result both conform; `try` chains them seamlessly.
func parseAndDouble(s: String) -> Int64? {
    let n = try Int64.parse(s);    // .None short-circuits the whole function
    .Some(n * 2)
}
```

_Defined in `lang/std/core/error.ks`._

### Members

#### typealias `Output`

```kestrel
type Output
```

The value produced by `try expr` on success.

_Defined in `lang/std/core/error.ks`._

#### typealias `Residual`

```kestrel
type Residual
```

The residual carried out of `try expr` on failure.

_Defined in `lang/std/core/error.ks`._

#### function `tryExtract`

```kestrel
func tryExtract() -> ControlFlow[Output, Residual]
```

Splits `self` into the success value or the early-return residual.

_Defined in `lang/std/core/error.ks`._

## protocol `_ExpressibleByArrayLiteral`

```kestrel
public protocol _ExpressibleByArrayLiteral
```

Compiler-internal protocol for array-literal lowering.

The lexer/parser lower `[a, b, c]` to a call into this init with a raw
pointer to a stack-allocated buffer of `Element`s. Only the compiler
uses this directly; user types should conform to
`ExpressibleByArrayLiteral` (which extends this with a friendlier API).

_Defined in `lang/std/core/literals.ks`._

### Members

#### typealias `Element`

```kestrel
type Element
```

_Defined in `lang/std/core/literals.ks`._

#### initializer `Literal Bridge`

```kestrel
init(_arrayLiteralPointer: consuming lang.ptr[Element], _arrayLiteralCount: consuming lang.i64)
```

Compiler-emitted init taking a raw pointer and count.

Both params are `consuming`: the compiler hands ownership of the
stack buffer's address (and the count) over to the implementation,
which stores them in its own storage. This convention is what the
MIR lowering's structural predicate looks for — implementations
that deviate will be silently skipped during literal lowering.

_Defined in `lang/std/core/literals.ks`._

## protocol `_ExpressibleByDictionaryLiteral`

```kestrel
public protocol _ExpressibleByDictionaryLiteral
```

Compiler-internal protocol for dictionary-literal lowering.

The compiler lowers `[k1: v1, k2: v2]` into a call with a raw pointer
to a `(Key, Value)` buffer. As with array literals, user types should
prefer `ExpressibleByDictionaryLiteral`.

_Defined in `lang/std/core/literals.ks`._

### Members

#### typealias `Key`

```kestrel
type Key
```

_Defined in `lang/std/core/literals.ks`._

#### initializer `Literal Bridge`

```kestrel
init(consuming lang.ptr[(Key, Value)], consuming lang.i64)
```

Compiler-emitted init taking a raw `(Key, Value)` pointer and count.

Both params are `consuming` for the same reason as the array
bridge: the compiler hands ownership of the stack buffer to the
implementation. MIR lowering matches on the unwrapped param
shape, so an impl that deviates from this convention will be
skipped during literal lowering.

_Defined in `lang/std/core/literals.ks`._

#### typealias `Value`

```kestrel
type Value
```

_Defined in `lang/std/core/literals.ks`._

## function `fatalError`

```kestrel
public func fatalError(String) -> Never
```

Aborts the process with `message`.

Returns `!` (the never type), so the compiler treats any code after a
`fatalError` call as unreachable. Use sparingly — almost every "this
should be impossible" branch is better expressed as a `Result` error or
a precondition check, because `fatalError` produces no recovery
opportunity for the caller.

### Examples

```
let mode = readMode();
match mode {
    .Read => doRead(),
    .Write => doWrite(),
    _ => fatalError("unsupported mode")
}
```

_Defined in `lang/std/core/panic.ks`._

