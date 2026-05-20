# std.text

## Submodules

- [`std.text.unicode`](std.text.unicode.md)

## enum `Alignment`

```kestrel
public enum Alignment
```

Horizontal alignment of formatted output within a fixed field width.

Pairs with `FormatOptions.width` and `FormatOptions.fill` to position
shorter values inside the requested column. When the value is already at
least as wide as the field, alignment has no visible effect. The
formatter for `String` is the canonical consumer; numeric and boolean
formatters honour the same convention.

### Examples

```
var opts = FormatOptions();
opts.width = .Some(8);
opts.alignment = .Right;
"ab".format(options: opts);  // "      ab"
opts.alignment = .Center;
"ab".format(options: opts);  // "   ab   "
```

_Defined in `lang/std/text/format.ks`._

### Members

#### case `Center`

```kestrel
case Center
```

Pad on both sides; if the padding is odd, the extra space goes on the right.

_Defined in `lang/std/text/format.ks`._

#### case `Left`

```kestrel
case Left
```

Pad on the right; the value sits flush against the left edge of the field.

_Defined in `lang/std/text/format.ks`._

#### case `Right`

```kestrel
case Right
```

Pad on the left; the value sits flush against the right edge of the field.

_Defined in `lang/std/text/format.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Alignment) -> Bool
```

Returns true if both cases are the same variant.

Equality is structural — there are no payloads. Used by the
`Equatable` conformance so `FormatOptions.isEqual` can fall through
without payload comparisons.

##### Examples

```
Alignment.Left.isEqual(to: .Left);    // true
Alignment.Left.isEqual(to: .Center);  // false
```

_Defined in `lang/std/text/format.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(Alignment) -> Bool
```

Pattern-match form of equality — delegates to `isEqual`.

Lets `Alignment` appear in `match` patterns against another value.

##### Examples

```
Alignment.Right.matches(.Right);  // true
```

_Defined in `lang/std/text/format.ks`._

## struct `ByteIndex`

```kestrel
public struct ByteIndex { /* private fields */ }
```

A typed wrapper for a byte position within a `String`.

`ByteIndex` exists so that APIs taking string positions can refuse
raw `Int64`s, which removes the "is this a byte offset or a char
offset?" ambiguity at the call site. The wrapped `value` is a
plain UTF-8 byte offset; arithmetic is the caller's responsibility.

### Representation

A single `Int64` field.

_Defined in `lang/std/text/views.ks`._

### Members

#### initializer `From Value`

```kestrel
public init(Int64)
```

Wraps a raw byte offset.

_Defined in `lang/std/text/views.ks`._

#### function `advance`

```kestrel
public func advance(by: Int64) -> ByteIndex
```

Advances by `n` bytes. Pure arithmetic — no string needed.

_Defined in `lang/std/text/slice.ks`._

#### field `value`

```kestrel
public var value: Int64
```

The wrapped byte offset.

_Defined in `lang/std/text/views.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: ByteIndex) -> Bool
```

Returns true if the two indices wrap the same byte offset.

_Defined in `lang/std/text/views.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(ByteIndex) -> Ordering
```

Compares two byte indices by their wrapped offsets.

_Defined in `lang/std/text/views.ks`._

### Implements `BytesIndex`

#### typealias `BytesYield`

```kestrel
type BytesYield = UInt8
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytes`

```kestrel
public func readBytes(from: BytesView) -> UInt8
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesChecked`

```kestrel
public func readBytesChecked(from: BytesView) -> UInt8?
```

_Defined in `lang/std/text/views.ks`._

#### function `readBytesUnchecked`

```kestrel
public func readBytesUnchecked(from: BytesView) -> UInt8
```

_Defined in `lang/std/text/views.ks`._

## struct `BytesIterator`

```kestrel
public struct BytesIterator { /* private fields */ }
```

Single-pass forward iterator over the raw UTF-8 bytes of a string.

Yielded by `BytesView.iter()`. Walks the underlying buffer one byte
at a time and returns each as a `UInt8`. The iterator holds a raw
pointer into the source string's storage; do not mutate the source
while iterating.

### Examples

```
var it = "hi".bytes.iter();
it.next();  // Some(104)  // 'h'
it.next();  // Some(105)  // 'i'
it.next();  // None
```

### Representation

A `(ptr, length, index)` triple: a raw pointer to the buffer plus
the cursor and total-length pair the iterator advances through.

### Memory Model

Value type. The pointer aliases string storage; do not retain the
iterator across mutations of the source `String`.

_Defined in `lang/std/text/views.ks`._

### Members

#### initializer `From Pointer`

```kestrel
public init(ptr: lang.ptr[lang.i8], length: Int64, index: Int64)
```

Constructs a bytes iterator from a raw pointer, total byte count, and starting offset.

Prefer `String.bytes.iter()` over calling this directly.

##### Safety

`ptr` must point to at least `length` valid bytes; `index` must
be in `0..=length`.

_Defined in `lang/std/text/views.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = UInt8
```

The element type yielded by `next()` — always `UInt8`.

_Defined in `lang/std/text/views.ks`._

#### function `next`

```kestrel
public mutating func next() -> UInt8?
```

Returns the next byte, or `None` once `index` reaches `length`.

Each call reads one byte and advances the cursor by 1.

_Defined in `lang/std/text/views.ks`._

## protocol `BytesSubstringIndex`

```kestrel
public protocol BytesSubstringIndex
```

Range-only index for `BytesView.substring`. Conformed by every
range type so a single generic `substring` can dispatch over all of
them. Single-element indexes (`Int64`) deliberately don't conform —
`substring` is range-flavored only.

_Defined in `lang/std/text/views.ks`._

### Members

#### function `readBytesSubstring`

```kestrel
func readBytesSubstring(from: BytesView) -> String
```

_Defined in `lang/std/text/views.ks`._

## struct `BytesView`

```kestrel
public struct BytesView { /* private fields */ }
```

A read-only view over the raw UTF-8 bytes of a `String`.

Returned by `String.bytes`. Provides O(1) byte indexing and
iteration; the bytes are returned as `UInt8` exactly as they sit
in memory. The most common reason to reach for `BytesView` is to
perform byte-level operations (substring searches, hashing) without
paying the cost of UTF-8 decoding. For code-point or grapheme
iteration, see `CharsView` / `GraphemesView`.

### Examples

```
let s = "hi";
s.bytes.count;               // 2
s.bytes(checked: 0);         // Some(104)
s.bytes(checked: 5);         // None (out of bounds)
```

### Representation

A `(ptr, length)` pair pointing at the source string's UTF-8 buffer.

### Memory Model

Borrows the source string's storage; the view is invalidated by any
mutation that reallocates that buffer. Copy out to a new `String`
(e.g. via `substring`) if you need an independent value.

_Defined in `lang/std/text/views.ks`._

### Members

#### subscript `Checked Index`

```kestrel
public subscript[I](checked: I) -> I.BytesYield? { get }
```

Reads at `index`, returning `.None` on out-of-bounds.

_Defined in `lang/std/text/views.ks`._

#### subscript `Clamping`

```kestrel
public subscript[I](clamped: I) -> I.BytesClampedYield { get }
```

Reads at `index` with bounds saturated to `[0, count)`. Single-
byte indexes yield `UInt8?` (`None` on empty view); range indexes
yield `BytesView` (always valid, possibly empty).

_Defined in `lang/std/text/views.ks`._

#### initializer `From Slice`

```kestrel
public init(slice: StringSlice)
```

Constructs a bytes view backed by the given string slice.
The view retains shared ownership of the underlying bytes.

_Defined in `lang/std/text/views.ks`._

#### subscript `Indexed Byte / Sub-view`

```kestrel
public subscript[I](I) -> I.BytesYield { get }
```

Reads a single byte (`UInt8`) for `Int64` indexes, or a zero-copy
sub-view (`BytesView`) for `Range[Int64]` / `ClosedRange[Int64]`.
Panics on out-of-bounds. Range slicing does not validate UTF-8
boundaries — call `.toString()` on the sub-view if you need an
owned `String` (which validates).

_Defined in `lang/std/text/views.ks`._

#### subscript `Unchecked Index`

```kestrel
public subscript[I](unchecked: I) -> I.BytesYield { get }
```

Reads at `index` with no bounds check.

##### Safety

Caller must guarantee `0 <= index < count`. For ranges, the
endpoints must be in `0..=count`; otherwise the resulting
sub-view aliases out-of-bounds memory.

_Defined in `lang/std/text/views.ks`._

#### subscript `Wrapping`

```kestrel
public subscript[I](wrapped: I) -> I.BytesWrappedYield { get }
```

Reads at `index` with modulo wrap-around. Negative indices wrap
from the end: `view.bytes(wrapped: -1)` reads the last byte.
Returns `None` on an empty view.

_Defined in `lang/std/text/views.ks`._

#### function `asRaw`

```kestrel
public func asRaw() -> lang.ptr[lang.i8]
```

Returns the raw pointer to the underlying byte buffer.

Intended for FFI bridges; the pointer is only valid as long as
the source string remains live and unmutated.

_Defined in `lang/std/text/views.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Number of bytes in the view.

O(1). This is **byte** count, not character count — see
`CharsView.count` for the latter (which is O(n)).

_Defined in `lang/std/text/views.ks`._

#### field `endIndex`

```kestrel
public var endIndex: ByteIndex { get }
```

Byte index one past the last byte.

_Defined in `lang/std/text/views.ks`._

#### function `firstIndex`

```kestrel
public func firstIndex(of: UInt8) -> ByteIndex?
```

Returns the index of the first occurrence of `byte`, or `.None`.

_Defined in `lang/std/text/views.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` if the view spans zero bytes.

_Defined in `lang/std/text/views.ks`._

#### function `lastIndex`

```kestrel
public func lastIndex(of: UInt8) -> ByteIndex?
```

Returns the index of the last occurrence of `byte`, or `.None`.

_Defined in `lang/std/text/views.ks`._

#### function `slice`

```kestrel
public func slice(from: ByteIndex, to: ByteIndex) -> StringSlice
```

Returns a `StringSlice` covering the byte range `[start, end)`.

_Defined in `lang/std/text/views.ks`._

#### field `startIndex`

```kestrel
public var startIndex: ByteIndex { get }
```

Byte index of the first byte.

_Defined in `lang/std/text/views.ks`._

#### function `substring`

```kestrel
public func substring[__opaque_0](__opaque_0) -> String where __opaque_0: BytesSubstringIndex
```

Convenience: dispatches to a `BytesSubstringIndex` to produce
an owned `String` covering the requested byte range. Equivalent
to `self(range).toString()` for both `Range[Int64]` and
`ClosedRange[Int64]`.

_Defined in `lang/std/text/views.ks`._

#### function `toString`

```kestrel
public func toString() -> String
```

Materializes the view as an owned `String`. Copies all bytes
into a fresh buffer; the result is independent of the source.
Bytes are copied verbatim — no UTF-8 validation is performed.

_Defined in `lang/std/text/views.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = UInt8
```

The element type yielded by iteration — always `UInt8`.

_Defined in `lang/std/text/views.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = BytesIterator
```

The iterator type returned by `iter()`.

_Defined in `lang/std/text/views.ks`._

#### function `iter`

```kestrel
public func iter() -> BytesIterator
```

Returns a `BytesIterator` positioned at byte 0.

Required by `Iterable`. Each call produces a fresh iterator —
the view is reusable.

_Defined in `lang/std/text/views.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> BytesView
```

_Defined in `lang/std/text/views.ks`._

## struct `Char`

```kestrel
public struct Char { /* private fields */ }
```

A single Unicode scalar value (code point in `0..=0x10FFFF`, surrogates excluded).

`Char` is the unit yielded by `String.chars` / `CharsView`; iterating
graphemes (`String.graphemes`) instead returns `Grapheme` clusters
that may comprise multiple `Char`s. The character-literal syntax
constructs values directly: `'a'`, `'\n'`, `'\u{1F600}'`. For the
raw byte representation, see `utf8Length()` and the free
`encodeUtf8` / `decodeUtf8` functions.

### Examples

```
let a: Char = 'a';
a.isAsciiLetter;    // true
a.utf8Length();      // 1
let smile: Char = '\u{1F600}';
smile.utf8Length();  // 4
```

### Representation

A single `UInt32` holding the scalar value. Comparison and hashing
operate on that integer directly.

_Defined in `lang/std/text/char.ks`._

### Members

#### initializer `Char Literal`

```kestrel
public init(charLiteral: lang.i32)
```

Compiler-emitted constructor for character literals.

Called when you write `'a'`, `'\n'`, `'\u{1F600}'`. Not intended
for direct use — `Char(value:)` is the user-facing constructor.

##### Examples

```
let c: Char = 'a';  // lowers to Char(charLiteral: ...)
```

_Defined in `lang/std/text/char.ks`._

#### initializer `From Digit`

```kestrel
public init(fromDigit: UInt32)
```

Returns the ASCII digit `Char` for a numeric value `0`–`9`, `null` otherwise.

Inverse of `digitValue`. Values outside `0..=9` return `null`.

##### Examples

```
Char(fromDigit: 7);   // Some('7')
Char(fromDigit: 12);  // None
```

_Defined in `lang/std/text/char.ks`._

#### initializer `From Value`

```kestrel
public init(UInt32)
```

Returns a `Char` if the value is a valid Unicode scalar, `null` otherwise.
Rejects values > U+10FFFF and the surrogate range U+D800..U+DFFF.

##### Examples

```
let c = Char(65);      // Some('A')
let bad = Char(0xD800); // None (surrogate)
```

_Defined in `lang/std/text/char.ks`._

#### initializer `Unchecked`

```kestrel
public init(unchecked: UInt32)
```

Wraps a raw `UInt32` scalar value as a `Char` without validation.

##### Safety

The caller must ensure `value` is a valid Unicode scalar
(0..=0x10FFFF, excluding surrogates U+D800..U+DFFF).

_Defined in `lang/std/text/char.ks`._

#### function `digitValue`

```kestrel
public func digitValue() -> UInt32?
```

Returns the numeric value `0`–`9` for ASCII digits, otherwise `None`.

Inverse of `fromDigit`. Non-ASCII digit characters return `None`
— match `isAsciiDigit` semantics.

##### Examples

```
'7'.digitValue();  // Some(7)
'a'.digitValue();  // None
```

_Defined in `lang/std/text/char.ks`._

#### function `hasLowercaseExpansion`

```kestrel
public func hasLowercaseExpansion() -> Bool
```

Returns true if the lowercase form is multi-char.

Rare in practice but exists for full Unicode round-tripping.

_Defined in `lang/std/text/char.ks`._

#### function `hasTitlecaseExpansion`

```kestrel
public func hasTitlecaseExpansion() -> Bool
```

Returns true if the titlecase form is multi-char.

_Defined in `lang/std/text/char.ks`._

#### function `hasUppercaseExpansion`

```kestrel
public func hasUppercaseExpansion() -> Bool
```

Returns true if the uppercase form is multi-char (e.g. `ß` → `SS`).

When `true`, prefer `uppercaseExpansion()` over `uppercased()`
to avoid silently dropping characters.

##### Examples

```
'\u{00DF}'.hasUppercaseExpansion();  // true (ß)
'a'.hasUppercaseExpansion();         // false
```

_Defined in `lang/std/text/char.ks`._

#### field `isAscii`

```kestrel
public var isAscii: Bool { get }
```

Returns true if the scalar is in the ASCII range (`< 0x80`).

Cheap byte-range test; does not consult Unicode tables. For
"alphabetic by Unicode" use `unicode.toLowercase` round-tripping
or the property tables directly.

##### Examples

```
'A'.isAscii;          // true
'\u{00E9}'.isAscii;   // false (é)
```

_Defined in `lang/std/text/char.ks`._

#### field `isAsciiAlphanumeric`

```kestrel
public var isAsciiAlphanumeric: Bool { get }
```

Returns true for ASCII letters and ASCII digits.

Composition of `isAsciiLetter` and `isAsciiDigit`; same ASCII-only
caveats apply.

##### Examples

```
'a'.isAsciiAlphanumeric;  // true
'7'.isAsciiAlphanumeric;  // true
'_'.isAsciiAlphanumeric;  // false
```

_Defined in `lang/std/text/char.ks`._

#### field `isAsciiDigit`

```kestrel
public var isAsciiDigit: Bool { get }
```

Returns true for the ASCII digits `0`–`9`.

**ASCII-only.** Other Unicode digit categories (Arabic-Indic,
Devanagari, etc.) return `false`. See `digitValue()` for parsing
to numeric value.

##### Examples

```
'7'.isAsciiDigit;   // true
'a'.isAsciiDigit;   // false
```

_Defined in `lang/std/text/char.ks`._

#### field `isAsciiLetter`

```kestrel
public var isAsciiLetter: Bool { get }
```

Returns true for ASCII letters `A`–`Z` / `a`–`z`.

**ASCII-only.** Non-ASCII letters (e.g. `é`, `Ω`, `日`) return
`false` even though they are letters in Unicode. For the full
Unicode test, use the property tables in `std.text.unicode`.

##### Examples

```
'A'.isAsciiLetter;         // true
'\u{00E9}'.isAsciiLetter;  // false (é — non-ASCII)
'7'.isAsciiLetter;         // false
```

_Defined in `lang/std/text/char.ks`._

#### field `isAsciiLowercase`

```kestrel
public var isAsciiLowercase: Bool { get }
```

Returns true for ASCII lowercase letters `a`–`z`.

**ASCII-only.** Use `unicode.toLowercase` round-tripping for
general Unicode case tests.

##### Examples

```
'a'.isAsciiLowercase;   // true
'A'.isAsciiLowercase;   // false
```

_Defined in `lang/std/text/char.ks`._

#### field `isAsciiUppercase`

```kestrel
public var isAsciiUppercase: Bool { get }
```

Returns true for ASCII uppercase letters `A`–`Z`.

**ASCII-only.** Use `unicode.toUppercase` round-tripping for
general Unicode case tests.

##### Examples

```
'A'.isAsciiUppercase;         // true
'a'.isAsciiUppercase;         // false
'\u{00C9}'.isAsciiUppercase;  // false (É — non-ASCII)
```

_Defined in `lang/std/text/char.ks`._

#### field `isControl`

```kestrel
public var isControl: Bool { get }
```

Returns true for the C0 controls (`< U+0020`) and DEL (`U+007F`).

Does not include the C1 controls (`U+0080`–`U+009F`); add a
dedicated test if you need them.

##### Examples

```
'\n'.isControl;     // true
'\x7F'.isControl;   // true
'a'.isControl;      // false
```

_Defined in `lang/std/text/char.ks`._

#### field `isWhitespace`

```kestrel
public var isWhitespace: Bool { get }
```

Returns true for the common ASCII whitespace set: space, tab, LF, CR, form feed.

Does not include Unicode whitespace such as `U+00A0` (no-break
space) or `U+2028` (line separator). For Unicode-aware
whitespace, consult the property tables.

##### Examples

```
' '.isWhitespace;    // true
'\t'.isWhitespace;   // true
'\n'.isWhitespace;   // true
'a'.isWhitespace;    // false
```

_Defined in `lang/std/text/char.ks`._

#### function `lowercaseExpansion`

```kestrel
public func lowercaseExpansion() -> String
```

Returns the multi-char lowercase form as a `String`.

Empty string if no expansion exists.

_Defined in `lang/std/text/char.ks`._

#### function `lowercased`

```kestrel
public func lowercased() -> Char
```

Returns the lowercase form, using full Unicode case-mapping tables.

Locale-independent. For characters with multi-char lowercase
expansions, see `lowercaseExpansion()`.

##### Examples

```
'A'.lowercased();         // 'a'
'\u{0130}'.lowercased();  // 'i' (Turkish dotted I — first char only)
```

_Defined in `lang/std/text/char.ks`._

#### function `titlecaseExpansion`

```kestrel
public func titlecaseExpansion() -> String
```

Returns the multi-char titlecase form as a `String`.

Empty string if no expansion exists.

_Defined in `lang/std/text/char.ks`._

#### function `titlecased`

```kestrel
public func titlecased() -> Char
```

Returns the titlecase form, using full Unicode case-mapping tables.

Titlecase differs from uppercase for some characters — e.g.
ligatures like `ǳ` titlecase to `ǲ` (capital plus small) rather
than `Ǳ` (full uppercase).

##### Examples

```
'a'.titlecased();   // 'A'
```

_Defined in `lang/std/text/char.ks`._

#### function `toString`

```kestrel
public func toString() -> String
```

Converts this code point to an owned `String`.

_Defined in `lang/std/text/char.ks`._

#### function `uppercaseExpansion`

```kestrel
public func uppercaseExpansion() -> String
```

Returns the multi-char uppercase form as a `String`.

For characters without an expansion this returns the empty
string; use `hasUppercaseExpansion()` first to distinguish.

##### Examples

```
'\u{00DF}'.uppercaseExpansion();  // "SS"
'a'.uppercaseExpansion();         // ""
```

_Defined in `lang/std/text/char.ks`._

#### function `uppercased`

```kestrel
public func uppercased() -> Char
```

Returns the uppercase form, using full Unicode case-mapping tables.

For characters whose uppercase form is multiple `Char`s (e.g.
German `ß` → `SS`), this returns only the first `Char`. Use
`hasUppercaseExpansion()` plus `uppercaseExpansion()` to handle
those cases correctly. Locale-independent — does not perform
Turkish / Azeri tailoring.

##### Examples

```
'a'.uppercased();         // 'A'
'\u{00DF}'.uppercased();  // 'S' (first char of "SS"; see hasUppercaseExpansion)
```

_Defined in `lang/std/text/char.ks`._

#### function `utf8Length`

```kestrel
public func utf8Length() -> Int64
```

Returns how many UTF-8 bytes are required to encode this character (1–4).

Constant time — branches on the scalar value alone. Use this to
size buffers before calling `encodeUtf8`.

##### Examples

```
'a'.utf8Length();          // 1
'\u{00E9}'.utf8Length();   // 2 (é)
'\u{20AC}'.utf8Length();   // 3 (€)
'\u{1F600}'.utf8Length();  // 4 (😀)
```

_Defined in `lang/std/text/char.ks`._

#### function `value`

```kestrel
public func value() -> UInt32
```

Returns the raw Unicode scalar as a `UInt32`.

Useful for arithmetic on code points (e.g. `digitValue`'s offset
trick) or interop with APIs that take a numeric code point.

##### Examples

```
'A'.value();  // 65
'\u{1F600}'.value();  // 128512
```

_Defined in `lang/std/text/char.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Char) -> Bool
```

Returns true if both characters are the same Unicode scalar.

Pure scalar-value equality — no case folding, no normalization.

##### Examples

```
'a'.isEqual(to: 'a');  // true
'a'.isEqual(to: 'A');  // false
```

_Defined in `lang/std/text/char.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(Char) -> Ordering
```

Compares two characters by scalar value.

Yields code-point order, which agrees with byte order in UTF-8
(UTF-8 is order-preserving). Not the same as locale-aware
collation.

##### Examples

```
'a'.compare('b');  // Less
'b'.compare('a');  // Greater
'a'.compare('a');  // Equal
```

_Defined in `lang/std/text/char.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(Char) -> Bool
```

Pattern-match form of equality — delegates to `isEqual`.

_Defined in `lang/std/text/char.ks`._

### Implements `ExpressibleByCharLiteral`

#### initializer `Char Literal`

```kestrel
init(charLiteral: lang.i32)
```

Builds an instance from a character literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Hashes this character by writing its 4-byte scalar value to the hasher.

Uses native byte order — fine for in-process hash maps; do not
use the result for content-addressed storage.

_Defined in `lang/std/text/char.ks`._

### Implements `RangeMatchable`

#### function `isAtLeast`

```kestrel
public func isAtLeast(Char) -> Bool
```

Returns true if `self >= bound`. Used by `RangeMatchable` for `case 'a'...'z'`.

_Defined in `lang/std/text/char.ks`._

#### function `isAtMost`

```kestrel
public func isAtMost(Char) -> Bool
```

Returns true if `self <= bound`. Used by `RangeMatchable` for `case 'a'...'z'`.

_Defined in `lang/std/text/char.ks`._

#### function `isBelow`

```kestrel
public func isBelow(Char) -> Bool
```

Returns true if `self < bound`. Used by `RangeMatchable` for half-open patterns.

_Defined in `lang/std/text/char.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating StringBuilder, FormatOptions)
```

_Defined in `lang/std/text/char.ks`._

## struct `CharIndex`

```kestrel
public struct CharIndex { /* private fields */ }
```

A typed wrapper for a character position within a `String`.

Unlike `ByteIndex`, `CharIndex` carries the byte offset of the
underlying character — code-point indexing is O(n), so this
pre-resolved offset is what gets stored. Construct one by walking
the string yourself; the type is purely a tag for clarity.

### Representation

A single `Int64` field holding the byte offset of the character.

_Defined in `lang/std/text/views.ks`._

### Members

#### initializer `From Offset`

```kestrel
public init(Int64)
```

Wraps a pre-resolved byte offset for a character position.

_Defined in `lang/std/text/views.ks`._

#### function `advance`

```kestrel
public func advance(by: Int64, from: StringSlice) -> CharIndex
```

Advances by `n` code points. Requires the source string to
decode UTF-8 boundaries. O(n) in chars advanced.

_Defined in `lang/std/text/slice.ks`._

#### field `byteOffset`

```kestrel
public var byteOffset: Int64
```

The byte offset where the indexed character begins.

_Defined in `lang/std/text/views.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: CharIndex) -> Bool
```

Returns true if the two indices point at the same byte offset.

_Defined in `lang/std/text/views.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(CharIndex) -> Ordering
```

_Defined in `lang/std/text/slice.ks`._

### Implements `CharsIndex`

#### typealias `CharsYield`

```kestrel
type CharsYield = Char
```

_Defined in `lang/std/text/views.ks`._

#### function `readChars`

```kestrel
public func readChars(from: CharsView) -> Char
```

_Defined in `lang/std/text/views.ks`._

#### function `readCharsChecked`

```kestrel
public func readCharsChecked(from: CharsView) -> Char?
```

_Defined in `lang/std/text/views.ks`._

## struct `CharsIterator`

```kestrel
public struct CharsIterator { /* private fields */ }
```

Single-pass forward iterator over Unicode code points (`Char`).

Yielded by `CharsView.iter()` and consumed by `GraphemesIterator`.
On each `next()` call, decodes one UTF-8 character starting at the
current cursor and advances by its byte length. Invalid bytes are
skipped one at a time and surfaced as `U+FFFD` (the Unicode
replacement character) so the iteration always makes progress.

### Examples

```
var it = "hi".chars.iter();
it.next();  // Some('h')
it.next();  // Some('i')
it.next();  // None
```

### Representation

A `(ptr, length, byteIndex)` triple. `byteIndex` walks the buffer
in variable-width steps according to the UTF-8 encoding.

### Memory Model

Value type that aliases the source string's buffer. Do not retain
across mutations of the source `String`.

_Defined in `lang/std/text/views.ks`._

### Members

#### initializer `From Pointer`

```kestrel
public init(ptr: lang.ptr[lang.i8], length: Int64, byteIndex: Int64)
```

Constructs a chars iterator from a raw pointer, byte length, and starting byte offset.

Prefer `String.chars.iter()` over calling this directly.

##### Safety

`ptr` must point to `length` valid UTF-8 bytes; `byteIndex` must
be `0` or land on a UTF-8 boundary.

_Defined in `lang/std/text/views.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = Char
```

The element type yielded by `next()` — always `Char`.

_Defined in `lang/std/text/views.ks`._

#### function `next`

```kestrel
public mutating func next() -> Char?
```

Returns the next code point, or `None` when the buffer is exhausted.

On invalid UTF-8 the iterator yields the replacement character
`U+FFFD` and advances by one byte; this guarantees forward
progress without aborting.

_Defined in `lang/std/text/views.ks`._

## protocol `CharsSubstringIndex`

```kestrel
public protocol CharsSubstringIndex
```

Range-only index for `CharsView.substring`. See `BytesSubstringIndex`.

_Defined in `lang/std/text/views.ks`._

### Members

#### function `readCharsSubstring`

```kestrel
func readCharsSubstring(from: CharsView) -> String
```

_Defined in `lang/std/text/views.ks`._

## struct `CharsView`

```kestrel
public struct CharsView { /* private fields */ }
```

A view over the Unicode code points in a `String`.

Returned by `String.chars`. Iteration is O(1) per code point but
`count()` is O(n) because UTF-8 is variable-width. Range subscripts
are O(n) (the segment-walk dominates) but yield a zero-copy
`CharsView` sub-view — call `.toString()` to materialize an owned
`String`. To index in O(1), use `BytesView` and convert byte offsets
back yourself.

### Examples

```
let s = "héllo";
s.chars.count;                       // 5 (code points)
s.bytes.count;                       // 6 (bytes — 'é' is 2 bytes)
s.chars(0..<4).toString();           // "héll"
s.chars.substring(0..<4);            // "héll"
```

### Representation

A `(ptr, length)` pair, plus the on-demand UTF-8 decoder.

### Memory Model

Borrows the source string's buffer. Invalidated by any mutation
that reallocates the storage.

_Defined in `lang/std/text/views.ks`._

### Members

#### subscript `Checked Index`

```kestrel
public subscript[I](checked: I) -> I.CharsYield? { get }
```

Reads at `index`, returning `.None` on out-of-bounds.

_Defined in `lang/std/text/views.ks`._

#### subscript `Clamping`

```kestrel
public subscript[I](clamped: I) -> I.CharsClampedYield { get }
```

Reads at `index` with bounds saturated to `[0, count)`. Single-
char indexes yield `Char?` (`None` on empty view); range indexes
yield `CharsView` (always valid, possibly empty).

_Defined in `lang/std/text/views.ks`._

#### initializer `From Slice`

```kestrel
public init(slice: StringSlice)
```

Constructs a chars view backed by the given string slice.
The view retains shared ownership of the underlying bytes.

_Defined in `lang/std/text/views.ks`._

#### subscript `Indexed Char / Sub-view`

```kestrel
public subscript[I](I) -> I.CharsYield { get }
```

Reads a single code point (`Char`) for `Int64` indexes, or a
zero-copy `CharsView` sub-view for `Range[Int64]` /
`ClosedRange[Int64]`. All access is O(n) because UTF-8 is
variable-width. Panics on out-of-bounds.

_Defined in `lang/std/text/views.ks`._

#### subscript `Wrapping`

```kestrel
public subscript[I](wrapped: I) -> I.CharsWrappedYield { get }
```

Reads at `index` with modulo wrap-around. Negative indices wrap
from the end: `view.chars(wrapped: -1)` reads the last char.
Returns `None` on an empty view.

_Defined in `lang/std/text/views.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

_Defined in `lang/std/text/views.ks`._

#### field `endIndex`

```kestrel
public var endIndex: CharIndex { get }
```

Char index at the end (one past the last byte).

_Defined in `lang/std/text/views.ks`._

#### field `first`

```kestrel
public var first: Char? { get }
```

The first code point, or `None` if the view is empty.

_Defined in `lang/std/text/views.ks`._

#### function `firstIndex`

```kestrel
public func firstIndex(of: Char) -> CharIndex?
```

Returns the index of the first occurrence of `c`, or `.None`.

_Defined in `lang/std/text/views.ks`._

#### function `firstIndex`

```kestrel
public func firstIndex(where: (Char) -> Bool) -> CharIndex?
```

Returns the index of the first code point matching `predicate`, or `.None`.

_Defined in `lang/std/text/views.ks`._

#### function `index`

```kestrel
public func index(at: Int64) -> CharIndex?
```

Resolves the n-th code point to its byte offset. O(n).

_Defined in `lang/std/text/views.ks`._

#### function `indexedIter`

```kestrel
public func indexedIter() -> IndexedCharsIterator
```

Returns an iterator yielding `(CharIndex, Char)` pairs.

_Defined in `lang/std/text/views.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` when the view spans zero bytes (no code points).

O(1) — checks `byteCount`, not `count`.

_Defined in `lang/std/text/views.ks`._

#### field `last`

```kestrel
public var last: Char? { get }
```

The last code point, or `None` if the view is empty.

_Defined in `lang/std/text/views.ks`._

#### function `lastIndex`

```kestrel
public func lastIndex(of: Char) -> CharIndex?
```

Returns the index of the last occurrence of `c`, or `.None`.

_Defined in `lang/std/text/views.ks`._

#### function `lastIndex`

```kestrel
public func lastIndex(where: (Char) -> Bool) -> CharIndex?
```

Returns the index of the last code point matching `predicate`, or `.None`.

_Defined in `lang/std/text/views.ks`._

#### field `reversed`

```kestrel
public var reversed: ReversedCharsView { get }
```

A reversed view that iterates code points back-to-front.

_Defined in `lang/std/text/views.ks`._

#### function `slice`

```kestrel
public func slice(from: CharIndex, to: CharIndex) -> StringSlice
```

Returns a `StringSlice` covering `[start, end)` by byte offset.

_Defined in `lang/std/text/views.ks`._

#### field `startIndex`

```kestrel
public var startIndex: CharIndex { get }
```

Char index at byte 0 (the first code point).

_Defined in `lang/std/text/views.ks`._

#### function `substring`

```kestrel
public func substring[__opaque_0](__opaque_0) -> String where __opaque_0: CharsSubstringIndex
```

Convenience: dispatches to a `CharsSubstringIndex` to produce
an owned `String` covering the requested code-point range.
Equivalent to `self(range).toString()` for both `Range[Int64]`
and `ClosedRange[Int64]`.

_Defined in `lang/std/text/views.ks`._

#### function `toString`

```kestrel
public func toString() -> String
```

Materializes the view as an owned `String`. O(n) — copies bytes.

_Defined in `lang/std/text/views.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = Char
```

The element type yielded by iteration — always `Char`.

_Defined in `lang/std/text/views.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = CharsIterator
```

The iterator type returned by `iter()`.

_Defined in `lang/std/text/views.ks`._

#### function `iter`

```kestrel
public func iter() -> CharsIterator
```

Returns a `CharsIterator` positioned at byte 0.

Each call returns a fresh iterator; the view itself is reusable.

_Defined in `lang/std/text/views.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> CharsView
```

_Defined in `lang/std/text/views.ks`._

## struct `DefaultStringInterpolation`

```kestrel
public struct DefaultStringInterpolation { /* private fields */ }
```

The default `Interpolatable` accumulator used for `String` interpolation.

Stores each literal and each formatted interpolation as a separate
`String` part, then concatenates them in `build()`. The two-pass
design lets `build()` size the result buffer exactly, avoiding the
repeated reallocation cost a single-buffer accumulator would pay.

### Examples

```
var acc = DefaultStringInterpolation(literalCapacity: 7, interpolationCount: 1);
acc.appendLiteral("hello, ");
acc.appendInterpolation("world", options: FormatOptions.default());
acc.build();  // "hello, world"
```

### Representation

A single `StringBuilder` that accumulates all literal and formatted
bytes in one buffer. Pre-sized using the compiler's capacity hints.

_Defined in `lang/std/text/format.ks`._

### Members

#### initializer `With Capacity`

```kestrel
public init(literalCapacity: Int64, interpolationCount: Int64)
```

Constructs an empty accumulator pre-sized from compile-time hints.

`literalCapacity` is the exact byte count of static segments;
`interpolationCount` estimates ~16 bytes per hole.

##### Examples

```
var acc = DefaultStringInterpolation(literalCapacity: 0, interpolationCount: 0);
acc.build();  // ""
```

_Defined in `lang/std/text/format.ks`._

#### function `appendInterpolation`

```kestrel
public mutating func appendInterpolation[__opaque_0](__opaque_0, FormatOptions) where __opaque_0: Formattable
```

Formats one interpolation hole directly into the buffer.

_Defined in `lang/std/text/format.ks`._

#### function `build`

```kestrel
public mutating func build() -> String
```

Transfers the buffer into a `String` without copying.

##### Examples

```
var acc = DefaultStringInterpolation(literalCapacity: 0, interpolationCount: 0);
acc.appendLiteral("a");
acc.appendLiteral("b");
acc.build();  // "ab"
```

_Defined in `lang/std/text/format.ks`._

### Implements `Interpolatable`

#### initializer `With Capacity`

```kestrel
init(literalCapacity: Int64, interpolationCount: Int64)
```

Constructs an empty accumulator with capacity hints derived from the literal at compile time.

`literalCapacity` is the total byte count of the static segments;
`interpolationCount` is the number of `\{...}` holes. Implementors
can use these to preallocate.

_Defined in `lang/std/text/format.ks`._

#### function `appendLiteral`

```kestrel
public mutating func appendLiteral(String)
```

Appends a static literal segment directly into the buffer.

_Defined in `lang/std/text/format.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> DefaultStringInterpolation
```

Returns a copy with a cloned builder buffer.

_Defined in `lang/std/text/format.ks`._

## protocol `ExpressibleByStringInterpolation`

```kestrel
public protocol ExpressibleByStringInterpolation
```

Marker protocol for types constructible from a completed string interpolation.

Refines `ExpressibleByStringLiteral` so a single conformance covers
both pure-literal `"abc"` and interpolated `"a\{x}b"` forms. The
compiler picks `Interpolation` as the accumulator type, drives it via
`Interpolatable`, then hands it to `init(interpolation:)`.

_Defined in `lang/std/text/format.ks`._

### Members

#### initializer `From Interpolation`

```kestrel
init(Interpolation)
```

Constructs `Self` from a fully built interpolation accumulator.

_Defined in `lang/std/text/format.ks`._

#### typealias `Interpolation`

```kestrel
type Interpolation
```

The accumulator type used to build interpolated values of `Self`.

_Defined in `lang/std/text/format.ks`._

### Implements `ExpressibleByStringLiteral`

#### initializer `String Literal`

```kestrel
init(stringLiteral: lang.ptr[lang.i8], lang.i64)
```

Builds an instance from a string literal.

_Defined in `lang/std/core/literals.ks`._

## enum `FloatStyle`

```kestrel
public enum FloatStyle
```

How a floating-point value should be rendered.

Selected by the `:f` / `:e` / `:E` / `:%` type slots in the format
mini-language and read by the `Float32` / `Float64` formatters. Choice
of style is independent of `precision` — `Auto` honours precision as
"max significant digits", `Fixed` and `Scientific` treat it as
"decimal places". The non-`Auto` variants always emit a decimal point.

### Examples

```
var opts = FormatOptions();
opts.precision = .Some(2);
opts.floatStyle = .Fixed;
(3.14159).format(options: opts);       // "3.14"
opts.floatStyle = .Scientific;
(3.14159).format(options: opts);       // "3.14e0"
opts.floatStyle = .Percent;
(0.5).format(options: opts);           // "50.00%"
```

_Defined in `lang/std/text/format.ks`._

### Members

#### case `Auto`

```kestrel
case Auto
```

Shortest round-trippable representation; switches to scientific for very large or very small magnitudes.

_Defined in `lang/std/text/format.ks`._

#### case `Fixed`

```kestrel
case Fixed
```

Fixed-point — `precision` controls decimal places.

_Defined in `lang/std/text/format.ks`._

#### case `Percent`

```kestrel
case Percent
```

Multiplies by 100 and appends `%`.

_Defined in `lang/std/text/format.ks`._

#### case `Scientific`

```kestrel
case Scientific
```

Scientific notation with lowercase `e` exponent marker.

_Defined in `lang/std/text/format.ks`._

#### case `ScientificUpper`

```kestrel
case ScientificUpper
```

Scientific notation with uppercase `E` exponent marker.

_Defined in `lang/std/text/format.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: FloatStyle) -> Bool
```

Returns true if both cases are the same variant.

All cases are payload-less, so equality is purely structural.

##### Examples

```
FloatStyle.Fixed.isEqual(to: .Fixed);       // true
FloatStyle.Fixed.isEqual(to: .Scientific);  // false
```

_Defined in `lang/std/text/format.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(FloatStyle) -> Bool
```

Pattern-match form of equality — delegates to `isEqual`.

##### Examples

```
FloatStyle.Auto.matches(.Auto);  // true
```

_Defined in `lang/std/text/format.ks`._

## struct `FormatOptions`

```kestrel
public struct FormatOptions { /* private fields */ }
```

Mutable bag of formatting knobs threaded through every `Formattable.format` call.

`FormatOptions` is the parsed form of the format-spec mini-language.
String interpolation `"\{expr:spec}"` constructs one of these from the
trailing spec, then hands it to the formatter for `expr`'s type. Each
formatter reads only the fields that apply to it: integers ignore
`floatStyle`, strings ignore `radix`, and so on.

### Format spec mini-language

`[[fill]align][sign][#][0][width][.precision][type]`

Where `type` is one of:
  - Integers: `b` (binary), `o` (octal), `x` (hex lower), `X` (hex upper)
  - Floats:   `f` (fixed), `e` (scientific), `E` (scientific upper), `%` (percent)
  - Any:      `?` (debug)

### Examples

```
"\{n:>8}";      // right-align, width 8
"\{n:08x}";     // zero-pad, width 8, hex
"\{n:#X}";      // hex upper with 0x prefix
"\{pi:.2}";     // precision 2 decimal places
"\{pi:.2e}";    // scientific with 2 decimal places
"\{ratio:%}";   // as percentage (0.5 -> "50%")
"\{name:^10}";  // center, width 10
"\{value:?}";   // debug format
```

### Representation

A flat record of independent fields — no validation across them. Each
formatter is responsible for ignoring fields outside its domain and
applying its own defaults when an option is absent.

_Defined in `lang/std/text/format.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Creates a `FormatOptions` with every field at its default value.

Defaults: no `width` or `precision`, left alignment, space fill,
decimal radix, lowercase hex, negative-only sign, no alternate form,
`Auto` float style, debug off.

##### Examples

```
var opts = FormatOptions();
opts.width = .Some(6);
opts.alignment = .Right;
"hi".format(options: opts);  // "    hi"
```

_Defined in `lang/std/text/format.ks`._

#### field `alignment`

```kestrel
public var alignment: Alignment
```

How to position the value inside `width` when padding is required.

_Defined in `lang/std/text/format.ks`._

#### field `alternate`

```kestrel
public var alternate: Bool
```

Alternate form: emit the conventional radix prefix (`0b`, `0o`, `0x`/`0X`) for non-decimal integers.

_Defined in `lang/std/text/format.ks`._

#### field `debug`

```kestrel
public var debug: Bool
```

When `true`, formatters should produce a structural / debug representation rather than a user-facing one.

_Defined in `lang/std/text/format.ks`._

#### function `default`

```kestrel
public static func default() -> FormatOptions
```

Returns a fresh `FormatOptions` with all fields at their default values.

Equivalent to calling `FormatOptions()`; provided as a static so
callers that want defaults without spelling out the constructor
(e.g. default-arg expressions) have a clean call site.

##### Examples

```
let opts = FormatOptions.default();
(42).format(options: opts);  // "42"
```

_Defined in `lang/std/text/format.ks`._

#### field `fill`

```kestrel
public var fill: Char
```

Padding character — defaults to `' '`. Only applies when `width` is set and the value is shorter.

_Defined in `lang/std/text/format.ks`._

#### field `floatStyle`

```kestrel
public var floatStyle: FloatStyle
```

Float rendering style (fixed, scientific, percent, auto).

_Defined in `lang/std/text/format.ks`._

#### field `precision`

```kestrel
public var precision: Int64?
```

For floats: number of decimal places (or significant digits in `Auto` mode). For strings: maximum character count.

_Defined in `lang/std/text/format.ks`._

#### field `radix`

```kestrel
public var radix: Int64
```

Numeric base for integer formatting: 2 (binary), 8 (octal), 10 (decimal), 16 (hex).

_Defined in `lang/std/text/format.ks`._

#### field `sign`

```kestrel
public var sign: Sign
```

Sign-display strategy for numeric formatters.

_Defined in `lang/std/text/format.ks`._

#### field `uppercase`

```kestrel
public var uppercase: Bool
```

When `true`, integer hex digits are emitted as `A`–`F` rather than `a`–`f`.

_Defined in `lang/std/text/format.ks`._

#### field `width`

```kestrel
public var width: Int64?
```

Minimum field width in characters; shorter values are padded with `fill` according to `alignment`.

_Defined in `lang/std/text/format.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: FormatOptions) -> Bool
```

Returns true if all fields are equal between the two options.

`width` and `precision` are not compared — they typically reflect
per-call overrides rather than logical identity. Compare them
explicitly if needed.

##### Examples

```
let a = FormatOptions();
let b = FormatOptions();
a.isEqual(to: b);  // true
var c = FormatOptions();
c.alternate = true;
a.isEqual(to: c);  // false
```

_Defined in `lang/std/text/format.ks`._

## protocol `Formattable`

```kestrel
public protocol Formattable
```

Protocol for types that can render themselves as a `String` under a `FormatOptions`.

Print routines and string interpolation `"\{expr}"` and `"\{expr:spec}"`
both ultimately bottom out in `format`. Implementors should honour
every `FormatOptions` field that is meaningful for their domain
(alignment and width are universal; `radix` only applies to integers,
`floatStyle` only to floats) and silently ignore fields that aren't.

### Examples

```
"\{name}";         // "Alice"          (default formatting)
"\{name:>10}";     // "     Alice"     (right-align, width 10)
"\{n:08x}";        // "0000002a"       (zero-pad, hex, width 8)
"\{pi:.2}";        // "3.14"           (precision 2)
"\{value:?}";      // debug representation
```

_Defined in `lang/std/text/format.ks`._

### Members

#### function `format`

```kestrel
func format(into: mutating StringBuilder, FormatOptions)
```

Writes this value's formatted representation directly into `writer`.

This is the kernel method — all formatting ultimately bottoms out
here. The convenience `format(options:) -> String` in the protocol
extension calls this under the hood.

_Defined in `lang/std/text/format.ks`._

#### function `formatted`

```kestrel
public func formatted(FormatOptions) -> String
```

Returns this value rendered as a `String`.

Convenience wrapper: creates a `StringBuilder`, calls
`format(into:)`, and returns the built string. Uses a distinct
name to avoid overload-resolution ambiguity with `format(into:)`.

_Defined in `lang/std/text/format.ks`._

## struct `Grapheme`

```kestrel
public struct Grapheme { /* private fields */ }
```

An extended grapheme cluster — what users perceive as a single character.

A grapheme may comprise one `Char` (e.g. `'a'`) or several
(combining marks, regional-indicator country flags, ZWJ-joined emoji
sequences). `String.graphemes` is the canonical producer; iteration
uses UAX #29 segmentation. Treat `Grapheme` as the right unit for
any user-visible operation (cursor movement, selection, truncation
for display).

### Examples

```
let g = Grapheme(char: 'a');
g.charCount();   // 1
g.isAscii;     // true
g.utf8Length();  // 1
```

### Representation

An `Array[Char]` of the constituent code points in scalar order.

_Defined in `lang/std/text/char.ks`._

### Members

#### initializer `From Chars`

```kestrel
public init(chars: Array[Char])
```

Constructs a grapheme from a sequence of `Char`s.

The caller is responsible for the chars actually forming a
single UAX #29 cluster — the constructor does not segment or
validate. `GraphemesIterator` is the canonical producer of valid
clusters. Single-char input avoids allocating; multi-char input
keeps the trailing code points in a separate array.

##### Examples

```
var chars = Array[Char]();
chars.append('e');
chars.append('\u{0301}');  // combining acute
let g = Grapheme(chars: chars);
g.charCount();  // 2
```

_Defined in `lang/std/text/char.ks`._

#### initializer `Single Char`

```kestrel
public init(char: Char)
```

Constructs a one-`Char` grapheme.

Allocation-free — the common path for ASCII iteration through
`GraphemesView`.

##### Examples

```
let g = Grapheme(char: 'a');
g.charCount();  // 1
```

_Defined in `lang/std/text/char.ks`._

#### function `charCount`

```kestrel
public func charCount() -> Int64
```

Returns the number of `Char`s in this cluster — `1` for plain ASCII, more for combining sequences and ZWJ-joined emoji.

_Defined in `lang/std/text/char.ks`._

#### field `chars`

```kestrel
public var chars: Array[Char] { get }
```

The constituent code points in scalar order.

Materializes a fresh `Array[Char]` on every access.

_Defined in `lang/std/text/char.ks`._

#### field `firstChar`

```kestrel
public var firstChar: Char { get }
```

Returns the first `Char` of the cluster.

The first code point of this grapheme cluster.

_Defined in `lang/std/text/char.ks`._

#### field `isAscii`

```kestrel
public var isAscii: Bool { get }
```

Returns true iff the cluster is exactly one ASCII `Char`.

A single-`Char` non-ASCII grapheme (e.g. `é` as the precomposed
`U+00E9`) returns `false`. Multi-`Char` clusters always return
`false` even if every component is ASCII.

_Defined in `lang/std/text/char.ks`._

#### function `utf8Length`

```kestrel
public func utf8Length() -> Int64
```

Returns the total UTF-8 byte length of all constituent `Char`s.

Sum of each `Char.utf8Length()`. Use this to size a buffer
before re-encoding the cluster.

_Defined in `lang/std/text/char.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Grapheme) -> Bool
```

Returns true if the two graphemes are the same length and every `Char` is equal pairwise.

**Not** Unicode normalization-aware: precomposed `é` (`U+00E9`)
and decomposed `e` + `U+0301` are not equal under this check
even though they represent the same user-perceived character.
Normalize both sides first if you need that.

##### Examples

```
let a = Grapheme(char: 'a');
let b = Grapheme(char: 'a');
a.isEqual(to: b);  // true
```

_Defined in `lang/std/text/char.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> Grapheme
```

Returns a deep copy of this grapheme.

_Defined in `lang/std/text/char.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(Grapheme) -> Ordering
```

_Defined in `lang/std/text/char.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

_Defined in `lang/std/text/char.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating StringBuilder, FormatOptions)
```

_Defined in `lang/std/text/char.ks`._

## struct `GraphemeIndex`

```kestrel
public struct GraphemeIndex { /* private fields */ }
```

A typed wrapper for a grapheme-cluster position within a `String`.

Like `CharIndex` but ranges over UAX #29 clusters rather than
code points. Stores the byte offset of the cluster's first byte;
resolving requires walking the segmenter.

### Representation

A single `Int64` field holding the byte offset of the grapheme.

_Defined in `lang/std/text/views.ks`._

### Members

#### initializer `From Offset`

```kestrel
public init(Int64)
```

Wraps a pre-resolved byte offset for a grapheme position.

_Defined in `lang/std/text/views.ks`._

#### function `advance`

```kestrel
public func advance(by: Int64, from: StringSlice) -> GraphemeIndex
```

Advances by `n` grapheme clusters. Requires the source slice to
run the UAX #29 segmenter forward. O(n) in graphemes advanced.

##### Examples

```
let s = "héllo";
let idx = s.graphemes.startIndex;    // byte 0
let next = idx.advance(by: 2, from: s.asSlice());
// Skipped 'h' (1 byte) and 'é' (2 bytes) → byte 3
```

_Defined in `lang/std/text/views.ks`._

#### field `byteOffset`

```kestrel
public var byteOffset: Int64
```

The byte offset where the indexed grapheme begins.

_Defined in `lang/std/text/views.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: GraphemeIndex) -> Bool
```

Returns true if the two indices point at the same byte offset.

_Defined in `lang/std/text/views.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(GraphemeIndex) -> Ordering
```

_Defined in `lang/std/text/slice.ks`._

## struct `GraphemesIterator`

```kestrel
public struct GraphemesIterator { /* private fields */ }
```

Iterator over extended grapheme clusters under UAX #29 segmentation.

Wraps a `CharsIterator` and consults the Unicode grapheme-break
property tables on each step. Buffers one look-ahead `Char` so it
can decide whether the next code point starts a new cluster; that
pending char is yielded as the start of the *next* cluster on the
following call. Handles ZWJ-joined sequences and regional-indicator
flag pairs.

### Examples

```
var it = "a\u{0301}b".graphemes.iter();
it.next();  // Some(Grapheme: ['a', U+0301])
it.next();  // Some(Grapheme: ['b'])
it.next();  // None
```

### Representation

Wraps a `CharsIterator` plus a small amount of state machine: the
pending look-ahead char, the previous break property, the
"previous-previous was Regional Indicator" flag (for flag pairs),
and a `started` marker.

_Defined in `lang/std/text/views.ks`._

### Members

#### initializer `From Chars`

```kestrel
public init(CharsIterator)
```

Wraps a `CharsIterator` to produce graphemes via UAX #29 segmentation.

Prefer `someString.graphemes.iter()` over calling this directly.

_Defined in `lang/std/text/views.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = Grapheme
```

The element type yielded by `next()` — always `Grapheme`.

_Defined in `lang/std/text/views.ks`._

#### function `next`

```kestrel
public mutating func next() -> Grapheme?
```

Returns the next grapheme cluster, or `None` when the source is exhausted.

Accumulates code points until `shouldBreakBetween` reports a
boundary, then returns them as a `Grapheme`. The look-ahead
char that triggered the break is held back for the next call.

_Defined in `lang/std/text/views.ks`._

## protocol `GraphemesSubstringIndex`

```kestrel
public protocol GraphemesSubstringIndex
```

Range-only index for `GraphemesView.substring`. See `BytesSubstringIndex`.

_Defined in `lang/std/text/views.ks`._

### Members

#### function `readGraphemesSubstring`

```kestrel
func readGraphemesSubstring(from: GraphemesView) -> String
```

_Defined in `lang/std/text/views.ks`._

## struct `GraphemesView`

```kestrel
public struct GraphemesView { /* private fields */ }
```

A view over the user-perceived characters (extended grapheme clusters) of a `String`.

Returned by `String.graphemes`. Use this — not `chars` — when you
need the unit a user thinks of as a single character: emoji
sequences, accented forms, country flags, etc. Both iteration and
`count()` are O(n) because each cluster requires consulting the
UAX #29 break tables.

### Examples

```
let flag = "\u{1F1FA}\u{1F1F8}";  // 🇺🇸
flag.chars.count;        // 2 (regional indicators)
flag.graphemes.count;    // 1 (one flag)
```

### Representation

A `(ptr, length)` pair; iteration is delegated to a wrapped
`CharsIterator` plus the UAX #29 segmenter state machine.

_Defined in `lang/std/text/views.ks`._

### Members

#### subscript `Checked Index`

```kestrel
public subscript[I](checked: I) -> I.GraphemesYield? { get }
```

Reads at `index`, returning `.None` on out-of-bounds.

_Defined in `lang/std/text/views.ks`._

#### subscript `Clamping`

```kestrel
public subscript[I](clamped: I) -> I.GraphemesClampedYield { get }
```

Reads at `index` saturated to `[0, count)`. Single-grapheme
indexes yield `Grapheme?` (`.None` only when the view is empty);
range indexes yield `GraphemesView` (always valid, possibly empty).

_Defined in `lang/std/text/views.ks`._

#### initializer `From Slice`

```kestrel
public init(slice: StringSlice)
```

Constructs a graphemes view backed by the given string slice.
The view retains shared ownership of the underlying bytes.

_Defined in `lang/std/text/views.ks`._

#### subscript `Indexed Grapheme / Sub-view`

```kestrel
public subscript[I](I) -> I.GraphemesYield { get }
```

`Int64` reads a single cluster; `Range[Int64]` /
`ClosedRange[Int64]` yield a zero-copy `GraphemesView` sub-view
covering those clusters. **O(n)** — walks the segmenter from the
start. Panics on out-of-bounds.

_Defined in `lang/std/text/views.ks`._

#### subscript `Wrapping`

```kestrel
public subscript[I](wrapped: I) -> I.GraphemesWrappedYield { get }
```

Reads at `index` with modulo wrap-around. Negative indices wrap
from the end: `view.graphemes(wrapped: -1)` reads the last
grapheme cluster. Returns `None` on an empty view.

_Defined in `lang/std/text/views.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Number of grapheme clusters. **O(n)** — walks the entire string
through the UAX #29 segmenter. Cache the result if you need it
more than once; each access re-walks the string.

_Defined in `lang/std/text/views.ks`._

#### field `endIndex`

```kestrel
public var endIndex: GraphemeIndex { get }
```

Grapheme index at the end (one past the last byte).

_Defined in `lang/std/text/views.ks`._

#### field `first`

```kestrel
public var first: Grapheme? { get }
```

The first grapheme cluster, or `None` if the view is empty.

O(1) in practice — decodes one cluster from the start.

_Defined in `lang/std/text/views.ks`._

#### function `firstIndex`

```kestrel
public func firstIndex(where: (Grapheme) -> Bool) -> GraphemeIndex?
```

Returns the index of the first grapheme matching `predicate`, or `.None`.

_Defined in `lang/std/text/views.ks`._

#### function `index`

```kestrel
public func index(at: Int64) -> GraphemeIndex?
```

Resolves the n-th grapheme cluster to its byte offset. O(n) —
walks the segmenter from the start.

_Defined in `lang/std/text/views.ks`._

#### function `indexedIter`

```kestrel
public func indexedIter() -> IndexedGraphemesIterator
```

Returns an iterator yielding `(GraphemeIndex, Grapheme)` pairs.

_Defined in `lang/std/text/views.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` when the view spans zero bytes (no graphemes).

O(1) — checks `byteCount`, not `count`.

_Defined in `lang/std/text/views.ks`._

#### field `last`

```kestrel
public var last: Grapheme? { get }
```

The last grapheme cluster, or `None` if the view is empty.

O(n) — walks the entire string through the segmenter.

_Defined in `lang/std/text/views.ks`._

#### function `slice`

```kestrel
public func slice(from: GraphemeIndex, to: GraphemeIndex) -> StringSlice
```

Returns a `StringSlice` covering `[start, end)` by byte offset.

_Defined in `lang/std/text/views.ks`._

#### field `startIndex`

```kestrel
public var startIndex: GraphemeIndex { get }
```

Grapheme index at byte 0.

_Defined in `lang/std/text/views.ks`._

#### function `substring`

```kestrel
public func substring[__opaque_0](__opaque_0) -> String where __opaque_0: GraphemesSubstringIndex
```

Convenience: dispatches to a `GraphemesSubstringIndex` to
produce an owned `String` covering the requested cluster range.
Equivalent to `self(range).toString()` for both `Range[Int64]`
and `ClosedRange[Int64]`.

_Defined in `lang/std/text/views.ks`._

#### function `toString`

```kestrel
public func toString() -> String
```

Materializes the view as an owned `String`. O(n) — copies bytes.

_Defined in `lang/std/text/views.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = Grapheme
```

The element type yielded by iteration — always `Grapheme`.

_Defined in `lang/std/text/views.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = GraphemesIterator
```

The iterator type returned by `iter()`.

_Defined in `lang/std/text/views.ks`._

#### function `iter`

```kestrel
public func iter() -> GraphemesIterator
```

Returns a `GraphemesIterator` positioned at byte 0.

_Defined in `lang/std/text/views.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> GraphemesView
```

_Defined in `lang/std/text/views.ks`._

## struct `IndexedCharsIterator`

```kestrel
public struct IndexedCharsIterator { /* private fields */ }
```

Iterator yielding `(CharIndex, Char)` pairs — the byte offset of each
code point alongside the decoded character. Useful when you need to
know where each char starts in the buffer.

_Defined in `lang/std/text/views.ks`._

### Members

#### initializer `init`

```kestrel
public init(ptr: lang.ptr[lang.i8], length: Int64)
```

_Defined in `lang/std/text/views.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = (CharIndex, Char)
```

_Defined in `lang/std/text/views.ks`._

#### function `next`

```kestrel
public mutating func next() -> (CharIndex, Char)?
```

_Defined in `lang/std/text/views.ks`._

## struct `IndexedGraphemesIterator`

```kestrel
public struct IndexedGraphemesIterator { /* private fields */ }
```

Iterator yielding `(GraphemeIndex, Grapheme)` pairs — the byte offset
of each grapheme cluster alongside the grapheme value.

_Defined in `lang/std/text/views.ks`._

### Members

#### initializer `init`

```kestrel
public init(inner: GraphemesIterator)
```

_Defined in `lang/std/text/views.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = (GraphemeIndex, Grapheme)
```

_Defined in `lang/std/text/views.ks`._

#### function `next`

```kestrel
public mutating func next() -> (GraphemeIndex, Grapheme)?
```

_Defined in `lang/std/text/views.ks`._

## struct `IndexedLinesIterator`

```kestrel
public struct IndexedLinesIterator { /* private fields */ }
```

Iterator yielding `(LineIndex, String)` pairs — the byte offset of each
line's start alongside the line content (without terminator).

_Defined in `lang/std/text/views.ks`._

### Members

#### initializer `init`

```kestrel
public init(ptr: lang.ptr[lang.i8], length: Int64)
```

_Defined in `lang/std/text/views.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = (LineIndex, String)
```

_Defined in `lang/std/text/views.ks`._

#### function `next`

```kestrel
public mutating func next() -> (LineIndex, String)?
```

_Defined in `lang/std/text/views.ks`._

## protocol `Interpolatable`

```kestrel
public protocol Interpolatable
```

Protocol for the accumulator type that string interpolation builds into.

The compiler lowers `"hello, \{name}!"` to a sequence of
`appendLiteral` and `appendInterpolation` calls on a fresh value of
the implementor's type, then reads the final string out (typically
via a `build()` method on the concrete accumulator). `String` ships
`DefaultStringInterpolation` as its accumulator; custom string-like
types can supply their own to intercept literal pieces or coerce
formatted parts.

_Defined in `lang/std/text/format.ks`._

### Members

#### initializer `With Capacity`

```kestrel
init(literalCapacity: Int64, interpolationCount: Int64)
```

Constructs an empty accumulator with capacity hints derived from the literal at compile time.

`literalCapacity` is the total byte count of the static segments;
`interpolationCount` is the number of `\{...}` holes. Implementors
can use these to preallocate.

_Defined in `lang/std/text/format.ks`._

#### function `appendLiteral`

```kestrel
mutating func appendLiteral(String)
```

Appends a static literal segment.

Called once per run of literal text between `\{...}` holes. May be
called with the empty string; implementors should be cheap in
that case.

_Defined in `lang/std/text/format.ks`._

## struct `LineIndex`

```kestrel
public struct LineIndex { /* private fields */ }
```

A typed wrapper for a line position within a string.
Stores the byte offset of the line's first byte.

_Defined in `lang/std/text/slice.ks`._

### Members

#### function `advance`

```kestrel
public func advance(by: Int64, from: StringSlice) -> LineIndex
```

Advances by `n` lines. Scans for line terminators (`\n`, `\r\n`,
`\r`) from the current byte offset. O(n) in lines advanced.

##### Examples

```
let s = "a\nb\nc";
let idx = s.lines.startIndex;       // byte 0
let second = idx.advance(by: 1, from: s.asSlice());
// second.byteOffset == 2 (past "a\n")
```

_Defined in `lang/std/text/views.ks`._

#### field `byteOffset`

```kestrel
public var byteOffset: Int64
```

_Defined in `lang/std/text/slice.ks`._

#### initializer `init`

```kestrel
public init(Int64)
```

_Defined in `lang/std/text/slice.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: LineIndex) -> Bool
```

_Defined in `lang/std/text/slice.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(LineIndex) -> Ordering
```

_Defined in `lang/std/text/slice.ks`._

## struct `LinesIterator`

```kestrel
public struct LinesIterator { /* private fields */ }
```

Iterator that yields each line of a string as a `String`.

Recognises both `\n` (LF) and `\r\n` (CRLF) as line terminators
and a lone `\r` (CR) as a terminator on its own. The terminator
itself is **not** included in the yielded line. A trailing line
without a terminator is still emitted; an empty input emits no
lines.

### Examples

```
var it = "a\nb\r\nc".lines.iter();
it.next();  // Some("a")
it.next();  // Some("b")
it.next();  // Some("c")
it.next();  // None
```

### Representation

A `(ptr, length, byteIndex, done)` quadruple. `done` flips to true
after the trailing-no-terminator case has been emitted, so further
calls return `None`.

_Defined in `lang/std/text/views.ks`._

### Members

#### initializer `From Pointer`

```kestrel
public init(ptr: lang.ptr[lang.i8], length: Int64, byteIndex: Int64, done: Bool)
```

Constructs a lines iterator from a raw pointer, total byte count, starting byte offset, and `done` flag.

Prefer `someString.lines.iter()` over calling this directly.

##### Safety

`ptr` must point to `length` valid UTF-8 bytes; `byteIndex`
must be `0` or sit at a UTF-8 boundary.

_Defined in `lang/std/text/views.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = String
```

The element type yielded by `next()` — always `String`.

_Defined in `lang/std/text/views.ks`._

#### function `next`

```kestrel
public mutating func next() -> String?
```

Returns the next line, or `None` once exhausted.

Scans byte-by-byte for `\n` or `\r`, treating `\r\n` as a
single terminator. The yielded string contains the bytes up
to but not including the terminator.

_Defined in `lang/std/text/views.ks`._

## protocol `LinesSubstringIndex`

```kestrel
public protocol LinesSubstringIndex
```

Range-only index for `LinesView.substring`. See `BytesSubstringIndex`.

_Defined in `lang/std/text/views.ks`._

### Members

#### function `readLinesSubstring`

```kestrel
func readLinesSubstring(from: LinesView) -> String
```

_Defined in `lang/std/text/views.ks`._

## struct `LinesView`

```kestrel
public struct LinesView { /* private fields */ }
```

A view over the lines of a `String`, split on `\n`, `\r\n`, or `\r`.

Returned by `String.lines`. The yielded strings (from iteration or
single-line subscripting) do not include the terminator; a trailing
line without a terminator is still emitted. Range subscripts
(`lines(0..<n)`) yield a zero-copy `LinesView` sub-view whose
underlying byte range still includes the original terminators —
iterating the sub-view round-trips the same line strings, and
`.toString()` reconstructs the original substring exactly.

### Examples

```
var lines = Array[String]();
for line in "a\nb\nc".lines {
    lines.append(line);
}
lines.count;  // 3

// Range subscript preserves terminators in the underlying bytes:
"a\r\nb\nc".lines(0..<2).toString();  // "a\r\nb\n"
```

### Representation

A `(ptr, length)` pair pointing into the source string.

_Defined in `lang/std/text/views.ks`._

### Members

#### subscript `Checked Index`

```kestrel
public subscript[I](checked: I) -> I.LinesYield? { get }
```

Reads at `index`, returning `.None` on out-of-bounds.

_Defined in `lang/std/text/views.ks`._

#### subscript `Clamping`

```kestrel
public subscript[I](clamped: I) -> I.LinesClampedYield { get }
```

Reads at `index` saturated to `[0, count)`. Single-line indexes
yield `String?` (`.None` only when the view holds no lines);
range indexes yield `LinesView` (always valid, possibly empty).

_Defined in `lang/std/text/views.ks`._

#### initializer `From Slice`

```kestrel
public init(slice: StringSlice)
```

Constructs a lines view backed by the given string slice.
The view retains shared ownership of the underlying bytes.

_Defined in `lang/std/text/views.ks`._

#### subscript `Indexed Line / Sub-view`

```kestrel
public subscript[I](I) -> I.LinesYield { get }
```

`Int64` reads a single line as a `String` (without terminator).
`Range[Int64]` / `ClosedRange[Int64]` yield a zero-copy
`LinesView` sub-view covering those lines (terminators preserved
in the underlying bytes). **O(n)** — walks the buffer from the
start. Panics on out-of-bounds.

_Defined in `lang/std/text/views.ks`._

#### subscript `Wrapping`

```kestrel
public subscript[I](wrapped: I) -> I.LinesWrappedYield { get }
```

Reads at `index` with modulo wrap-around. Negative indices wrap
from the end: `view.lines(wrapped: -1)` reads the last line.
Returns `None` on an empty view.

_Defined in `lang/std/text/views.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Number of lines in the view. **O(n)** — walks the buffer
scanning for terminators. Cache the result if you need it more
than once.

_Defined in `lang/std/text/views.ks`._

#### field `endIndex`

```kestrel
public var endIndex: LineIndex { get }
```

Line index at the end (one past the last byte).

_Defined in `lang/std/text/views.ks`._

#### field `first`

```kestrel
public var first: String? { get }
```

The first line (without terminator), or `None` if the view is empty.

O(first line length) — scans for the first terminator.

_Defined in `lang/std/text/views.ks`._

#### function `index`

```kestrel
public func index(at: Int64) -> LineIndex?
```

Resolves the n-th line to its byte offset. O(n) — scans for
line terminators from the start.

_Defined in `lang/std/text/views.ks`._

#### function `indexedIter`

```kestrel
public func indexedIter() -> IndexedLinesIterator
```

Returns an iterator yielding `(LineIndex, String)` pairs.

_Defined in `lang/std/text/views.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` when the view spans zero bytes (no lines).

O(1) — checks `byteCount`, not `count`.

_Defined in `lang/std/text/views.ks`._

#### function `slice`

```kestrel
public func slice(from: LineIndex, to: LineIndex) -> StringSlice
```

Returns a `StringSlice` covering `[start, end)` by byte offset.

_Defined in `lang/std/text/views.ks`._

#### field `startIndex`

```kestrel
public var startIndex: LineIndex { get }
```

Line index at byte 0.

_Defined in `lang/std/text/views.ks`._

#### function `substring`

```kestrel
public func substring[__opaque_0](__opaque_0) -> String where __opaque_0: LinesSubstringIndex
```

Convenience: dispatches to a `LinesSubstringIndex` to produce
an owned `String` covering the requested line range, with their
original terminators preserved. Equivalent to
`self(range).toString()` for both `Range[Int64]` and
`ClosedRange[Int64]`.

_Defined in `lang/std/text/views.ks`._

#### function `toString`

```kestrel
public func toString() -> String
```

Materializes the view as an owned `String` covering the entire
underlying buffer (terminators included). O(n) — copies bytes.

_Defined in `lang/std/text/views.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = String
```

The element type yielded by iteration — always `String`.

_Defined in `lang/std/text/views.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = LinesIterator
```

The iterator type returned by `iter()`.

_Defined in `lang/std/text/views.ks`._

#### function `iter`

```kestrel
public func iter() -> LinesIterator
```

Returns a `LinesIterator` positioned at byte 0.

_Defined in `lang/std/text/views.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> LinesView
```

_Defined in `lang/std/text/views.ks`._

## struct `ReversedCharsIterator`

```kestrel
public struct ReversedCharsIterator { /* private fields */ }
```

Iterator that yields code points back-to-front by walking backward
through UTF-8 continuation bytes to find each leading byte, then
decoding forward.

_Defined in `lang/std/text/views.ks`._

### Members

#### initializer `init`

```kestrel
public init(ptr: lang.ptr[lang.i8], length: Int64)
```

_Defined in `lang/std/text/views.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = Char
```

_Defined in `lang/std/text/views.ks`._

#### function `next`

```kestrel
public mutating func next() -> Char?
```

_Defined in `lang/std/text/views.ks`._

## struct `ReversedCharsView`

```kestrel
public struct ReversedCharsView { /* private fields */ }
```

A reversed view over the code points in a string. Iterates characters
back-to-front without allocating.

### Examples

```
let view = "abc".chars.reversed;
view.first();    // Some('c')
view.count;      // 3
```

_Defined in `lang/std/text/views.ks`._

### Members

#### field `count`

```kestrel
public var count: Int64 { get }
```

Number of code points. O(n).

_Defined in `lang/std/text/views.ks`._

#### field `first`

```kestrel
public var first: Char? { get }
```

The first element of the reversed view (= last char of the source).

_Defined in `lang/std/text/views.ks`._

#### initializer `init`

```kestrel
public init(slice: StringSlice)
```

_Defined in `lang/std/text/views.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

_Defined in `lang/std/text/views.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = Char
```

_Defined in `lang/std/text/views.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = ReversedCharsIterator
```

_Defined in `lang/std/text/views.ks`._

#### function `iter`

```kestrel
public func iter() -> ReversedCharsIterator
```

_Defined in `lang/std/text/views.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> ReversedCharsView
```

_Defined in `lang/std/text/views.ks`._

## enum `Sign`

```kestrel
public enum Sign
```

How the sign of a numeric value should be rendered.

Read by integer and float formatters before emitting the magnitude.
`Negative` is the conventional default — only `-` for negative values,
nothing for non-negatives. `Always` is useful for diffs or coordinates
where every value should carry an explicit sign; `Space` reserves a
column so columns of mixed signs line up.

### Examples

```
var opts = FormatOptions();
opts.sign = .Always;
(3).format(options: opts);   // "+3"
(-3).format(options: opts);  // "-3"
opts.sign = .Space;
(3).format(options: opts);   // " 3"
```

_Defined in `lang/std/text/format.ks`._

### Members

#### case `Always`

```kestrel
case Always
```

Always show a sign — `+` for non-negative, `-` for negative.

_Defined in `lang/std/text/format.ks`._

#### case `Negative`

```kestrel
case Negative
```

Show `-` for negative values, no prefix for zero or positive (default).

_Defined in `lang/std/text/format.ks`._

#### case `Space`

```kestrel
case Space
```

Use a leading space for non-negative, `-` for negative; keeps mixed-sign columns aligned.

_Defined in `lang/std/text/format.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Sign) -> Bool
```

Returns true if both cases are the same variant.

Used by `Equatable` to lift case identity into a `Bool` for
composite comparisons (see `FormatOptions.isEqual`).

##### Examples

```
Sign.Always.isEqual(to: .Always);     // true
Sign.Negative.isEqual(to: .Always);   // false
```

_Defined in `lang/std/text/format.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(Sign) -> Bool
```

Pattern-match form of equality — delegates to `isEqual`.

##### Examples

```
Sign.Space.matches(.Space);  // true
```

_Defined in `lang/std/text/format.ks`._

## struct `SplitView`

```kestrel
public struct SplitView { /* private fields */ }
```

Lazy view over the segments of a string split on a fixed separator.

Each segment is a zero-copy `StringSlice` into the original buffer.
Use `iter()` for one-pass iteration, or `first()`/`last()`/`collect()`
for targeted access.

### Examples

```
let view = "a,b,c".asSlice().split(",");
view.first();            // Some("a")
view.count;              // 3
view.collect();          // [StringSlice("a"), StringSlice("b"), StringSlice("c")]
```

_Defined in `lang/std/text/views.ks`._

### Members

#### function `collect`

```kestrel
public func collect() -> Array[StringSlice]
```

Collects all segments into an array.

_Defined in `lang/std/text/views.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Number of segments. O(n) — iterates once to count.

_Defined in `lang/std/text/views.ks`._

#### field `first`

```kestrel
public var first: StringSlice? { get }
```

The first segment, or `.None` if empty.

_Defined in `lang/std/text/views.ks`._

#### initializer `init`

```kestrel
public init(slice: StringSlice, separator: String)
```

_Defined in `lang/std/text/views.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

True when the source slice is empty.

_Defined in `lang/std/text/views.ks`._

#### field `last`

```kestrel
public var last: StringSlice? { get }
```

The last segment, or `.None` if empty.

_Defined in `lang/std/text/views.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = StringSlice
```

_Defined in `lang/std/text/views.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = SplitViewIterator
```

_Defined in `lang/std/text/views.ks`._

#### function `iter`

```kestrel
public func iter() -> SplitViewIterator
```

_Defined in `lang/std/text/views.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> SplitView
```

_Defined in `lang/std/text/views.ks`._

## struct `SplitViewIterator`

```kestrel
public struct SplitViewIterator { /* private fields */ }
```

Iterator that yields `StringSlice` segments produced by splitting on a
fixed separator. Zero-copy: each yielded slice is a window into the
original source buffer.

_Defined in `lang/std/text/views.ks`._

### Members

#### initializer `init`

```kestrel
public init(slice: StringSlice, separator: String)
```

_Defined in `lang/std/text/views.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = StringSlice
```

_Defined in `lang/std/text/views.ks`._

#### function `next`

```kestrel
public mutating func next() -> StringSlice?
```

_Defined in `lang/std/text/views.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> SplitViewIterator
```

_Defined in `lang/std/text/views.ks`._

## struct `SplitWhereView`

```kestrel
public struct SplitWhereView { /* private fields */ }
```

Lazy view over the segments of a string split at every code point
matching a predicate. The matching characters are excluded from segments.

### Examples

```
let view = "hello world".asSlice().split { (c) in c == Char(" ") };
view.first();    // Some("hello")
view.count;      // 2
```

_Defined in `lang/std/text/views.ks`._

### Members

#### function `collect`

```kestrel
public func collect() -> Array[StringSlice]
```

Collects all segments into an array.

_Defined in `lang/std/text/views.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Number of segments. O(n) — iterates once to count.

_Defined in `lang/std/text/views.ks`._

#### field `first`

```kestrel
public var first: StringSlice? { get }
```

The first segment, or `.None` if empty.

_Defined in `lang/std/text/views.ks`._

#### initializer `init`

```kestrel
public init(slice: StringSlice, where: (Char) -> Bool)
```

_Defined in `lang/std/text/views.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

True when the source slice is empty.

_Defined in `lang/std/text/views.ks`._

#### field `last`

```kestrel
public var last: StringSlice? { get }
```

The last segment, or `.None` if empty.

_Defined in `lang/std/text/views.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = StringSlice
```

_Defined in `lang/std/text/views.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = SplitWhereViewIterator
```

_Defined in `lang/std/text/views.ks`._

#### function `iter`

```kestrel
public func iter() -> SplitWhereViewIterator
```

_Defined in `lang/std/text/views.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> SplitWhereView
```

_Defined in `lang/std/text/views.ks`._

## struct `SplitWhereViewIterator`

```kestrel
public struct SplitWhereViewIterator { /* private fields */ }
```

Iterator that yields `StringSlice` segments produced by splitting at
every code point matching a predicate. The matching character is not
included in any segment.

_Defined in `lang/std/text/views.ks`._

### Members

#### initializer `init`

```kestrel
public init(slice: StringSlice, where: (Char) -> Bool)
```

_Defined in `lang/std/text/views.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = StringSlice
```

_Defined in `lang/std/text/views.ks`._

#### function `next`

```kestrel
public mutating func next() -> StringSlice?
```

_Defined in `lang/std/text/views.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> SplitWhereViewIterator
```

_Defined in `lang/std/text/views.ks`._

## protocol `Str`

```kestrel
public protocol Str
```

Shared read-only protocol for `String` and `StringSlice`.

Requires exactly one method from conformers: `asSlice()`. All
read-only methods are defined once in `extend Str` and inherited
by both types automatically.

_Defined in `lang/std/text/str.ks`._

### Members

#### function `asSlice`

```kestrel
func asSlice() -> StringSlice
```

_Defined in `lang/std/text/str.ks`._

#### field `byteCount`

```kestrel
public var byteCount: Int64 { get }
```

Number of UTF-8 bytes. O(1).

##### Examples

```
"hello".byteCount;       // 5
"\u{00E9}".byteCount;    // 2 (é is two UTF-8 bytes)
```

_Defined in `lang/std/text/str.ks`._

#### field `bytes`

```kestrel
public var bytes: BytesView { get }
```

View over the raw UTF-8 bytes.

##### Examples

```
"hi".bytes.count;  // 2
```

_Defined in `lang/std/text/str.ks`._

#### function `caseFolded`

```kestrel
public func caseFolded() -> String
```

Returns a new string with Unicode case folding applied to
each code point.

Case folding maps characters to a canonical form suitable
for case-insensitive comparison. Currently single-char folds
only (e.g. `A` → `a`); multi-char expansions like `ß` → `ss`
are not yet supported.

##### Examples

```
"Hello".caseFolded();  // "hello"
```

_Defined in `lang/std/text/str.ks`._

#### field `chars`

```kestrel
public var chars: CharsView { get }
```

View over Unicode code points.

##### Examples

```
"caf\u{00E9}".chars.count;  // 4
```

_Defined in `lang/std/text/str.ks`._

#### function `contains`

```kestrel
public func contains(String) -> Bool
```

Returns true if `substring` appears anywhere in this string.

##### Examples

```
"hello world".contains(substring: "world");  // true
"hello world".contains(substring: "xyz");    // false
```

_Defined in `lang/std/text/str.ks`._

#### function `contains`

```kestrel
public func contains(where: (Char) -> Bool) -> Bool
```

Returns true if any code point matches `predicate`.

##### Examples

```
"abc123".contains(where: { (c) in c.isAsciiDigit });  // true
```

_Defined in `lang/std/text/str.ks`._

#### function `ends`

```kestrel
public func ends(with: String) -> Bool
```

Returns true if this string ends with `suffix`.

Empty suffix always returns true. Comparison is byte-wise.

##### Examples

```
"hello".ends(with: "llo");  // true
"hello".ends(with: "xyz");  // false
```

_Defined in `lang/std/text/str.ks`._

#### function `equalsCaseInsensitive`

```kestrel
public func equalsCaseInsensitive(String) -> Bool
```

Compares two strings for equality after Unicode case folding.

Folds each string to its case-folded form and compares the
results byte-wise. Not normalization-aware — `é` (`U+00E9`)
and `e\u{0301}` are still considered different.

##### Examples

```
"Hello".equalsCaseInsensitive("HELLO");  // true
"Hello".equalsCaseInsensitive("World");  // false
```

_Defined in `lang/std/text/str.ks`._

#### function `firstIndex`

```kestrel
public func firstIndex(of: String) -> ByteIndex?
```

Returns the byte index of the first occurrence of `substring`,
or `None` if not found.

The empty substring matches at the start. Uses `memmem` for
efficient byte-level search.

##### Examples

```
"hello world".firstIndex(of: "world");  // Some(ByteIndex(6))
"hello world".firstIndex(of: "xyz");    // None
```

_Defined in `lang/std/text/str.ks`._

#### field `graphemes`

```kestrel
public var graphemes: GraphemesView { get }
```

View over grapheme clusters (user-perceived characters).

##### Examples

```
"caf\u{00E9}".graphemes.count;  // 4
```

_Defined in `lang/std/text/str.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

True when the string contains no bytes.

##### Examples

```
"".isEmpty;       // true
"hello".isEmpty;  // false
```

_Defined in `lang/std/text/str.ks`._

#### function `lastIndex`

```kestrel
public func lastIndex(of: String) -> ByteIndex?
```

Returns the byte index of the last occurrence of `substring`,
or `None` if not found.

Scans from the left using repeated `memmem` calls, keeping
the last match position.

##### Examples

```
"abcabc".lastIndex(of: "abc");  // Some(ByteIndex(3))
"abcabc".lastIndex(of: "xyz");  // None
```

_Defined in `lang/std/text/str.ks`._

#### field `lines`

```kestrel
public var lines: LinesView { get }
```

View over lines, recognising `\n`, `\r\n`, and `\r`.

##### Examples

```
"a\nb\nc".lines.count;  // 3
```

_Defined in `lang/std/text/str.ks`._

#### function `lowercased`

```kestrel
public func lowercased() -> String
```

Returns the lowercase form using full Unicode case mapping.

Locale-independent. Handles multi-character expansions
(e.g. Turkish dotted I). All-ASCII strings with no uppercase
letters short-circuit to `toOwned()` (no per-char decode).

##### Examples

```
"Hello".lowercased();      // "hello"
"\u{0130}".lowercased();   // "i\u{0307}"
```

_Defined in `lang/std/text/str.ks`._

#### function `lowercasedAscii`

```kestrel
public func lowercasedAscii() -> String
```

Returns a copy with only ASCII letters lowercased; non-ASCII
bytes pass through unchanged.

Cheap byte-level scan with no Unicode tables. For full
Unicode case mapping, use `lowercased()`.

##### Examples

```
"H\u{00E9}LLO".lowercasedAscii();  // "h\u{00E9}llo"
```

_Defined in `lang/std/text/str.ks`._

#### function `pad`

```kestrel
public func pad(leading: Int64, with: Char) -> String
```

Returns the string padded at the start with `char` so the
total *code-point* count is at least `length`.

If the string is already at least `length` code points long,
returns a copy unchanged.

##### Examples

```
"42".pad(leading: 5, with: '0');  // "00042"
```

_Defined in `lang/std/text/str.ks`._

#### function `pad`

```kestrel
public func pad(trailing: Int64, with: Char) -> String
```

Returns the string padded at the end with `char` so the
total *code-point* count is at least `length`.

If the string is already at least `length` code points long,
returns a copy unchanged.

##### Examples

```
"42".pad(trailing: 5, with: '.');  // "42..."
```

_Defined in `lang/std/text/str.ks`._

#### function `repeated`

```kestrel
public func repeated(Int64) -> String
```

Returns this string concatenated with itself `count` times.

Non-positive `count` returns the empty string. Pre-allocates
the result buffer for the exact final length.

##### Examples

```
"ab".repeated(3);  // "ababab"
"ab".repeated(0);  // ""
```

_Defined in `lang/std/text/str.ks`._

#### function `replaced`

```kestrel
public func replaced(String, with: String) -> String
```

Returns a copy with every occurrence of `pattern` replaced
by `replacement`.

Empty `pattern` is a no-op (returns a copy). Searches
greedily from the left and skips past each replacement so
substituted text is not re-matched.

##### Examples

```
"hello world".replaced("o", with: "0");    // "hell0 w0rld"
"abcabc".replaced("ab", with: "ABCD");     // "ABCDcABCDc"
```

_Defined in `lang/std/text/str.ks`._

#### function `split`

```kestrel
public func split(String) -> SplitView
```

Returns a lazy view that splits on `separator`, yielding
zero-copy `StringSlice` segments.

The empty separator is special-cased to split per code
point. Adjacent separators produce empty segments.

##### Examples

```
"a,b,c".split(",").collect();   // ["a", "b", "c"]
"a,,b".split(",").count;        // 3 (empty segment preserved)
```

_Defined in `lang/std/text/str.ks`._

#### function `split`

```kestrel
public func split(where: (Char) -> Bool) -> SplitWhereView
```

Returns a lazy view that splits at every code point matching
`predicate`, yielding zero-copy `StringSlice` segments.

The matching characters are not included in any segment.

##### Examples

```
"hello world".split(where: { (c) in c.isWhitespace }).count;  // 2
```

_Defined in `lang/std/text/str.ks`._

#### function `starts`

```kestrel
public func starts(with: String) -> Bool
```

Returns true if this string starts with `prefix`.

Empty prefix always returns true. Comparison is byte-wise.

##### Examples

```
"hello".starts(with: "hel");  // true
"hello".starts(with: "xyz");  // false
```

_Defined in `lang/std/text/str.ks`._

#### function `titlecased`

```kestrel
public func titlecased() -> String
```

Returns the titlecase form using full Unicode case mapping.

Word boundaries are detected by `Char.isWhitespace`; the
first non-space character of each run is titlecased and the
rest lowercased.

##### Examples

```
"hello world".titlecased();  // "Hello World"
"FOO BAR".titlecased();      // "Foo Bar"
```

_Defined in `lang/std/text/str.ks`._

#### function `toOwned`

```kestrel
public func toOwned() -> String
```

Copies this string's bytes into a new independent `String`.

For `String`, this is equivalent to `clone()`. For
`StringSlice`, it copies only the slice's bytes, releasing
the reference to the source buffer.

##### Examples

```
let slice = "hello world".asSlice();
let owned = slice.toOwned();  // independent copy
```

_Defined in `lang/std/text/str.ks`._

#### function `trimmed`

```kestrel
public func trimmed() -> StringSlice
```

Returns a zero-copy slice with leading and trailing ASCII
whitespace removed.

Whitespace characters: space (`' '`), tab (`'\t'`), newline
(`'\n'`), carriage return (`'\r'`), and form feed (`'\x0C'`).
The returned `StringSlice` shares the source buffer — no
allocation occurs.

##### Examples

```
"  hello  ".trimmed().toOwned();   // "hello"
"\t\n".trimmed().isEmpty;          // true
```

_Defined in `lang/std/text/str.ks`._

#### function `trimmed`

```kestrel
public func trimmed(where: (Char) -> Bool) -> StringSlice
```

Returns a zero-copy slice with leading and trailing code points
matching `predicate` removed.

Decodes the source one `Char` at a time. Leading characters
that satisfy the predicate are skipped; the trailing boundary
is the last character that does *not* match.

##### Examples

```
"00042".trimmed(where: { (c) in c.isEqual(to: '0') }).toOwned();  // "42"
```

_Defined in `lang/std/text/str.ks`._

#### function `trimmedEnd`

```kestrel
public func trimmedEnd() -> StringSlice
```

Returns a zero-copy slice with trailing whitespace removed.

See `trimmed()` for the whitespace set. Leading whitespace
is preserved.

##### Examples

```
"  hello  ".trimmedEnd().toOwned();  // "  hello"
```

_Defined in `lang/std/text/str.ks`._

#### function `trimmedEnd`

```kestrel
public func trimmedEnd(where: (Char) -> Bool) -> StringSlice
```

Returns a zero-copy slice with trailing code points matching
`predicate` removed. Leading matches are preserved.

##### Examples

```
"abc000".trimmedEnd(where: { (c) in c.isEqual(to: '0') }).toOwned();  // "abc"
```

_Defined in `lang/std/text/str.ks`._

#### function `trimmedStart`

```kestrel
public func trimmedStart() -> StringSlice
```

Returns a zero-copy slice with leading whitespace removed.

See `trimmed()` for the whitespace set. Trailing whitespace
is preserved.

##### Examples

```
"  hello  ".trimmedStart().toOwned();  // "hello  "
```

_Defined in `lang/std/text/str.ks`._

#### function `trimmedStart`

```kestrel
public func trimmedStart(where: (Char) -> Bool) -> StringSlice
```

Returns a zero-copy slice with leading code points matching
`predicate` removed. Trailing matches are preserved.

##### Examples

```
"000abc".trimmedStart(where: { (c) in c.isEqual(to: '0') }).toOwned();  // "abc"
```

_Defined in `lang/std/text/str.ks`._

#### function `uppercased`

```kestrel
public func uppercased() -> String
```

Returns the uppercase form using full Unicode case mapping.

Locale-independent. Handles multi-character expansions
(e.g. `ß` → `SS`). All-ASCII strings with no lowercase
letters short-circuit to `toOwned()`.

##### Examples

```
"hello".uppercased();             // "HELLO"
"stra\u{00DF}e".uppercased();     // "STRASSE"
```

_Defined in `lang/std/text/str.ks`._

#### function `uppercasedAscii`

```kestrel
public func uppercasedAscii() -> String
```

Returns a copy with only ASCII letters uppercased; non-ASCII
bytes pass through unchanged.

Cheap byte-level scan with no Unicode tables. For full
Unicode case mapping, use `uppercased()`.

##### Examples

```
"h\u{00E9}llo".uppercasedAscii();  // "H\u{00E9}LLO"
```

_Defined in `lang/std/text/str.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item
```

The element type that iteration yields.

_Defined in `lang/std/iter/iterator.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator
```

The concrete iterator type returned by `iter()`. Constrained so
`TargetIterator.Item` matches `Self.Item`.

_Defined in `lang/std/iter/iterator.ks`._

#### function `iter`

```kestrel
public func iter() -> CharsIterator
```

Returns a `CharsIterator` over the code points.

Required by `Iterable`. Each call returns a fresh iterator;
the source is reusable.

##### Examples

```
for c in "abc" { ... }  // iterates 'a', 'b', 'c'
```

_Defined in `lang/std/text/str.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Self) -> Bool
```

Returns true if both strings have the same byte sequence.

Pure byte-wise equality — not normalization-aware. For
case-insensitive comparison, see `equalsCaseInsensitive`.

##### Examples

```
"abc".isEqual(to: "abc");  // true
"abc".isEqual(to: "ABC");  // false
```

_Defined in `lang/std/text/str.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(Self) -> Ordering
```

Lexicographic byte-wise comparison.

Returns `Less` / `Equal` / `Greater` according to the first
differing byte; if one string is a prefix of the other, the
shorter is less.

##### Examples

```
"abc".compare("abd");  // Less
"abc".compare("ab");   // Greater
```

_Defined in `lang/std/text/str.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Hashes the byte content into `hasher`.

_Defined in `lang/std/text/str.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating StringBuilder, FormatOptions)
```

Formats the string using the given options.

##### Examples

```
"hi".format(FormatOptions(width: 5));  // "hi   "
```

_Defined in `lang/std/text/str.ks`._

## struct `String`

```kestrel
public struct String { /* private fields */ }
```

A UTF-8 encoded, dynamically sized string with copy-on-write semantics.

`String` is the standard text type. The bytes are always valid
UTF-8. Storage is shared between clones via an
`RcBox`; mutating a `String` whose storage is referenced elsewhere
triggers a copy. Three different views (`bytes`, `chars`,
`graphemes`) plus a `lines` view expose different units of
iteration over the same buffer.

### Examples

```
var s = "hello";
s.append(", world");
s.byteCount;            // 12
s.contains(",");  // true
for line in "a\nb".lines { /* ... */ }
```

### UTF-8

All public mutators preserve UTF-8 validity. The `bytes` view
returns raw `UInt8`s for hashing and FFI; the `chars` view decodes
code points; the `graphemes` view applies UAX #29 segmentation for
user-perceived characters. Choose the view that matches your unit:
byte-level work uses `bytes`, scalar-level work uses `chars`, and
anything user-visible (cursor movement, truncation) uses `graphemes`.

### Representation

A single `CowBox[StringStorage]` field. The storage record carries
`(ptr, len, cap)`; the empty string uses a null pointer with both
counts zero.

### Memory Model

Reference-counted, copy-on-write. Cloning is O(1); the first
mutation after a shared clone allocates and copies the bytes. The
raw byte pointer returned from `bytes` aliases the live buffer;
retain strings, not pointers.

### Guarantees

- Bytes are valid UTF-8 after every public mutator.
- `byteCount`, `capacity`, and `isEmpty` are O(1); `count` (code
  points) is O(n).
- Clones do not share mutation; `s.clone()` and `s` will diverge as
  soon as either is mutated.

_Defined in `lang/std/text/string.ks`._

### Members

#### initializer `Empty`

```kestrel
public init()
```

Constructs an empty string.

Allocates no buffer; the empty string is represented by a null
pointer with zero length and capacity. Required by
`Defaultable`.

##### Examples

```
let s = String();
s.isEmpty;     // true
s.byteCount;   // 0
```

_Defined in `lang/std/text/string.ks`._

#### initializer `From CString`

```kestrel
public init(from: CString)
```

Builds a `String` by copying the bytes out of `cstring`, excluding the null terminator.

O(n) — `cstring.length` walks to the terminator and the byte
copy is linear. Empty `CString`s (length zero) yield the
default empty `String` without touching the pointer.

##### Safety

`cstring.raw` must be valid for at least `length` readable
bytes plus a terminator. The conversion does not free the
`CString`'s buffer — caller still owns it.

##### Examples

```
let cstr = CString(raw: somePtr);
let s = String(from: cstr);
```

_Defined in `lang/std/ffi/cstring.ks`._

#### initializer `From Char Iterable`

```kestrel
public init[I](from: I) where I: Iterable, I.Item == Char
```

Builds a string by encoding each character of `chars` as UTF-8.

Mirrors `Array.init(from:)` and `Set.init(from:)` — accepts any
`Iterable` whose `Item` is `Char`. Useful for materializing the
result of an iterator chain back into a `String`:

```
let upper = String(from: "hello".chars.iter().map { it.toUpper() });
// "HELLO"
```

_Defined in `lang/std/text/string.ks`._

#### initializer `From Storage`

```kestrel
init(storage: CowBox[StringStorage])
```

Wraps an existing `CowBox[StringStorage]` as a new `String`.

Module-internal — used by `clone()`, `StringBuilder.build()`,
and other std.text code that constructs strings from raw storage.

_Defined in `lang/std/text/string.ks`._

#### initializer `From UTF-8`

```kestrel
public init[S](fromUtf8: S) where S: Slice[UInt8]
```

Constructs a string from validated UTF-8 bytes, returning `null`
if the input is not valid UTF-8.

##### Examples

```
let s = String(fromUtf8: "héllo".bytes);  // Some("héllo")
```

_Defined in `lang/std/text/string.ks`._

#### initializer `From UTF-8 Lossy`

```kestrel
public init[S](fromUtf8Lossy: S) where S: Slice[UInt8]
```

Constructs a string from bytes, replacing invalid UTF-8 sequences
with the Unicode replacement character (U+FFFD).

##### Examples

```
let s = String(fromUtf8Lossy: mixedBytes);  // invalid bytes become '�'
```

_Defined in `lang/std/text/string.ks`._

#### initializer `From UTF-8 Unchecked`

```kestrel
public init[S](fromUtf8Unchecked: S) where S: Slice[UInt8]
```

Constructs a string by copying bytes without UTF-8 validation.

##### Safety

The caller must ensure the bytes are valid UTF-8.

_Defined in `lang/std/text/string.ks`._

#### initializer `String Literal`

```kestrel
public init(stringLiteral: lang.ptr[lang.i8], lang.i64)
```

Compiler-emitted constructor for string literals.

Receives a static byte pointer and length, then memcpys into a
fresh heap allocation so the resulting `String` owns its bytes
(and can be mutated independently of the literal pool).

##### Errors

Panics with `"String allocation failed"` if the system
allocator returns null.

_Defined in `lang/std/text/string.ks`._

#### initializer `With Capacity`

```kestrel
public init(capacity: Int64)
```

Constructs an empty string with at least `capacity` bytes preallocated.

Useful before a series of appends whose total byte count is
known: avoids the geometric-growth reallocations the default
constructor would incur. A non-positive `capacity` is treated
as zero.

##### Errors

Panics with `"String allocation failed"` if the system
allocator returns null.

##### Examples

```
var s = String(capacity: 64);
s.byteCount;  // 0
s.capacity;   // 64
```

_Defined in `lang/std/text/string.ks`._

#### function `_appendBytes`

```kestrel
mutating func _appendBytes(Pointer[UInt8], Int64)
```

Appends `n` bytes from `ptr` via `memcpy`. Internal — caller
ensures the bytes preserve UTF-8 validity.

##### Safety

`ptr` must reference at least `n` valid UTF-8 bytes that, when
concatenated to the current buffer, yield valid UTF-8.

_Defined in `lang/std/text/string.ks`._

#### function `append`

```kestrel
public mutating func append[__opaque_0](__opaque_0) where __opaque_0: Str
```

Appends `other`'s bytes to this string. COW.

Triggers a copy if storage is shared. Empty appends are a fast
no-op.

##### Examples

```
var s = "hello";
s.append(", world");
s;  // "hello, world"
```

_Defined in `lang/std/text/string.ks`._

#### function `append`

```kestrel
public mutating func append(char: Char)
```

Appends a single code point, encoding it as UTF-8.

Sizes the buffer for the encoded length (1–4 bytes) before
writing.

##### Examples

```
var s = "h";
s.append(char: 'i');
s.append(char: '\u{1F600}');
s;  // "hi😀"
```

_Defined in `lang/std/text/string.ks`._

#### function `appendByte`

```kestrel
internal mutating func appendByte(UInt8)
```

Appends a raw byte. Internal — caller ensures UTF-8 validity.

Do not use to append ASCII characters: prefer `appendChar(c)` or
`append(other)`. This exists only for low-level UTF-8 plumbing
inside the stdlib (e.g. an encoder that already produced bytes).

_Defined in `lang/std/text/string.ks`._

#### field `byteCount`

```kestrel
public var byteCount: Int64 { get }
```

The number of UTF-8 bytes in the string. O(1).

This is **not** the character count — see `count` for that.
Pure ASCII strings have `byteCount == count`.

_Defined in `lang/std/text/string.ks`._

#### field `bytes`

```kestrel
public var bytes: BytesView { get }
```

`s.bytes` — view over the raw UTF-8 bytes. O(1) byte indexing,
byte-level iteration. Index via the view's subscripts:
`s.bytes(i)`, `s.bytes(checked: i)`, `s.bytes(0..<n)`.

_Defined in `lang/std/text/string.ks`._

#### field `capacity`

```kestrel
public var capacity: Int64 { get }
```

The number of bytes the storage buffer can hold without reallocating. O(1).

_Defined in `lang/std/text/string.ks`._

#### field `chars`

```kestrel
public var chars: CharsView { get }
```

`s.chars` — view over the Unicode code points. O(n) indexing,
scalar-level iteration. Index via the view's subscripts:
`s.chars(i)`, `s.chars(checked: i)`.

_Defined in `lang/std/text/string.ks`._

#### function `clear`

```kestrel
public mutating func clear()
```

Truncates the string to length zero, keeping the allocated buffer.

Capacity is unchanged, so this is the right primitive for
reusing a buffer in a hot loop.

_Defined in `lang/std/text/string.ks`._

#### function `fromBytesUnchecked`

```kestrel
static func fromBytesUnchecked(Pointer[UInt8], Int64) -> String
```

Internal helper: copies `count` bytes from `ptr` without validation.

_Defined in `lang/std/text/string.ks`._

#### function `fromRawBytes`

```kestrel
static func fromRawBytes(lang.ptr[lang.i8], Int64) -> String
```

Internal helper: copies `count` bytes from a raw `lang.ptr[lang.i8]`.

_Defined in `lang/std/text/string.ks`._

#### field `graphemes`

```kestrel
public var graphemes: GraphemesView { get }
```

`s.graphemes` — view over user-perceived characters
(UAX #29 grapheme clusters). Iterate or count, no random access.

_Defined in `lang/std/text/string.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

True if the string holds zero bytes. O(1).

_Defined in `lang/std/text/string.ks`._

#### field `lines`

```kestrel
public var lines: LinesView { get }
```

A view over the lines of the string, recognising `\n`, `\r\n`, and `\r`.

_Defined in `lang/std/text/string.ks`._

#### function `lowercase`

```kestrel
public mutating func lowercase()
```

Replaces this string with its lowercase form using full Unicode case mapping.

_Defined in `lang/std/text/string.ks`._

#### function `lowercaseAscii`

```kestrel
public mutating func lowercaseAscii()
```

Lowercases ASCII letters in place; non-ASCII bytes are left untouched.

Cheap byte-level scan with no Unicode tables. For locale-
independent Unicode case folding, use `lowercase`.

##### Examples

```
var s = "HéLLO";
s.lowercaseAscii();
s;  // "héllo" — only ASCII letters touched
```

_Defined in `lang/std/text/string.ks`._

#### function `replace`

```kestrel
public mutating func replace(String, with: String)
```

Replaces every occurrence of `pattern` with `replacement`, in place.

_Defined in `lang/std/text/string.ks`._

#### function `substringBytes`

```kestrel
internal func substringBytes(from: Int64, to: Int64) -> String
```

Internal substring by byte range. Returns empty for invalid ranges.

Do not use for per-character slicing in a loop — each call copies
`end - start` bytes, so walking the string yields O(N²) behaviour.
For iteration, use `decodeUtf8` with a running byte offset, or the
`chars()` / `bytes()` views.

_Defined in `lang/std/text/string.ks`._

#### function `toCString`

```kestrel
public func toCString() -> CString
```

Allocates a fresh null-terminated copy of this string and returns it as a `CString`.

Sizes the buffer to `byteCount + 1`, copies the source bytes
via `memcpy`, and writes the trailing `\0`. The caller takes
ownership and must release the buffer with `cstr.free()`.

##### Safety

The returned `CString` aliases freshly allocated memory; do
not pass it to a C function that takes ownership of the
pointer (it will then be double-freed) and do not forget to
free it.

##### Examples

```
let cstr = "Hello, C!".toCString();
let _ = puts(cstr);
cstr.free();
```

_Defined in `lang/std/ffi/cstring.ks`._

#### function `trim`

```kestrel
public mutating func trim()
```

Removes leading and trailing ASCII whitespace in place.

Recognises the same whitespace set as `Char.isWhitespace`:
space, tab, LF, CR, form feed. For Unicode-aware trimming, use
the `(where:)` overloads with a custom predicate. Non-mutating
mirrors live under `trimmed*`.

##### Examples

```
var s = "  hi  ";
s.trim();
s;  // "hi"
```

_Defined in `lang/std/text/string.ks`._

#### function `trim`

```kestrel
public mutating func trim(where: (Char) -> Bool)
```

Removes leading and trailing code points matching `predicate`, in place.

##### Examples

```
var s = "***hi***";
s.trim { (c) in c == '*' };
s;  // "hi"
```

_Defined in `lang/std/text/string.ks`._

#### function `trimEnd`

```kestrel
public mutating func trimEnd()
```

Removes trailing ASCII whitespace in place.

_Defined in `lang/std/text/string.ks`._

#### function `trimEnd`

```kestrel
public mutating func trimEnd(where: (Char) -> Bool)
```

Removes trailing code points matching `predicate`, in place.

_Defined in `lang/std/text/string.ks`._

#### function `trimStart`

```kestrel
public mutating func trimStart()
```

Removes leading ASCII whitespace in place.

_Defined in `lang/std/text/string.ks`._

#### function `trimStart`

```kestrel
public mutating func trimStart(where: (Char) -> Bool)
```

Removes leading code points matching `predicate`, in place.

_Defined in `lang/std/text/string.ks`._

#### function `uppercase`

```kestrel
public mutating func uppercase()
```

Replaces this string with its uppercase form using full Unicode case mapping.

Locale-independent. Handles multi-character expansions —
e.g. German `ß` → `SS`.

_Defined in `lang/std/text/string.ks`._

#### function `uppercaseAscii`

```kestrel
public mutating func uppercaseAscii()
```

Uppercases ASCII letters in place; non-ASCII bytes are left untouched.

_Defined in `lang/std/text/string.ks`._

### Implements `Str`

#### function `asSlice`

```kestrel
public func asSlice() -> StringSlice
```

Returns a `StringSlice` covering this string's entire buffer.
Shares storage via refcount — zero-copy.

_Defined in `lang/std/text/string.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = Char
```

The element type yielded by iteration — always `Char`.

_Defined in `lang/std/text/string.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = CharsIterator
```

The iterator type returned by `iter()`.

_Defined in `lang/std/text/string.ks`._

#### function `iter`

```kestrel
public func iter() -> CharsIterator
```

Returns a `CharsIterator` over the code points starting at byte 0.

Required by `Iterable`. Each call returns a fresh iterator;
the string itself is reusable.

_Defined in `lang/std/text/string.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: String) -> Bool
```

Returns true if both strings have the same byte sequence.

Pure byte-wise equality — not normalization-aware. For
case-insensitive comparison, see `equalsCaseInsensitive`.

##### Examples

```
"abc".isEqual(to: "abc");  // true
"abc".isEqual(to: "ABC");  // false
```

_Defined in `lang/std/text/string.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(String) -> Bool
```

Pattern-match form of `isEqual`: each `case "literal" =>` arm
dispatches through here. Cost is `O(len)` per arm because the
compiler emits one call per literal — past a handful of arms,
E316 will suggest an `if/else if` chain instead.

_Defined in `lang/std/text/string.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(String) -> Ordering
```

Lexicographic byte-wise comparison.

Returns `Less` / `Equal` / `Greater` according to the first
differing byte; if one string is a prefix of the other, the
shorter is less. Byte order coincides with code-point order
because UTF-8 is order-preserving — this is *not* the same as
locale-aware collation.

##### Examples

```
"abc".compare("abd");  // Less
"abc".compare("ab");   // Greater
"abc".compare("abc");  // Equal
```

_Defined in `lang/std/text/string.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> String
```

Returns a shallow clone — storage is shared until either side mutates.

O(1). Mutation triggers a deep copy via `CowBox.write()`.

_Defined in `lang/std/text/string.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating StringBuilder, FormatOptions)
```

Renders this string under the supplied `FormatOptions`.

Honours `width`, `alignment`, and `fill`. `precision` /
`radix` / `floatStyle` / `sign` are ignored — they don't apply
to strings. Aligned padding is measured in *code points*, not
bytes, so multi-byte characters count as one column for
alignment purposes (display width still depends on font).

##### Examples

```
var opts = FormatOptions();
opts.width = .Some(10);
opts.alignment = .Left;
"test".format(opts);   // "test      "
opts.alignment = .Right;
"test".format(opts);   // "      test"
opts.alignment = .Center;
"test".format(opts);   // "   test   "
```

_Defined in `lang/std/text/string.ks`._

### Implements `Addable`

#### typealias `Output`

```kestrel
type Output = String
```

The output type of `+` (concatenation) — always `String`.

_Defined in `lang/std/text/string.ks`._

#### function `add`

```kestrel
public func add(String) -> String
```

Returns the concatenation `self + other`. Required by `Addable`.

Equivalent to cloning `self` and appending `other`.

_Defined in `lang/std/text/string.ks`._

#### field `zero`

```kestrel
public static var zero: String { get }
```

The additive identity for strings — the empty string `""`.

_Defined in `lang/std/text/string.ks`._

### Implements `ExpressibleByStringLiteral`

#### initializer `String Literal`

```kestrel
init(stringLiteral: lang.ptr[lang.i8], lang.i64)
```

Builds an instance from a string literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Hashes the raw byte sequence into the supplied hasher.

Sends the whole buffer in a single `write` so the hasher gets
to choose how to consume it.

_Defined in `lang/std/text/string.ks`._

### Implements `Defaultable`

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

### Implements `Convertible`

#### initializer `From Source`

```kestrel
init(from: From)
```

Creates an instance from `value`.

_Defined in `lang/std/core/convertible.ks`._

## struct `StringBuilder`

```kestrel
public struct StringBuilder { /* private fields */ }
```

Write-only buffer for efficient string construction. No COW, no
RcBox, no `isUnique` checks — every append writes directly.

`build()` transfers ownership of the buffer into a new `String`
without copying. The builder resets to empty and can be reused.

### Examples

```
var b = StringBuilder();
b.append("hello");
b.append(char: ' ');
b.append("world");
let s = b.build();   // "hello world", zero-copy
```

### Representation

`(ptr: Pointer[UInt8], len: Int64, cap: Int64)`.

### Memory Model

Owns its buffer directly. `build()` donates the buffer to a
`String`; the builder is left empty. `deinit` frees the buffer
if `build()` was never called.

_Defined in `lang/std/text/builder.ks`._

### Members

#### initializer `Empty`

```kestrel
public init()
```

Creates an empty builder with no allocation.

_Defined in `lang/std/text/builder.ks`._

#### initializer `With Capacity`

```kestrel
public init(capacity: Int64)
```

Creates an empty builder with at least `capacity` bytes preallocated.

_Defined in `lang/std/text/builder.ks`._

#### function `append`

```kestrel
public mutating func append[__opaque_0](__opaque_0) where __opaque_0: Str
```

Appends the UTF-8 bytes of `other` to this builder. Accepts any
type conforming to `Str` — `String`, `StringSlice`, etc.

_Defined in `lang/std/text/builder.ks`._

#### function `append`

```kestrel
public mutating func append(char: Char)
```

Appends a single code point, encoding it as UTF-8.

_Defined in `lang/std/text/builder.ks`._

#### function `appendByte`

```kestrel
internal mutating func appendByte(UInt8)
```

Appends a raw byte. Caller must ensure UTF-8 validity.

_Defined in `lang/std/text/builder.ks`._

#### function `appendBytes`

```kestrel
internal mutating func appendBytes(ptr: Pointer[UInt8], count: Int64)
```

Appends `count` bytes from `ptr`. Caller must ensure UTF-8 validity.

_Defined in `lang/std/text/builder.ks`._

#### function `build`

```kestrel
public mutating func build() -> String
```

Transfers the buffer into a new `String` without copying.
The builder resets to empty and can be reused.

_Defined in `lang/std/text/builder.ks`._

#### field `byteCount`

```kestrel
public var byteCount: Int64 { get }
```

Number of bytes written so far.

_Defined in `lang/std/text/builder.ks`._

#### function `clear`

```kestrel
public mutating func clear()
```

Resets length to zero, keeping the allocated buffer for reuse.

_Defined in `lang/std/text/builder.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

True when nothing has been written.

_Defined in `lang/std/text/builder.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> StringBuilder
```

Returns a copy with its own buffer.

_Defined in `lang/std/text/builder.ks`._

## protocol `StringIndex`

```kestrel
public protocol StringIndex
```

Protocol for typed string indices. Each index wraps a pre-resolved
byte offset; the type tag determines what unit the index addresses
and what the subscript returns.

_Defined in `lang/std/text/slice.ks`._

### Members

#### typealias `Yield`

```kestrel
type Yield
```

_Defined in `lang/std/text/slice.ks`._

#### function `read`

```kestrel
func read(from: StringSlice) -> Yield
```

_Defined in `lang/std/text/slice.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
func isEqual(to: Self) -> Bool
```

Returns `true` iff `self` and `other` are considered equal. Should
be reflexive, symmetric, and transitive — `Hashable` requires equal
values to hash equal, so don't drift from those laws.

_Defined in `lang/std/core/protocols.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
func compare(Self) -> Ordering
```

Returns the ordering of `self` relative to `other`. Must be a
total order — for any `a`, `b`, `c` exactly one of `Less`,
`Equal`, `Greater` holds, and the order is transitive.

_Defined in `lang/std/core/protocols.ks`._

## struct `StringSlice`

```kestrel
public struct StringSlice { /* private fields */ }
```

An immutable window into a `String`'s UTF-8 bytes with shared
ownership. The central read-only abstraction of the text library.

Zero-cost to create from a String (share the RcBox, cover the
whole range). Zero-cost to narrow (adjust start/end). Keeps the
source alive as long as the slice exists.

### Examples

```
let s = "hello, world";
let slice = s.asSlice();
slice.byteCount;              // 12
slice.toOwned();               // "hello, world"
```

### Representation

`(source: RcBox[StringStorage], start: Int64, end: Int64)`.

### Memory Model

Shared ownership via `RcBox`. The source string's buffer stays
alive as long as any slice references it. Call `.toOwned()` to
copy just the slice's bytes into an independent `String`.

_Defined in `lang/std/text/slice.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(source: RcBox[StringStorage], start: Int64, end: Int64)
```

Creates a slice covering `[start, end)` in the given storage.

_Defined in `lang/std/text/slice.ks`._

#### function `_rawPtr`

```kestrel
func _rawPtr() -> Pointer[UInt8]
```

_Defined in `lang/std/text/slice.ks`._

#### function `_readByte`

```kestrel
func _readByte(at: Int64) -> UInt8
```

_Defined in `lang/std/text/slice.ks`._

#### field `byteCount`

```kestrel
public var byteCount: Int64 { get }
```

Number of UTF-8 bytes in this slice. O(1).

_Defined in `lang/std/text/slice.ks`._

#### field `end`

```kestrel
public var end: Int64
```

_Defined in `lang/std/text/slice.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

True when the slice covers zero bytes.

_Defined in `lang/std/text/slice.ks`._

#### field `source`

```kestrel
var source: RcBox[StringStorage]
```

_Defined in `lang/std/text/slice.ks`._

#### field `start`

```kestrel
public var start: Int64
```

_Defined in `lang/std/text/slice.ks`._

#### function `subslice`

```kestrel
public func subslice(from: Int64, to: Int64) -> StringSlice
```

Returns a sub-slice covering `[newStart, newEnd)` relative to
the source buffer (absolute byte offsets, not relative to this
slice's start).

_Defined in `lang/std/text/slice.ks`._

#### function `toOwned`

```kestrel
public func toOwned() -> String
```

Copies just this slice's bytes into a new independent `String`.

_Defined in `lang/std/text/slice.ks`._

### Implements `Str`

#### function `asSlice`

```kestrel
public func asSlice() -> StringSlice
```

Returns self — StringSlice is already a slice.

_Defined in `lang/std/text/slice.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: StringSlice) -> Bool
```

_Defined in `lang/std/text/slice.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(StringSlice) -> Ordering
```

_Defined in `lang/std/text/slice.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

_Defined in `lang/std/text/slice.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> StringSlice
```

_Defined in `lang/std/text/slice.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating StringBuilder, FormatOptions)
```

_Defined in `lang/std/text/slice.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = Char
```

_Defined in `lang/std/text/slice.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = CharsIterator
```

_Defined in `lang/std/text/slice.ks`._

#### function `iter`

```kestrel
public func iter() -> CharsIterator
```

Iterates code points in this slice.

_Defined in `lang/std/text/slice.ks`._

## struct `Utf8DecodeResult`

```kestrel
public struct Utf8DecodeResult { /* private fields */ }
```

The output of decoding one UTF-8 character from a byte buffer.

Carries both the decoded `Char` and the number of bytes consumed,
so the caller can advance their cursor without re-running
`utf8Length()`. Returned as `Some` from `decodeUtf8`; `None`
indicates an invalid or truncated sequence.

### Examples

```
let r = Utf8DecodeResult(char: 'a', bytesConsumed: 1);
r.char;           // 'a'
r.bytesConsumed;  // 1
```

### Representation

A plain pair `(char: Char, bytesConsumed: Int64)`. Both fields are
public to keep the type cheap to inspect.

_Defined in `lang/std/text/char.ks`._

### Members

#### initializer `From Fields`

```kestrel
public init(char: Char, bytesConsumed: Int64)
```

Constructs a decode result from an already-decoded char and byte length.

Mainly used by `decodeUtf8` itself; user code rarely needs to
build one directly.

_Defined in `lang/std/text/char.ks`._

#### field `bytesConsumed`

```kestrel
public var bytesConsumed: Int64
```

How many bytes the encoded form occupied (1–4).

_Defined in `lang/std/text/char.ks`._

#### field `char`

```kestrel
public var char: Char
```

The decoded character.

_Defined in `lang/std/text/char.ks`._

## function `_writePadded`

```kestrel
public func _writePadded(into: mutating StringBuilder, String, FormatOptions)
```

Writes `content` into `writer` with width/alignment/fill padding applied.
Used by String, integer, and float `format(into:)` implementations.

_Defined in `lang/std/text/format.ks`._

## function `decodeUtf8`

```kestrel
public func decodeUtf8(lang.ptr[lang.i8], Int64, at: Int64) -> Utf8DecodeResult?
```

Decodes one UTF-8 character starting at `index` inside the buffer of `length` bytes pointed to by `ptr`.

Returns `Some(Utf8DecodeResult)` on success, where `bytesConsumed`
is `1`–`4`. Returns `None` for any of the malformed-input cases:
truncated multi-byte sequence, continuation byte where a leading
byte was expected, or invalid leading byte (`0xF8`–`0xFF`).
**Does not** validate against overlong encodings or surrogate-range
scalars — feed only well-formed UTF-8 if those matter.

### Safety

`ptr` must be valid for `length` bytes. The function bounds-checks
`index` and any continuation bytes against `length`.

### Examples

```
var result = String("hé");
// Conceptually:
// decodeUtf8(rawPtr, 3, at: 0)  // Some(char: 'h', bytesConsumed: 1)
// decodeUtf8(rawPtr, 3, at: 1)  // Some(char: 'é', bytesConsumed: 2)
// decodeUtf8(rawPtr, 3, at: 3)  // None (past the end)
```

_Defined in `lang/std/text/char.ks`._

## function `encodeUtf8`

```kestrel
public func encodeUtf8(Char, lang.ptr[lang.i8], at: Int64) -> Int64
```

Encodes `c` as UTF-8 starting at `ptr + index`, returning the number of bytes written (1–4).

Companion of `decodeUtf8`. `c.utf8Length()` predicts the same byte
count without writing — call it first to ensure the buffer has
room.

### Safety

`ptr + index` through `ptr + index + utf8Length() - 1` must lie
within an allocated, writable region. No bounds checking happens
here.

### Examples

```
// Conceptually, given a buffer `buf` of length 4:
// encodeUtf8('a',         buf, at: 0);  // 1
// encodeUtf8('\u{1F600}', buf, at: 0);  // 4
```

_Defined in `lang/std/text/char.ks`._

