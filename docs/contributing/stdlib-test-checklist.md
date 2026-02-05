# Standard Library Test Coverage Checklist

Methods marked `[x]` have test coverage in `lib/kestrel-test-suite/tests/stdlib/`.
Methods marked `[ ]` are not yet tested.
Items marked "(known limitation)" have tests that fail due to compiler bugs.

---

## Array[T]

### Constructors
- [x] `init()`
- [x] `init(capacity:)`
- [ ] `init(repeating:count:)` — needs Cloneable on T
- [x] `init(from:)`
- [x] `init(count:generator:)` (known limitation — signature collision with internal init)

### Properties
- [x] `count`
- [x] `capacity`
- [x] `isEmpty`
- [ ] `indices`

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
- [ ] `asPointer()`
- [ ] `asSlice()`
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
- [ ] `shuffle(using:)`
- [x] `shuffled()`
- [ ] `shuffled(using:)`

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
- [ ] `hash(into:)`
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
- [ ] `deepClone()`
- [ ] `sumValues()`

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
- [ ] `compactMap(transform:)`
- [ ] `flatMap(transform:)`

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
- [ ] `hash(into:)`
- [ ] `format(options:)`

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
- [ ] `intersperseWith(separator:)`
- [x] `cycle()` (known limitation — ArrayIterator lacks Cloneable)

### Collecting
- [x] `collect()`
- [x] `count()`
- [x] `unzip()` (known limitation — AssociatedTypeProjection codegen issue)

### Folding
- [x] `fold(initial:combine:)`
- [x] `reduce(combine:)`
- [ ] `tryFold(initial:combine:)`
- [ ] `tryForEach(action:)`

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
- [ ] `isSorted(by:)`
- [ ] `isSortedBy(key:)`

### Numeric Extensions
- [x] `sum()`
- [x] `product()`

### Nested Iterator Extension
- [x] `flatten()` (known limitation — type inference for map + flatten chain)

### DoubleEndedIterator
- [ ] `rev()`

---

## Char

- [ ] `init(value:)`
- [ ] `value()`
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
- [ ] `hash(into:)`
- [x] `matches(other:)`

## Grapheme

- [x] `init(char:)`
- [x] `init(chars:)`
- [ ] `chars()`
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

## Float64 (entirely untested)

_Not yet inventoried - needs review._

## Other Numeric Types (Int8, Int16, Int32, UInt8, UInt16, UInt32, UInt64, Float32)

_Not yet inventoried - share same method signatures as Int64/Float64._

## IO Module (entirely untested)

_File, stdin/stdout/stderr, Read/Write protocols - needs review._

## Memory Module (partially tested via RcBox)

### Buffer
_Not yet inventoried - needs review._

### Pointer
_Not yet inventoried - needs review._
