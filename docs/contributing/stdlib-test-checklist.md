# Standard Library Test Coverage Checklist

Methods marked `[x]` have test coverage in `lib/kestrel-test-suite/tests/stdlib/`.
Methods marked `[ ]` are not yet tested.
Items marked "(known limitation)" have tests that fail due to compiler bugs.

---

## Array[T]

### Constructors
- [x] `init()`
- [x] `init(capacity:)`
- [x] `init(repeating:count:)` — needs Cloneable on T (TODO: may fail at monomorphization)
- [x] `init(from:)`
- [x] `init(count:generator:)` (known limitation — signature collision with internal init)

### Properties
- [x] `count`
- [x] `capacity`
- [x] `isEmpty`
- [x] `indices`

### Element Access
- [x] `subscript(index:)` (direct get)
- [x] `subscript(index:)` assignment (known limitation — "cannot assign to temporary value")
- [x] `subscript(checked:)`
- [x] `subscript(unchecked:)`
- [x] `subscript(wrapping:)`
- [x] `subscript(clamping:)`
- [x] `subscript(range:)`
- [x] `subscript(checkedRange:)`
- [x] `subscript(uncheckedRange:)`
- [x] `subscript(clampingRange:)`
- [x] `first()`
- [x] `last()`
- [x] `asPointer()`
- [x] `asSlice()`
- [x] `isValidIndex()`
- [x] `setUnchecked(index:value:)`

### Adding Elements
- [x] `append(element:)`
- [x] `append(contentsOf:)`
- [x] `appendFrom(iterable:)`
- [x] `insert(element:at:)`

### Removing Elements
- [x] `pop()`
- [x] `popFirst()`
- [x] `remove(at:)`
- [x] `removeSubrange(range:)`
- [x] `clear()`
- [x] `retain(matching:)`
- [x] `removeAll(matching:)`

### Reordering
- [x] `swap(at:with:)`
- [x] `reverse()`
- [x] `reversed()`
- [x] `rotate(by:)`
- [x] `replaceSubrange(range:with:)`

### Shuffling
- [x] `shuffle()`
- [x] `shuffle(using:)`
- [x] `shuffled()`
- [x] `shuffled(using:)`

### Capacity Management
- [x] `reserveCapacity(minimumCapacity:)`
- [x] `shrinkToFit()`

### Searching (Predicate)
- [x] `firstIndex(matching:)`
- [x] `lastIndex(matching:)`
- [x] `first(matching:)`
- [x] `last(matching:)`

### Predicates
- [x] `all(satisfy:)`
- [x] `any(satisfy:)`
- [x] `countWhere(predicate:)`

### Slicing
- [x] `prefix(count:)`
- [x] `suffix(count:)`
- [x] `drop(first:)`
- [x] `drop(last:)`

### Chunking & Windows
- [x] `chunks(of:)`
- [x] `windows(of:)`

### Partitioning
- [x] `partition(by:)`
- [x] `partitioned(by:)`

### Iteration
- [x] `iter()`

### Equatable Extension (where T: Equatable)
- [x] `equals(other:)`
- [x] `contains(element:)`
- [x] `firstIndex(of:)`
- [x] `lastIndex(of:)`
- [x] `starts(with:)`
- [x] `ends(with:)`
- [x] `split(separator:)`
- [x] `remove(element:)`
- [x] `removeAll(element:)`
- [x] `dedup()`
- [x] `deduped()`

### Comparable Extension (where T: Comparable)
- [x] `sort()`
- [x] `sorted()`
- [x] `min()`
- [x] `max()`
- [x] `isSorted()`
- [x] `binarySearch(element:)`

### Hash Extension (where T: Hash)
- [x] `unique()`
- [x] `removeDuplicates()`

### Custom Sort
- [x] `sort(by:)`
- [x] `sorted(by:)`
- [x] `sort(byKey:)`
- [x] `sorted(byKey:)`

### Nested/Formattable Extensions
- [x] `flatten()`
- [x] `joined(separator:)`

---

## String

### Constructors
- [x] `init()`
- [x] `init(capacity:)`

### Properties
- [x] `byteCount`
- [x] `capacity`
- [x] `isEmpty`
- [x] `count` (Unicode code point count)

### View Properties
- [x] `bytes`
- [x] `chars`
- [x] `graphemes`
- [x] `lines`

### Character Access
- [x] `first()`
- [x] `last()`
- [x] `char(at:)`
- [x] `char(checked:)`
- [x] `char(unchecked:)`
- [x] `char(wrapping:)`
- [x] `char(clamping:)`

### Byte Access
- [x] `byteAt(index:)`
- [x] `byteAtUnchecked(index:)`

### Appending
- [x] `append(other:)`
- [x] `appendChar(c:)`
- [x] `appendByte(byte:)`
- [x] `clear()`

### Substrings
- [x] `substringBytes(from:to:)`

### Searching
- [x] `contains(substring:)`
- [x] `contains(matching:)`
- [x] `find(substring:)`
- [x] `find(matching:)`
- [x] `reverseFind(substring:)`
- [x] `starts(with:)`
- [x] `ends(with:)`

### Trimming (Mutating)
- [x] `trim()`
- [x] `trimStart()`
- [x] `trimEnd()`
- [x] `trim(matching:)`
- [x] `trimStart(matching:)`
- [x] `trimEnd(matching:)`

### Trimming (Non-Mutating)
- [x] `trimmed()`
- [x] `trimmedStart()`
- [x] `trimmedEnd()`
- [x] `trimmed(matching:)`
- [x] `trimmedStart(matching:)`
- [x] `trimmedEnd(matching:)`

### Case Conversion (ASCII)
- [x] `lowercaseAscii()`
- [x] `uppercaseAscii()`
- [x] `lowercasedAscii()`
- [x] `uppercasedAscii()`

### Case Conversion (Unicode)
- [x] `lowercase()`
- [x] `uppercase()`
- [x] `lowercased()`
- [x] `uppercased()`
- [x] `titlecased()`
- [x] `equalsCaseInsensitive(other:)`

### Replacement
- [x] `replace(pattern:with:)`
- [x] `replaced(pattern:with:)`

### Splitting
- [x] `split(separator:)`
- [x] `split(matching:)`

### Repeating & Padding
- [x] `repeated(count:)`
- [x] `pad(start:with:)`
- [x] `pad(end:with:)`

### Iteration
- [x] `iter()`

### Protocol Methods
- [x] `equals(other:)`
- [x] `compare(other:)`
- [x] `hash(into:)`
- [x] `clone()`
- [x] `format(options:)`
- [x] `add(other:)`

---

## Dictionary[K, V, H]

### Constructors
- [x] `init()`
- [x] `init(capacity:)`
- [x] `init(from:)` (known limitation — codegen AssociatedTypeProjection)
- [x] `init(grouping:by:)` (known limitation — codegen AssociatedTypeProjection)
- [x] `init(uniqueKeysWithValues:)` (known limitation — codegen AssociatedTypeProjection)

### Properties
- [x] `count`
- [x] `capacity`
- [x] `isEmpty`
- [x] `keys`
- [x] `values`

### Subscripts
- [x] `subscript(key:)`
- [x] `subscript(key:default:)`
- [x] `subscript(key:inserting:)`
- [x] `subscript(unwrap:)`

### Mutation
- [x] `insert(key:value:)`
- [x] `remove(key:)`
- [x] `clear()`
- [x] `update(key:with:)`
- [x] `upsert(key:default:with:)`
- [x] `merge(other:uniquingKeysWith:)`
- [x] `mergeFrom(pairs:uniquingKeysWith:)` (known limitation — codegen AssociatedTypeProjection)
- [x] `retain(matching:)`
- [x] `removeAll(matching:)`
- [x] `reserveCapacity(minimumCapacity:)`
- [x] `shrinkToFit()`

### Lookup
- [x] `contains(key:)`

### Searching & Predicates
- [x] `contains(matching:)`
- [x] `first(matching:)`
- [x] `all(satisfy:)`
- [x] `any(satisfy:)`
- [x] `countWhere(predicate:)`

### Transformations
- [x] `mapValues(transform:)`
- [x] `compactMapValues(transform:)`
- [x] `filter(matching:)`
- [x] `merging(other:uniquingKeysWith:)`

### Iteration
- [x] `iter()`

### Equatable Extension (where V: Equatable)
- [x] `equals(other:)`
- [x] `containsValue(value:)`
- [x] `firstKey(forValue:)`
- [x] `allKeys(forValue:)`

### Other Extensions
- [x] `deepClone()` (TODO: may fail — needs Cloneable on K and V)
- [x] `sumValues()`

---

## Set[T, H]

### Constructors
- [x] `init()`
- [x] `init(capacity:)`
- [x] `init(from:)`

### Properties
- [x] `count`
- [x] `capacity`
- [x] `isEmpty`

### Membership
- [x] `contains(element:)`
- [x] `iter()`

### Adding Elements
- [x] `insert(element:)`
- [x] `insert(contentsOf:)`
- [x] `formUnion(other:)`

### Removing Elements
- [x] `remove(element:)`
- [x] `clear()`
- [x] `retain(matching:)`
- [x] `removeAll(matching:)`
- [x] `formIntersection(other:)`
- [x] `formDifference(other:)`
- [x] `formSymmetricDifference(other:)`

### Set Operations (Non-Mutating)
- [x] `union(other:)`
- [x] `intersection(other:)`
- [x] `difference(other:)`
- [x] `symmetricDifference(other:)`

### Set Relations
- [x] `isSubset(of:)`
- [x] `isStrictSubset(of:)`
- [x] `isSuperset(of:)`
- [x] `isStrictSuperset(of:)`
- [x] `isDisjoint(with:)`

### Searching & Predicates
- [x] `contains(matching:)`
- [x] `first(matching:)`
- [x] `all(satisfy:)`
- [x] `any(satisfy:)`
- [x] `countWhere(predicate:)`

### Transformations
- [x] `filter(matching:)`
- [x] `map(transform:)`
- [x] `compactMap(transform:)`
- [x] `flatMap(transform:)`

### Capacity & Conversion
- [x] `reserveCapacity(minimumCapacity:)`
- [x] `shrinkToFit()`
- [x] `toArray()`

### Extensions
- [x] `equals(other:)`
- [x] `min()`
- [x] `max()`
- [x] `sorted()`
- [x] `sum()` (known limitation — codegen type mismatch in Addable constraint resolution)

---

## Optional[T]

### Query Methods
- [x] `isSome()`
- [x] `isNone()`
- [x] `isSomeAnd(predicate:)`

### Unwrapping
- [x] `unwrap()`
- [x] `expect(message:)`
- [x] `unwrapOr(default:)`
- [x] `unwrap(orElse:)`

### Transformations
- [x] `map(transform:)`
- [x] `flatMap(transform:)`
- [x] `flatten()`
- [x] `filter(predicate:)`
- [x] `inspect(fn:)`

### Combinators
- [x] `then(other:)`
- [x] `orElse(alternative:)`
- [x] `xor(other:)`
- [x] `zip(with:)`

### Conversion to Result
- [x] `okOr(error:)` (known limitation — method lookup fails on Optional)
- [x] `okOrElse(error:)` (known limitation — method lookup fails on Optional)

### Mutating Operations
- [x] `take()`
- [x] `replace(value:)`
- [x] `takeIf(predicate:)`

### Iteration
- [x] `iter()`

### Extensions
- [x] `equals(other:)` (Equatable)
- [x] `contains(value:)` (Equatable)
- [x] `compare(other:)` (Comparable)
- [x] `hash(into:)` (Hash)
- [x] `clone()` (known limitation — Int64 lacks Cloneable witness)
- [x] `format(options:)` (Formattable)

---

## Result[T, E]

### Query Methods
- [x] `isOk()`
- [x] `isErr()`

### Unwrapping
- [x] `unwrap()`
- [x] `unwrapOr(default:)`
- [x] `unwrap(orElse:)`
- [x] `unwrapErr()`

### Transformations
- [x] `map(transform:)`
- [x] `flatMap(transform:)`
- [x] `mapErr(transform:)`
- [x] `flatMapErr(transform:)`

### Conversion to Optional
- [x] `ok()`
- [x] `err()`

### Combinators
- [x] `andValue(other:)`
- [x] `andThen(transform:)`
- [x] `orValue(other:)`
- [x] `orElse(alternative:)`

### Iteration
- [x] `iter()`

### Extensions
- [x] `equals(other:)` (Equatable)
- [x] `format(options:)` (Formattable)

---

## RcBox[T]

- [x] `init(value:)`
- [x] `getValue()`
- [x] `setValue(value:)` (known limitation — deepClone test includes this)
- [x] `isUnique()`
- [x] `refCount()`
- [x] `clone()`
- [x] `deepClone()` (known limitation — no Cloneable witness for Int64)

---

## Range[T]

- [x] `init(start:end:)`
- [x] `contains(value:)`
- [x] `isEmpty()`
- [x] `equals(other:)`
- [x] `iter()`

## ClosedRange[T]

- [x] `init(start:end:)`
- [x] `contains(value:)`
- [x] `isEmpty()`
- [x] `equals(other:)`
- [x] `iter()`

---

## Bool

- [x] `equals(other:)`
- [x] `matches(other:)`
- [x] `hash(into:)`
- [x] `logicalAnd(other:)`
- [x] `logicalOr(other:)`
- [x] `logicalNot()`
- [x] `format(options:)`

---

## Int64

### Properties
- [x] `sign`
- [x] `isPositive`
- [x] `isNegative`
- [x] `isZero`
- [x] `isPowerOfTwo`
- [x] `countOnes`
- [x] `countZeros`
- [x] `leadingZeros`
- [x] `trailingZeros`
- [x] `byteSwapped`

### Comparison
- [x] `equals(other:)`
- [x] `compare(other:)`

### Stepping
- [x] `successor()`
- [x] `predecessor()`

### Arithmetic (Wrapping)
- [x] `add(other:)`
- [x] `subtract(other:)`
- [x] `multiply(other:)`
- [x] `divide(other:)`
- [x] `modulo(other:)`
- [x] `negate()`
- [x] `abs()`

### Arithmetic (Checked)
- [x] `addChecked(other:)`
- [x] `subtractChecked(other:)`
- [x] `multiplyChecked(other:)`
- [x] `divideChecked(other:)`
- [x] `negateChecked()`
- [x] `absChecked()`

### Arithmetic (Saturating)
- [x] `addSaturating(other:)`
- [x] `subtractSaturating(other:)`
- [x] `multiplySaturating(other:)`
- [x] `negateSaturating()`
- [x] `absSaturating()`

### Extended Arithmetic
- [x] `pow(exponent:)`
- [x] `gcd(other:)`
- [x] `lcm(other:)`

### Clamping
- [x] `clamp(min:max:)`

### Bitwise
- [x] `bitwiseAnd(other:)`
- [x] `bitwiseOr(other:)`
- [x] `bitwiseXor(other:)`
- [x] `bitwiseNot()`
- [x] `shiftLeft(by:)`
- [x] `shiftRight(by:)`
- [x] `rotateLeft(by:)`
- [x] `rotateRight(by:)`

### Compound Assignment
- [x] `addAssign(other:)`
- [x] `subtractAssign(other:)`
- [x] `multiplyAssign(other:)`
- [x] `divideAssign(other:)`
- [x] `modAssign(other:)`
- [x] `bitwiseAndAssign(other:)`
- [x] `bitwiseOrAssign(other:)`
- [x] `bitwiseXorAssign(other:)`
- [x] `shiftLeftAssign(by:)`
- [x] `shiftRightAssign(by:)`

### Byte Conversion
- [x] `toBytes()`
- [x] `toBytesBigEndian()`
- [x] `toBytesLittleEndian()`
- [x] `fromBytes(bytes:)`
- [x] `fromBytesBigEndian(bytes:)`
- [x] `fromBytesLittleEndian(bytes:)`

### Parsing
- [x] `parse(string:)` (known limitation — overload resolution)
- [x] `parse(string:radix:)` (known limitation — overload resolution)

### Other
- [x] `hash(into:)`
- [x] `format(options:)`

---

## Iterator Protocol

### Transformation Adapters
- [x] `map(transform:)`
- [x] `filter(predicate:)`
- [x] `filterMap(transform:)`
- [x] `compactMap()`
- [x] `enumerate()`
- [x] `flatMap(transform:)` (known limitation — type inference)
- [x] `scan(initial:combine:)`

### Limiting Adapters
- [x] `take(count:)`
- [x] `takeWhile(predicate:)`
- [x] `skip(count:)`
- [x] `skipWhile(predicate:)`

### Combining Adapters
- [x] `zip(other:)`
- [x] `chain(other:)`

### Utility Adapters
- [x] `peekable()` (known limitation — AssociatedTypeProjection in layout)
- [x] `fuse()` (known limitation — tested with cycle, cycle needs Cloneable)
- [x] `inspect(inspector:)` (known limitation — verifying side effects requires mutable closure captures)
- [x] `stepBy(n:)`
- [x] `intersperse(separator:)` (known limitation — AssociatedTypeProjection in layout)
- [x] `intersperseWith(separator:)` (TODO: may hit AssociatedTypeProjection issue)
- [x] `cycle()` (known limitation — ArrayIterator lacks Cloneable)

### Collecting
- [x] `collect()`
- [x] `count()`
- [x] `unzip()` (known limitation — AssociatedTypeProjection codegen issue)

### Folding
- [x] `fold(initial:combine:)`
- [x] `reduce(combine:)`
- [x] `tryFold(initial:combine:)`
- [x] `tryForEach(action:)`

### Iteration
- [x] `forEach(action:)` (known limitation — captured variable assignment)

### Predicates
- [x] `any(predicate:)`
- [x] `all(predicate:)`

### Searching
- [x] `find(predicate:)`
- [x] `position(predicate:)`
- [x] `nth(n:)`
- [x] `last()`
- [x] `first()`

### Equatable Extension
- [x] `contains(element:)`

### Comparable Extension
- [x] `min()`
- [x] `max()`
- [x] `sorted()`
- [x] `minBy(key:)`
- [x] `maxBy(key:)`
- [x] `isSorted()`
- [x] `isSortedDescending()`
- [x] `isSorted(by:)`
- [x] `isSortedBy(key:)`

### Numeric Extensions
- [x] `sum()`
- [x] `product()`

### Nested Iterator Extension
- [x] `flatten()` (known limitation — type inference for map + flatten chain)

### DoubleEndedIterator
- [ ] `rev()` — no stdlib iterator implements DoubleEndedIterator yet

---

## Char

- [x] `init(value:)`
- [x] `value()`
- [x] `isAscii()`
- [x] `isAlphabetic()`
- [x] `isDigit()`
- [x] `isAlphanumeric()`
- [x] `isWhitespace()`
- [x] `isControl()`
- [x] `isUppercase()`
- [x] `isLowercase()`
- [x] `toUppercase()`
- [x] `toLowercase()`
- [x] `toTitlecase()`
- [x] `utf8Length()`
- [x] `digitValue()`
- [x] `fromDigit(d:)`
- [x] `equals(other:)`
- [x] `compare(other:)`
- [x] `hash(into:)`
- [x] `matches(other:)`

## Grapheme

- [x] `init(char:)`
- [x] `init(chars:)`
- [x] `chars()`
- [x] `charCount()`
- [x] `firstChar()`
- [x] `isAscii()`
- [x] `utf8Length()`
- [x] `equals(other:)`

---

## String Views

### BytesView
- [x] `count()`
- [x] `isEmpty()`
- [x] `byteAt(index:)`
- [x] `byteAtUnchecked(index:)`
- [x] `substring(from:to:)`
- [x] `substring(checked:to:)`
- [x] `iter()`

### CharsView
- [x] `iter()`
- [x] `count()`
- [x] `substring(from:to:)`
- [x] `substring(checked:to:)` (known limitation — empty range at non-zero offset returns None)

### GraphemesView
- [x] `iter()`
- [x] `count()`

### LinesView
- [x] `iter()`

---

## Float64

_Primary float type. Has a completely different interface from integer types (math functions,
trigonometry, IEEE 754 operations, etc.), so it gets a full detailed checklist._

### Constructors
- [x] `init()`
- [x] `init(floatLiteral:)`
- [x] `init(intLiteral:)`
- [x] `init(from: Int64)` (Convertible)
- [x] `init(from: Float32)` (Convertible)

### Static Constants
- [x] `zero`
- [x] `one`
- [x] `minValue`
- [x] `maxValue`
- [x] `minPositive`
- [x] `epsilon`
- [x] `infinity`
- [x] `nan`
- [x] `pi`
- [x] `e`
- [x] `tau`
- [x] `ln2`
- [x] `ln10`
- [x] `sqrt2`

### Classification Properties
- [x] `isNaN`
- [x] `isInfinite`
- [x] `isFinite`
- [x] `isNormal`
- [x] `isSubnormal`

### Sign Properties
- [x] `sign`
- [x] `isPositive`
- [x] `isNegative`
- [x] `isZero`

### Comparison
- [x] `equals(other:)`
- [x] `compare(other:)`

### Arithmetic
- [x] `add(other:)`
- [x] `subtract(other:)`
- [x] `multiply(other:)`
- [x] `divide(other:)`
- [x] `negate()`

### Basic Math
- [x] `abs()`
- [x] `floor()`
- [x] `ceil()`
- [x] `round()`
- [x] `trunc()`
- [x] `fract()`
- [x] `sqrt()`
- [x] `cbrt()`
- [x] `hypot(other:)`

### Exponential / Logarithmic
- [x] `exp()`
- [x] `exp2()`
- [x] `expm1()`
- [x] `ln()`
- [x] `ln1p()`
- [x] `log2()`
- [x] `log10()`
- [x] `log(base:)`
- [x] `pow(exponent:)`
- [x] `powi(exponent:)`

### Trigonometric
- [x] `sin()`
- [x] `cos()`
- [x] `tan()`
- [x] `asin()`
- [x] `acos()`
- [x] `atan()`
- [x] `atan2(x:)`
- [x] `sinCos()`

### Hyperbolic
- [x] `sinh()`
- [x] `cosh()`
- [x] `tanh()`
- [x] `asinh()`
- [x] `acosh()`
- [x] `atanh()`

### IEEE 754 Operations
- [x] `fma(a:b:)`
- [x] `copysign(from:)`
- [x] `nextUp()`
- [x] `nextDown()`
- [x] `remainder(dividingBy:)`

### Clamping / Interpolation
- [x] `clamp(min:max:)`
- [x] `lerp(to:t:)`

### Conversion
- [x] `toInt64()`
- [x] `toFloat32()`
- [x] `parse(string:)`

### Protocol Methods
- [x] `format(options:)`

---

## Float32

_Generated from the same template as Float64. Shared behavior (arithmetic, math functions,
trig, hyperbolic, IEEE 754, comparison, protocol methods) is covered by Float64 tests.
Only boundary-specific and conversion-specific items are listed here._

### Boundary-Specific Constants
- [x] `minValue` (different from Float64)
- [x] `maxValue` (different from Float64)
- [x] `minPositive` (different from Float64)
- [x] `epsilon` (different from Float64)

### Precision-Sensitive Behavior
- [x] Classification near Float32 subnormal boundary (`isNormal`, `isSubnormal`)
- [x] Precision loss in trig/math functions at Float32 resolution
- [x] `round()` / `trunc()` behavior near Float32 max

### Conversion
- [x] `init(from: Float64)` (Convertible — narrowing conversion)
- [x] `init(from: Int64)` (Convertible)
- [x] `toInt64()`
- [x] `toFloat64()`
- [x] `parse(string:)` — with Float32-range values and out-of-range values

---

## Signed Integer Types (Int8, Int16, Int32)

_All signed integer types are generated from the same template as Int64 and share identical
method signatures. Int64 (above) is the reference type with full test coverage. For Int8,
Int16, and Int32, only boundary-specific and bit-width-specific behavior needs per-type
testing. Shared behavior (basic arithmetic, bitwise ops, comparison, hash, format, compound
assignment, stepping, extended arithmetic, clamping) is covered by Int64 tests._

### Int8

#### Boundaries & Constants
- [x] `minValue` (-128)
- [x] `maxValue` (127)
- [x] `bitWidth` (8)

#### Overflow / Boundary Behavior
- [x] `addChecked(other:)` — overflow at 127
- [x] `subtractChecked(other:)` — underflow at -128
- [x] `multiplyChecked(other:)` — overflow near boundaries
- [x] `negateChecked()` — overflow at -128 (no positive 128)
- [x] `absChecked()` — overflow at -128
- [x] `addSaturating(other:)` — clamps to -128..127
- [x] `subtractSaturating(other:)` — clamps to -128..127
- [x] `multiplySaturating(other:)` — clamps to -128..127
- [x] `negateSaturating()` — -128 saturates to 127
- [x] `absSaturating()` — -128 saturates to 127

#### Bit-Width-Specific
- [x] `byteSwapped` (identity for single-byte type)
- [x] `leadingZeros` — relative to 8-bit width
- [x] `trailingZeros` — relative to 8-bit width
- [x] `rotateLeft(by:)` — 8-bit rotation
- [x] `rotateRight(by:)` — 8-bit rotation

#### Conversion
- [x] `init(from:)` — from Int64 and other integer types
- [x] `parse(string:)` — with Int8-range values and out-of-range values

### Int16

#### Boundaries & Constants
- [x] `minValue` (-32768)
- [x] `maxValue` (32767)
- [x] `bitWidth` (16)

#### Overflow / Boundary Behavior
- [x] `addChecked(other:)` — overflow at 32767
- [x] `subtractChecked(other:)` — underflow at -32768
- [x] `multiplyChecked(other:)` — overflow near boundaries
- [x] `negateChecked()` — overflow at -32768
- [x] `absChecked()` — overflow at -32768
- [x] `addSaturating(other:)` — clamps to -32768..32767
- [x] `subtractSaturating(other:)` — clamps to -32768..32767
- [x] `multiplySaturating(other:)` — clamps to -32768..32767
- [x] `negateSaturating()` — -32768 saturates to 32767
- [x] `absSaturating()` — -32768 saturates to 32767

#### Bit-Width-Specific
- [x] `byteSwapped` — 2-byte swap
- [x] `leadingZeros` — relative to 16-bit width
- [x] `rotateLeft(by:)` — 16-bit rotation
- [x] `rotateRight(by:)` — 16-bit rotation

#### Conversion
- [x] `init(from:)` — from Int64 and other integer types
- [x] `parse(string:)` — with Int16-range values and out-of-range values

### Int32

#### Boundaries & Constants
- [x] `minValue` (-2147483648)
- [x] `maxValue` (2147483647)
- [x] `bitWidth` (32)

#### Overflow / Boundary Behavior
- [x] `addChecked(other:)` — overflow at 2147483647
- [x] `subtractChecked(other:)` — underflow at -2147483648
- [x] `multiplyChecked(other:)` — overflow near boundaries
- [x] `negateChecked()` — overflow at -2147483648
- [x] `absChecked()` — overflow at -2147483648
- [x] `addSaturating(other:)` — clamps to -2147483648..2147483647
- [x] `subtractSaturating(other:)` — clamps to -2147483648..2147483647
- [x] `multiplySaturating(other:)` — clamps to -2147483648..2147483647
- [x] `negateSaturating()` — -2147483648 saturates to 2147483647
- [x] `absSaturating()` — -2147483648 saturates to 2147483647

#### Bit-Width-Specific
- [x] `byteSwapped` — 4-byte swap
- [x] `leadingZeros` — relative to 32-bit width
- [x] `rotateLeft(by:)` — 32-bit rotation
- [x] `rotateRight(by:)` — 32-bit rotation

#### Conversion
- [x] `init(from:)` — from Int64 and other integer types
- [x] `parse(string:)` — with Int32-range values and out-of-range values

---

## Unsigned Integer Types (UInt8, UInt16, UInt32, UInt64)

_All unsigned integer types are generated from the same template. They share the same
interface as signed integers except: no `negate()`, `negateChecked()`, `negateSaturating()`,
`abs()`, `absChecked()`, `absSaturating()`. Shared behavior (basic arithmetic, bitwise ops,
comparison, hash, format, compound assignment, stepping, extended arithmetic, clamping) is
covered by Int64 tests. Only boundary-specific and unsigned-specific items are listed._

### UInt8

#### Boundaries & Constants
- [x] `minValue` (0)
- [x] `maxValue` (255)
- [x] `bitWidth` (8)

#### Overflow / Boundary Behavior
- [x] `addChecked(other:)` — overflow at 255
- [x] `subtractChecked(other:)` — underflow at 0
- [x] `multiplyChecked(other:)` — overflow near 255
- [x] `addSaturating(other:)` — clamps to 0..255
- [x] `subtractSaturating(other:)` — clamps to 0 (no negative)
- [x] `multiplySaturating(other:)` — clamps to 255

#### Unsigned-Specific
- [ ] Verify `negate()` is absent (compile error or not available)
- [ ] Verify `abs()` is absent
- [x] Subtraction wrapping behavior (e.g., 0 - 1 wraps to 255)

#### Bit-Width-Specific
- [x] `byteSwapped` (identity for single-byte type)
- [x] `leadingZeros` — relative to 8-bit width
- [x] `rotateLeft(by:)` — 8-bit rotation
- [x] `rotateRight(by:)` — 8-bit rotation

#### Conversion
- [x] `init(from:)` — from Int64 and other integer types
- [x] `parse(string:)` — with UInt8-range values and out-of-range values

### UInt16

#### Boundaries & Constants
- [x] `minValue` (0)
- [x] `maxValue` (65535)
- [x] `bitWidth` (16)

#### Overflow / Boundary Behavior
- [x] `addChecked(other:)` — overflow at 65535
- [x] `subtractChecked(other:)` — underflow at 0
- [x] `multiplyChecked(other:)` — overflow near 65535
- [x] `addSaturating(other:)` — clamps to 0..65535
- [x] `subtractSaturating(other:)` — clamps to 0
- [x] `multiplySaturating(other:)` — clamps to 65535

#### Unsigned-Specific
- [ ] Verify `negate()` / `abs()` are absent
- [x] Subtraction wrapping behavior at 0

#### Bit-Width-Specific
- [x] `byteSwapped` — 2-byte swap
- [x] `leadingZeros` — relative to 16-bit width
- [x] `rotateLeft(by:)` — 16-bit rotation
- [x] `rotateRight(by:)` — 16-bit rotation

#### Conversion
- [x] `init(from:)` — from Int64 and other integer types
- [x] `parse(string:)` — with UInt16-range values and out-of-range values

### UInt32

#### Boundaries & Constants
- [x] `minValue` (0)
- [x] `maxValue` (4294967295)
- [x] `bitWidth` (32)

#### Overflow / Boundary Behavior
- [x] `addChecked(other:)` — overflow at 4294967295
- [x] `subtractChecked(other:)` — underflow at 0
- [x] `multiplyChecked(other:)` — overflow near 4294967295
- [x] `addSaturating(other:)` — clamps to 0..4294967295
- [x] `subtractSaturating(other:)` — clamps to 0
- [x] `multiplySaturating(other:)` — clamps to 4294967295

#### Unsigned-Specific
- [ ] Verify `negate()` / `abs()` are absent
- [x] Subtraction wrapping behavior at 0

#### Bit-Width-Specific
- [x] `byteSwapped` — 4-byte swap
- [x] `leadingZeros` — relative to 32-bit width
- [x] `rotateLeft(by:)` — 32-bit rotation
- [x] `rotateRight(by:)` — 32-bit rotation

#### Conversion
- [x] `init(from:)` — from Int64 and other integer types
- [x] `parse(string:)` — with UInt32-range values and out-of-range values

### UInt64

#### Boundaries & Constants
- [x] `minValue` (0)
- [x] `maxValue` (18446744073709551615)
- [x] `bitWidth` (64)

#### Overflow / Boundary Behavior
- [x] `addChecked(other:)` — overflow at max
- [x] `subtractChecked(other:)` — underflow at 0
- [x] `multiplyChecked(other:)` — overflow near max
- [x] `addSaturating(other:)` — clamps to 0..max
- [x] `subtractSaturating(other:)` — clamps to 0
- [x] `multiplySaturating(other:)` — clamps to max

#### Unsigned-Specific
- [ ] Verify `negate()` / `abs()` are absent
- [x] Subtraction wrapping behavior at 0

#### Bit-Width-Specific
- [x] `byteSwapped` — 8-byte swap (same width as Int64, but unsigned)
- [x] `leadingZeros` — relative to 64-bit width
- [x] `rotateLeft(by:)` — 64-bit rotation
- [x] `rotateRight(by:)` — 64-bit rotation

#### Conversion
- [x] `init(from:)` — from Int64 and other integer types
- [x] `parse(string:)` — with UInt64-range values and out-of-range values

---

## IO Module

### File

#### Constructors
- [ ] `open(path:)`
- [ ] `create(path:)`
- [ ] `openReadWrite(path:)`
- [ ] `openAppend(path:)`
- [ ] `createNew(path:)`

#### Read/Write Methods
- [ ] `read(into:)`
- [ ] `write(from:)`
- [ ] `flush()`

#### Seek Methods
- [ ] `seek(to:)`
- [ ] `position()`
- [ ] `rewind()`

#### Low-level
- [ ] `rawFd()`

### Stdin
- [ ] `init()`
- [ ] `read(into:)`

### Stdout
- [ ] `init()`
- [ ] `write(from:)`
- [ ] `flush()`

### Stderr
- [ ] `init()`
- [ ] `write(from:)`
- [ ] `flush()`

### Free Functions (stdio)
- [ ] `print(value:)`
- [ ] `println(value:)`
- [ ] `printlnEmpty()`
- [ ] `eprint(value:)`
- [ ] `eprintln(value:)`
- [ ] `readLine()`
- [ ] `prompt(message:)`

### Free Functions (file convenience)
- [ ] `readFileString(path:)`
- [ ] `readFileBytes(path:)`
- [ ] `writeFileString(path:content:)`
- [ ] `writeFileBytes(path:content:)`
- [ ] `appendFileString(path:content:)`
- [ ] `appendFileBytes(path:content:)`

### Read Protocol Implementations

#### Empty
- [x] `init()`
- [x] `read(into:)`

#### Repeat
- [x] `init(byte:)`
- [x] `read(into:)`

#### Cursor
- [x] `init(data:)`
- [x] `read(into:)`
- [x] `position()`
- [x] `setPosition(to:)`

### Read Free Functions
- [x] `readByte(reader:)`
- [x] `readAll(reader:into:)`
- [ ] `readExact(reader:into:)`

### Write Protocol Implementations

#### Sink
- [x] `init()`
- [x] `write(from:)`
- [x] `flush()`

#### Buffer (io)
- [x] `init()`
- [x] `init(capacity:)`
- [x] `write(from:)`
- [x] `flush()`
- [x] `count()`
- [x] `isEmpty()`
- [x] `clear()`
- [x] `asSlice()`
- [x] `toArray()`
- [x] `toString()`

### Write Free Functions
- [x] `writeAll(writer:from:)`
- [x] `writeByte(writer:byte:)`
- [x] `writeStr(writer:s:)`
- [x] `writeLine(writer:s:)`

### Error
- [x] `init(code:)`
- [ ] `last()`
- [x] `description()`
- [x] `errno()`
- [x] `notFound()`
- [x] `permissionDenied()`
- [x] `alreadyExists()`
- [x] `invalidInput()`
- [x] `wouldBlock()`
- [x] `interrupted()`
- [x] `brokenPipe()`

---

## Memory Module

### RawPointer
- [ ] `init(raw:)`
- [x] `init(address:)`
- [x] `nilPointer()`
- [x] `address`
- [x] `isNull`
- [x] `cast()`
- [x] `offset(by:)`
- [x] `equals(other:)`
- [ ] `hash(into:)`

### Pointer[T]
- [ ] `init(raw:)`
- [ ] `init(to:)`
- [x] `nullPointer()`
- [ ] `pointee` (get/set)
- [x] `address`
- [x] `isNull`
- [x] `read()`
- [x] `write(value:)`
- [x] `offset(by:)`
- [x] `asRaw()`
- [ ] `cast()`
- [x] `equals(other:)`
- [ ] `hash(into:)`

### Slice[T]
- [x] `init(pointer:count:)`
- [x] `count`
- [x] `isEmpty`
- [x] `pointer`
- [x] `subscript(safe:)`
- [x] `subscript(unchecked:)` (get/set)
- [x] `slice(from:to:)`
- [x] `iter()`
- [x] `first()`
- [x] `last()`
- [ ] `equals(other:)`

### Buffer[T, A]
- [x] `init(capacity:allocator:)`
- [x] `capacity`
- [x] `pointer`
- [x] `read(unchecked:)`
- [x] `write(unchecked:value:)`
- [x] `read(at:)`
- [x] `write(at:value:)`
- [x] `resize(to:)`
- [x] `asSlice()`
- [x] `slice(from:to:)`

### Allocator Protocol
- [x] `SystemAllocator.init()`
- [x] `allocate(layout:)`
- [x] `deallocate(ptr:layout:)`
- [x] `reallocate(ptr:oldLayout:newLayout:)`
