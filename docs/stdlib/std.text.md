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

#### function `equals`

```kestrel
public func equals(Alignment) -> Bool
```

Returns true if both cases are the same variant.

Equality is structural — there are no payloads. Used by the
`Equatable` conformance so `FormatOptions.equals` can fall through
without payload comparisons.

##### Examples

```
Alignment.Left.equals(.Left);    // true
Alignment.Left.equals(.Center);  // false
```

_Defined in `lang/std/text/format.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(Alignment) -> Bool
```

Pattern-match form of equality — delegates to `equals`.

Lets `Alignment` appear in `match` patterns against another value.

##### Examples

```
Alignment.Right.matches(.Right);  // true
```

_Defined in `lang/std/text/format.ks`._

## struct `AsciiChars`

```kestrel
public struct AsciiChars { /* private fields */ }
```

Namespace of common ASCII punctuation and whitespace characters.

Static factories rather than constants because Kestrel does not yet
have non-trivial `static let` initialisation; calls are cheap and
fully constant-folded.

### Examples

```
AsciiChars.space() == ' ';        // true
AsciiChars.newline() == '\n';     // true
AsciiChars.colon() == ':';        // true
```

_Defined in `lang/std/text/char.ks`._

### Members

#### function `apostrophe`

```kestrel
public static func apostrophe() -> Char
```

The single-quote / apostrophe character `'\''`.

_Defined in `lang/std/text/char.ks`._

#### function `backslash`

```kestrel
public static func backslash() -> Char
```

The backslash character `'\\'`.

_Defined in `lang/std/text/char.ks`._

#### function `carriageReturn`

```kestrel
public static func carriageReturn() -> Char
```

The carriage return character `'\r'` (`U+000D`).

_Defined in `lang/std/text/char.ks`._

#### function `colon`

```kestrel
public static func colon() -> Char
```

The colon character `':'`.

_Defined in `lang/std/text/char.ks`._

#### function `comma`

```kestrel
public static func comma() -> Char
```

The comma character `','`.

_Defined in `lang/std/text/char.ks`._

#### function `dot`

```kestrel
public static func dot() -> Char
```

The period character `'.'`.

_Defined in `lang/std/text/char.ks`._

#### function `newline`

```kestrel
public static func newline() -> Char
```

The newline character `'\n'` (`U+000A`, line feed).

_Defined in `lang/std/text/char.ks`._

#### function `nul`

```kestrel
public static func nul() -> Char
```

The null character `'\0'` (`U+0000`).

_Defined in `lang/std/text/char.ks`._

#### function `quote`

```kestrel
public static func quote() -> Char
```

The double-quote character `'"'`.

_Defined in `lang/std/text/char.ks`._

#### function `semicolon`

```kestrel
public static func semicolon() -> Char
```

The semicolon character `';'`.

_Defined in `lang/std/text/char.ks`._

#### function `slash`

```kestrel
public static func slash() -> Char
```

The forward-slash character `'/'`.

_Defined in `lang/std/text/char.ks`._

#### function `space`

```kestrel
public static func space() -> Char
```

The space character `' '` (`U+0020`).

_Defined in `lang/std/text/char.ks`._

#### function `tab`

```kestrel
public static func tab() -> Char
```

The horizontal tab character `'\t'` (`U+0009`).

_Defined in `lang/std/text/char.ks`._

## typealias `Byte`

```kestrel
public type Byte = UInt8
```

One byte of UTF-8 (or any other) encoded text — alias for `UInt8`.

_Defined in `lang/std/text/char.ks`._

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

#### field `value`

```kestrel
public var value: Int64
```

The wrapped byte offset.

_Defined in `lang/std/text/views.ks`._

### Implements `Equatable`

#### function `equals`

```kestrel
public func equals(ByteIndex) -> Bool
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

#### initializer `From Pointer`

```kestrel
public init(ptr: lang.ptr[lang.i8], length: Int64)
```

Constructs a bytes view from a raw pointer and a byte count.

Prefer `someString.bytes` over calling this directly.

##### Safety

`ptr` must point to `length` valid bytes that remain live for as
long as the view is used.

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

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` if the view spans zero bytes.

_Defined in `lang/std/text/views.ks`._

#### function `substring`

```kestrel
public func substring[I](I) -> String where I: BytesSubstringIndex
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
a.isAlphabetic();    // true
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

#### initializer `From Value`

```kestrel
public init(UInt32)
```

Wraps a raw `UInt32` scalar value as a `Char`.

No range or surrogate validation is performed; pass values you
already know are valid Unicode scalars. Prefer the literal syntax
(`'a'`, `'\u{...}'`) when the value is known at compile time.

##### Examples

```
let c = Char(value: UInt32(intLiteral: 0x41));
c == 'A';  // true
```

_Defined in `lang/std/text/char.ks`._

#### function `digitValue`

```kestrel
public func digitValue() -> UInt32?
```

Returns the numeric value `0`–`9` for ASCII digits, otherwise `None`.

Inverse of `fromDigit`. Non-ASCII digit characters return `None`
— match `isDigit` semantics.

##### Examples

```
'7'.digitValue();  // Some(7)
'a'.digitValue();  // None
```

_Defined in `lang/std/text/char.ks`._

#### function `fromDigit`

```kestrel
public static func fromDigit(UInt32) -> Char?
```

Returns the ASCII digit `Char` for a numeric value `0`–`9`, otherwise `None`.

Inverse of `digitValue`. Values outside `0..=9` return `None`.

##### Examples

```
Char.fromDigit(d: 7);   // Some('7')
Char.fromDigit(d: 12);  // None
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

#### function `isAlphabetic`

```kestrel
public func isAlphabetic() -> Bool
```

Returns true for ASCII letters `A`–`Z` / `a`–`z`.

**ASCII-only.** Non-ASCII letters (e.g. `é`, `Ω`, `日`) return
`false` even though they are letters in Unicode. For the full
Unicode test, use the property tables in `std.text.unicode`.

##### Examples

```
'A'.isAlphabetic();         // true
'\u{00E9}'.isAlphabetic();  // false (é — non-ASCII)
'7'.isAlphabetic();         // false
```

_Defined in `lang/std/text/char.ks`._

#### function `isAlphanumeric`

```kestrel
public func isAlphanumeric() -> Bool
```

Returns true for ASCII letters and ASCII digits.

Composition of `isAlphabetic` and `isDigit`; same ASCII-only
caveats apply.

##### Examples

```
'a'.isAlphanumeric();  // true
'7'.isAlphanumeric();  // true
'_'.isAlphanumeric();  // false
```

_Defined in `lang/std/text/char.ks`._

#### function `isAscii`

```kestrel
public func isAscii() -> Bool
```

Returns true if the scalar is in the ASCII range (`< 0x80`).

Cheap byte-range test; does not consult Unicode tables. For
"alphabetic by Unicode" use `unicode.toLowercase` round-tripping
or the property tables directly.

##### Examples

```
'A'.isAscii();          // true
'\u{00E9}'.isAscii();   // false (é)
```

_Defined in `lang/std/text/char.ks`._

#### function `isControl`

```kestrel
public func isControl() -> Bool
```

Returns true for the C0 controls (`< U+0020`) and DEL (`U+007F`).

Does not include the C1 controls (`U+0080`–`U+009F`); add a
dedicated test if you need them.

##### Examples

```
'\n'.isControl();     // true
'\x7F'.isControl();   // true
'a'.isControl();      // false
```

_Defined in `lang/std/text/char.ks`._

#### function `isDigit`

```kestrel
public func isDigit() -> Bool
```

Returns true for the ASCII digits `0`–`9`.

**ASCII-only.** Other Unicode digit categories (Arabic-Indic,
Devanagari, etc.) return `false`. See `digitValue()` for parsing
to numeric value.

##### Examples

```
'7'.isDigit();   // true
'a'.isDigit();   // false
```

_Defined in `lang/std/text/char.ks`._

#### function `isLowercase`

```kestrel
public func isLowercase() -> Bool
```

Returns true for ASCII lowercase letters `a`–`z`.

**ASCII-only.** Use `unicode.toLowercase` round-tripping for
general Unicode case tests.

##### Examples

```
'a'.isLowercase();   // true
'A'.isLowercase();   // false
```

_Defined in `lang/std/text/char.ks`._

#### function `isUppercase`

```kestrel
public func isUppercase() -> Bool
```

Returns true for ASCII uppercase letters `A`–`Z`.

**ASCII-only.** Use `unicode.toUppercase` round-tripping for
general Unicode case tests.

##### Examples

```
'A'.isUppercase();         // true
'a'.isUppercase();         // false
'\u{00C9}'.isUppercase();  // false (É — non-ASCII)
```

_Defined in `lang/std/text/char.ks`._

#### function `isWhitespace`

```kestrel
public func isWhitespace() -> Bool
```

Returns true for the common ASCII whitespace set: space, tab, LF, CR, form feed.

Does not include Unicode whitespace such as `U+00A0` (no-break
space) or `U+2028` (line separator). For Unicode-aware
whitespace, consult the property tables.

##### Examples

```
' '.isWhitespace();    // true
'\t'.isWhitespace();   // true
'\n'.isWhitespace();   // true
'a'.isWhitespace();    // false
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

#### function `equals`

```kestrel
public func equals(Char) -> Bool
```

Returns true if both characters are the same Unicode scalar.

Pure scalar-value equality — no case folding, no normalization.

##### Examples

```
'a'.equals('a');  // true
'a'.equals('A');  // false
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

Pattern-match form of equality — delegates to `equals`.

_Defined in `lang/std/text/char.ks`._

### Implements `ExpressibleByCharLiteral`

#### initializer `Char Literal`

```kestrel
init(charLiteral: lang.i32)
```

Builds an instance from a character literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `Hash`

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

#### field `byteOffset`

```kestrel
public var byteOffset: Int64
```

The byte offset where the indexed character begins.

_Defined in `lang/std/text/views.ks`._

### Implements `Equatable`

#### function `equals`

```kestrel
public func equals(CharIndex) -> Bool
```

Returns true if the two indices point at the same byte offset.

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

#### initializer `From Pointer`

```kestrel
public init(ptr: lang.ptr[lang.i8], length: Int64)
```

Constructs a chars view from a raw pointer and a byte length.

Prefer `someString.chars` over calling this directly.

##### Safety

`ptr` must point to `length` valid UTF-8 bytes that remain live
for the view's lifetime.

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

#### field `count`

```kestrel
public var count: Int64 { get }
```

_Defined in `lang/std/text/views.ks`._

#### function `substring`

```kestrel
public func substring[I](I) -> String where I: CharsSubstringIndex
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

A single `Array[String]` of accumulated parts. Empty literal pieces
are dropped on append.

_Defined in `lang/std/text/format.ks`._

### Members

#### function `build`

```kestrel
public func build() -> String
```

Concatenates all recorded parts into the final `String`.

Fast paths the zero-part and one-part cases. For the multi-part
case, computes the exact total byte length first, allocates once
at that size, then appends — no growth churn.

##### Examples

```
var acc = DefaultStringInterpolation(literalCapacity: 0, interpolationCount: 0);
acc.appendLiteral("a");
acc.appendLiteral("b");
acc.build();  // "ab"
```

_Defined in `lang/std/text/format.ks`._

#### initializer `init`

```kestrel
public init(literalCapacity: Int64, interpolationCount: Int64)
```

_Defined in `lang/std/text/format.ks`._

### Implements `Interpolatable`

#### initializer `With Capacity`

```kestrel
init(Int64, Int64)
```

Constructs an empty accumulator with capacity hints derived from the literal at compile time.

`literalCapacity` is the total byte count of the static segments;
`interpolationCount` is the number of `\{...}` holes. Implementors
can use these to preallocate.

_Defined in `lang/std/text/format.ks`._

#### function `appendInterpolation`

```kestrel
public mutating func appendInterpolation[T](T, FormatOptions) where T: Formattable
```

Records one interpolation hole, eagerly formatted with `options`.

Calls `value.format(options)` immediately so the resulting
`String` is what gets stored — `value` is not retained past this
call. Default `options` matches `FormatOptions.default()`.

_Defined in `lang/std/text/format.ks`._

#### function `appendLiteral`

```kestrel
public mutating func appendLiteral(String)
```

Records one static literal segment.

Empty literals are dropped — they would force `build()` to do
extra work without changing the result. Non-empty literals are
appended verbatim with no copying beyond the `String`'s own COW.

_Defined in `lang/std/text/format.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> DefaultStringInterpolation
```

Returns a shallow copy with cloned `parts`.

`String` is COW so the part clone is itself shallow; mutating
either copy after this call does not affect the other.

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
init(stringLiteral: lang.str)
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

#### function `equals`

```kestrel
public func equals(FloatStyle) -> Bool
```

Returns true if both cases are the same variant.

All cases are payload-less, so equality is purely structural.

##### Examples

```
FloatStyle.Fixed.equals(.Fixed);       // true
FloatStyle.Fixed.equals(.Scientific);  // false
```

_Defined in `lang/std/text/format.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(FloatStyle) -> Bool
```

Pattern-match form of equality — delegates to `equals`.

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

#### function `equals`

```kestrel
public func equals(FormatOptions) -> Bool
```

Returns true if all fields are equal between the two options.

`width` and `precision` are not compared — they typically reflect
per-call overrides rather than logical identity. Compare them
explicitly if needed.

##### Examples

```
let a = FormatOptions();
let b = FormatOptions();
a.equals(b);  // true
var c = FormatOptions();
c.alternate = true;
a.equals(c);  // false
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
func format(FormatOptions) -> String
```

Returns this value rendered as a `String` under the supplied options.

Default arg uses `FormatOptions.default()` so unsuffixed calls
behave like the bare `"\{expr}"` interpolation form.

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
g.isAscii();     // true
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

#### function `chars`

```kestrel
public func chars() -> Array[Char]
```

Returns the constituent code points in scalar order.

Materializes a fresh `Array[Char]` on every call.

_Defined in `lang/std/text/char.ks`._

#### function `firstChar`

```kestrel
public func firstChar() -> Char?
```

Returns the first `Char` of the cluster.

Always `.Some` — every grapheme has at least one code point.
Useful as a cheap "what kind of grapheme is this?" check
(alphabetic, digit, emoji-base, …) without inspecting the full
cluster.

_Defined in `lang/std/text/char.ks`._

#### function `isAscii`

```kestrel
public func isAscii() -> Bool
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

#### function `equals`

```kestrel
public func equals(Grapheme) -> Bool
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
a.equals(b);  // true
```

_Defined in `lang/std/text/char.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> Grapheme
```

Returns a deep copy of this grapheme.

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

#### field `byteOffset`

```kestrel
public var byteOffset: Int64
```

The byte offset where the indexed grapheme begins.

_Defined in `lang/std/text/views.ks`._

### Implements `Equatable`

#### function `equals`

```kestrel
public func equals(GraphemeIndex) -> Bool
```

Returns true if the two indices point at the same byte offset.

_Defined in `lang/std/text/views.ks`._

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

#### initializer `From Pointer`

```kestrel
public init(ptr: lang.ptr[lang.i8], length: Int64)
```

Constructs a graphemes view from a raw pointer and a byte length.

Prefer `someString.graphemes` over calling this directly.

##### Safety

`ptr` must point to `length` valid UTF-8 bytes that remain live
for the view's lifetime.

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

#### field `count`

```kestrel
public var count: Int64 { get }
```

Number of grapheme clusters. **O(n)** — walks the entire string
through the UAX #29 segmenter. Cache the result if you need it
more than once; each access re-walks the string.

_Defined in `lang/std/text/views.ks`._

#### function `substring`

```kestrel
public func substring[I](I) -> String where I: GraphemesSubstringIndex
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
init(Int64, Int64)
```

Constructs an empty accumulator with capacity hints derived from the literal at compile time.

`literalCapacity` is the total byte count of the static segments;
`interpolationCount` is the number of `\{...}` holes. Implementors
can use these to preallocate.

_Defined in `lang/std/text/format.ks`._

#### function `appendInterpolation`

```kestrel
mutating func appendInterpolation[T](T, FormatOptions) where T: Formattable
```

Appends one formatted interpolation hole.

Receives the runtime `value`, the parsed `options` from the
trailing spec (or defaults if no spec was given), and a generic
constraint that the value is `Formattable`.

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

#### initializer `From Pointer`

```kestrel
public init(ptr: lang.ptr[lang.i8], length: Int64)
```

Constructs a lines view from a raw pointer and a byte length.

Prefer `someString.lines` over calling this directly.

##### Safety

`ptr` must point to `length` valid bytes that remain live for
the view's lifetime.

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

#### field `count`

```kestrel
public var count: Int64 { get }
```

Number of lines in the view. **O(n)** — walks the buffer
scanning for terminators. Cache the result if you need it more
than once.

_Defined in `lang/std/text/views.ks`._

#### function `substring`

```kestrel
public func substring[I](I) -> String where I: LinesSubstringIndex
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

#### function `equals`

```kestrel
public func equals(Sign) -> Bool
```

Returns true if both cases are the same variant.

Used by `Equatable` to lift case identity into a `Bool` for
composite comparisons (see `FormatOptions.equals`).

##### Examples

```
Sign.Always.equals(.Always);     // true
Sign.Negative.equals(.Always);   // false
```

_Defined in `lang/std/text/format.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(Sign) -> Bool
```

Pattern-match form of equality — delegates to `equals`.

##### Examples

```
Sign.Space.matches(.Space);  // true
```

_Defined in `lang/std/text/format.ks`._

## struct `SplitIterator`

```kestrel
public struct SplitIterator { /* private fields */ }
```

Iterator that yields the segments produced by splitting a string on a fixed-byte separator.

Produced by `String.split(separator:)`. Walks the source byte-by-byte
looking for an exact match of the separator's bytes (no UTF-8
awareness needed — the separator itself is UTF-8 so its byte
pattern can never align inside a multi-byte sequence). The empty
separator is treated specially: it splits per code point.

### Examples

```
var it = "a,b,c".split(separator: ",");
it.next();  // Some("a")
it.next();  // Some("b")
it.next();  // Some("c")
it.next();  // None
```

### Representation

A `(ptr, length, sepPtr, sepLen, index, done)` record. `done` flips
once the trailing remainder has been emitted.

### Memory Model

Value type. Borrows both the source and the separator buffers; do
not retain across mutations of either.

_Defined in `lang/std/text/string.ks`._

### Members

#### initializer `From Pointers`

```kestrel
public init(ptr: Pointer[UInt8], length: Int64, sepPtr: Pointer[UInt8], sepLen: Int64)
```

Constructs a split iterator from source and separator byte buffers.

Prefer `someString.split(separator:)` over calling this directly.

##### Safety

Both pointers must remain valid for `length` and `sepLen` bytes
respectively for the iterator's lifetime.

_Defined in `lang/std/text/string.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = String
```

The element type yielded by `next()` — always `String`.

_Defined in `lang/std/text/string.ks`._

#### function `next`

```kestrel
public mutating func next() -> String?
```

Returns the next segment, or `None` when the source is exhausted.

With a non-empty separator, returns each piece between matches
and finally the trailing remainder. With the empty separator,
returns one code point per call.

_Defined in `lang/std/text/string.ks`._

## struct `SplitWhereIterator`

```kestrel
public struct SplitWhereIterator { /* private fields */ }
```

Iterator that splits a string at every code point matching a predicate.

Produced by `String.split(matching:)`. Decodes the source one
`Char` at a time and breaks the string at each character for which
the predicate returns `true`; the matching character itself is not
included in any segment.

### Examples

```
var it = "a1b2c".split(matching: |c| c.isDigit());
it.next();  // Some("a")
it.next();  // Some("b")
it.next();  // Some("c")
it.next();  // None
```

### Representation

A `(ptr, length, predicate, index, done)` record.

_Defined in `lang/std/text/string.ks`._

### Members

#### initializer `From Predicate`

```kestrel
public init(pointer: Pointer[UInt8], length: Int64, matching: (Char) -> Bool)
```

Constructs a split-where iterator from a buffer pointer and a `Char` predicate.

Prefer `someString.split(matching:)` over calling this directly.

##### Safety

`ptr` must remain valid for `length` bytes for the iterator's
lifetime.

_Defined in `lang/std/text/string.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = String
```

The element type yielded by `next()` — always `String`.

_Defined in `lang/std/text/string.ks`._

#### function `next`

```kestrel
public mutating func next() -> String?
```

Returns the next segment, or `None` when the source is exhausted.

_Defined in `lang/std/text/string.ks`._

## struct `String`

```kestrel
public struct String { /* private fields */ }
```

A UTF-8 encoded, dynamically sized string with copy-on-write semantics.

`String` is the standard text type. The bytes are always valid
UTF-8 except after the unsafe internal `appendByte` path, which is
only intended for callers (such as substring helpers) that already
know the bytes are valid. Storage is shared between clones via an
`RcBox`; mutating a `String` whose storage is referenced elsewhere
triggers a copy. Three different views (`bytes`, `chars`,
`graphemes`) plus a `lines` view expose different units of
iteration over the same buffer.

### Examples

```
var s = "hello";
s.append(", world");
s.byteCount;            // 12
s.contains(substring: ",");  // true
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

A single `RcBox[StringStorage]` field. The storage record carries
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
public mutating func append(String)
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

#### function `appendByte`

```kestrel
public mutating func appendByte(UInt8)
```

Appends a raw byte to the buffer.

**Unsafe** with respect to the UTF-8 invariant — the caller
must ensure the resulting byte sequence is still valid UTF-8.
Used primarily by substring helpers that copy whole UTF-8
sequences in.

##### Safety

The string must remain valid UTF-8 after the append; do not
use this to inject continuation bytes into the middle of a
sequence.

_Defined in `lang/std/text/string.ks`._

#### function `appendChar`

```kestrel
public mutating func appendChar(Char)
```

Appends a single code point, encoding it as UTF-8.

Sizes the buffer for the encoded length (1–4 bytes) before
writing.

##### Examples

```
var s = "h";
s.appendChar('i');
s.appendChar('\u{1F600}');
s;  // "hi😀"
```

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

#### function `contains`

```kestrel
public func contains(String) -> Bool
```

Returns true if `substring` appears anywhere in this string.

Equivalent to `find(substring).isSome()`. The empty substring
always matches.

_Defined in `lang/std/text/string.ks`._

#### function `contains`

```kestrel
public func contains(matching: (Char) -> Bool) -> Bool
```

Returns true if any code point matches `predicate`.

_Defined in `lang/std/text/string.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

_Defined in `lang/std/text/string.ks`._

#### function `ends`

```kestrel
public func ends(with: String) -> Bool
```

Returns true if the string ends with `suffix`. O(suffix length).

##### Examples

```
"hello".ends(with: "lo");  // true
"hello".ends(with: "he");  // false
```

_Defined in `lang/std/text/string.ks`._

#### function `equalsCaseInsensitive`

```kestrel
public func equalsCaseInsensitive(String) -> Bool
```

Compares two strings for equality after Unicode case folding.

Walks both `chars` iterators in lockstep, folding each pair of
code points before comparing. Note: this is not normalization
aware — `é` (`U+00E9`) and `e\u{0301}` are still considered
different. Normalize both sides first if you need that.

##### Examples

```
"Hello".equalsCaseInsensitive("HELLO");  // true
"Hello".equalsCaseInsensitive("World");  // false
```

_Defined in `lang/std/text/string.ks`._

#### function `find`

```kestrel
public func find(String) -> Int64?
```

Returns the byte offset of the first occurrence of `substring`, or `None`.

Naïve byte-by-byte search; O(n·m) in the worst case where m is
the substring length. The empty substring matches at offset
`0`.

##### Examples

```
"hello".find("ll");      // Some(2)
"hello".find("xyz");     // None
"hello".find("");        // Some(0)
```

_Defined in `lang/std/text/string.ks`._

#### function `find`

```kestrel
public func find(matching: (Char) -> Bool) -> Int64?
```

Returns the byte offset of the first code point matching `predicate`, or `None`.

Decodes UTF-8 as it scans so the predicate sees real `Char`s
and the offset returned lands on a valid character boundary.

_Defined in `lang/std/text/string.ks`._

#### function `first`

```kestrel
public func first() -> Char?
```

Returns the first code point, or `None` if the string is empty. O(1) for the common case.

##### Examples

```
"hi".first();  // Some('h')
"".first();    // None
```

_Defined in `lang/std/text/string.ks`._

#### function `fromBytesUnchecked`

```kestrel
static func fromBytesUnchecked(Pointer[UInt8], Int64) -> String
```

Constructs a string by copying `count` bytes starting at `ptr`, without UTF-8 validation.

Internal helper used by split iterators and substring helpers
that already know the byte range falls on UTF-8 boundaries.

##### Safety

`ptr` must reference at least `count` valid UTF-8 bytes; the
range starting at `ptr` and ending at `ptr + count` must not
split a multi-byte sequence.

_Defined in `lang/std/text/string.ks`._

#### function `fromRawBytes`

```kestrel
static func fromRawBytes(lang.ptr[lang.i8], Int64) -> String
```

Constructs a string by copying `count` bytes from a raw `lang.ptr[lang.i8]`.

Internal helper for view-side code that holds raw pointers but
needs to materialize an owned `String`. No UTF-8 validation.

##### Safety

`rawPtr` must reference at least `count` valid UTF-8 bytes; the
range must not split a multi-byte sequence.

_Defined in `lang/std/text/string.ks`._

#### function `fromUtf8`

```kestrel
public static func fromUtf8(Slice[UInt8]) -> String?
```

Constructs a string by copying validated UTF-8 bytes from `bytes`,
returning `.None` if the slice is not valid UTF-8.

Walks the slice end-to-end with `decodeUtf8`; any malformed,
truncated, or overlong sequence produces `.None`. The empty slice
is valid and yields the empty string. On success the bytes are
copied into a fresh heap allocation, so the returned `String`
owns its storage independently of `bytes`.

##### Errors

Panics with `"String allocation failed"` if the system allocator
returns null. Returns `.None` only for invalid UTF-8 — the
allocation case is unrecoverable.

##### Examples

```
String.fromUtf8(bytes: "héllo".bytes.asSlice());  // Some("héllo")
String.fromUtf8(bytes: badSlice);                 // None
```

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

#### function `last`

```kestrel
public func last() -> Char?
```

Returns the last code point, or `None` if the string is empty. O(n).

Has to scan from the start to identify the final UTF-8 sequence
— there is no way to read backwards through variable-width
UTF-8 without a separate index.

##### Examples

```
"hi".last();  // Some('i')
"".last();    // None
```

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

Locale-independent. Handles multi-character expansions
(rare in lowercasing). Implemented as `self = self.lowercased()`,
so a transient new buffer is allocated.

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

#### function `lowercased`

```kestrel
public func lowercased() -> String
```

Returns the lowercase form using full Unicode case mapping.

Two fast paths: an all-ASCII string with no uppercase letters
is returned cloned (no allocation beyond the COW share); an
all-ASCII string with uppercase letters routes to
`lowercasedAscii`. The slow path uses the Unicode tables and
honours multi-char expansions.

##### Examples

```
"Hello".lowercased();      // "hello"
"\u{0130}".lowercased();   // "i\u{0307}" (Turkish dotted I expansion)
```

_Defined in `lang/std/text/string.ks`._

#### function `lowercasedAscii`

```kestrel
public func lowercasedAscii() -> String
```

Returns a copy with ASCII letters lowercased; non-ASCII bytes pass through unchanged.

_Defined in `lang/std/text/string.ks`._

#### function `pad`

```kestrel
public func pad(leading: Int64, with: Char) -> String
```

Returns the string padded at the start with `char` so the total *code-point* count is `length`.

If the string is already at least `length` code points long,
returns a clone. Compare with `pad(end:with:)` for trailing
padding.

##### Examples

```
"42".pad(start: 5, with: '0');  // "00042"
```

_Defined in `lang/std/text/string.ks`._

#### function `pad`

```kestrel
public func pad(trailing: Int64, with: Char) -> String
```

Returns the string padded at the end with `char` so the total *code-point* count is `length`.

##### Examples

```
"42".pad(end: 5, with: '.');  // "42..."
```

_Defined in `lang/std/text/string.ks`._

#### function `repeated`

```kestrel
public func repeated(Int64) -> String
```

Returns this string concatenated with itself `count` times.

Non-positive `count` returns the empty string. Sizes the
result buffer for the exact final length to avoid growth.

##### Examples

```
"ab".repeated(count: 3);  // "ababab"
"ab".repeated(count: 0);  // ""
```

_Defined in `lang/std/text/string.ks`._

#### function `replace`

```kestrel
public mutating func replace(String, with: String)
```

Replaces every occurrence of `pattern` with `replacement`, in place.

Allocates a fresh string under the hood; the in-place form is
for ergonomics, not buffer reuse.

_Defined in `lang/std/text/string.ks`._

#### function `replaced`

```kestrel
public func replaced(String, with: String) -> String
```

Returns a copy with every occurrence of `pattern` replaced by `replacement`.

Empty `pattern` is a no-op (returns a clone). Searches greedily
from the left and skips past each replacement so substituted
text is not re-matched.

##### Examples

```
"hello world".replaced("o", with: "0");      // "hell0 w0rld"
"abcabc".replaced("ab", with: "ABCD");       // "ABCDcABCDc"
```

_Defined in `lang/std/text/string.ks`._

#### function `reverseFind`

```kestrel
public func reverseFind(String) -> Int64?
```

Returns the byte offset of the *last* occurrence of `substring`, or `None`.

Scans from the right but with the same naïve byte comparison
as `find`. The empty substring matches at offset `byteCount`.

##### Examples

```
"abcabc".reverseFind("abc");  // Some(3)
"abcabc".reverseFind("");     // Some(6)
```

_Defined in `lang/std/text/string.ks`._

#### function `split`

```kestrel
public func split(String) -> SplitIterator
```

Returns an iterator that splits this string on `separator` (byte-exact).

The empty separator is special-cased to split per code point.
See `SplitIterator` for the iteration shape.

##### Examples

```
var parts = Array[String]();
for p in "a,b,c".split(separator: ",") { parts.append(p); }
parts.count;  // 3
```

_Defined in `lang/std/text/string.ks`._

#### function `split`

```kestrel
public func split(matching: (Char) -> Bool) -> SplitWhereIterator
```

Returns an iterator that splits at every code point matching `predicate`.

The matching characters are not included in any segment.

##### Examples

```
var parts = Array[String]();
for p in "a 1 b 2 c".split(matching: |c| c.isDigit() or c.isWhitespace()) {
    if p.isEmpty == false { parts.append(p); }
}
// parts: ["a", "b", "c"]
```

_Defined in `lang/std/text/string.ks`._

#### function `starts`

```kestrel
public func starts(with: String) -> Bool
```

Returns true if the string begins with `prefix`. O(prefix length).

##### Examples

```
"hello".starts(with: "he");   // true
"hello".starts(with: "lo");   // false
```

_Defined in `lang/std/text/string.ks`._

#### function `substring`

```kestrel
public func substring[I](I) -> String where I: CharsSubstringIndex
```

Returns the substring covering code points in `range`. Defaults
to **chars** semantics — use `self.graphemes.substring(range)`
for grapheme-cluster slicing or `self.bytes.substring(range)`
(or `substringBytes`) for raw byte ranges. Accepts any range
type that conforms to `std.text.CharsSubstringIndex`
(`Range[Int64]` and `ClosedRange[Int64]` today).

Equivalent to `self.chars.substring(range)`. Panics on
out-of-bounds.

##### Examples

```
"héllo".substring(0..<4);   // "héll"
"héllo".substring(0..=3);   // "héll"
```

_Defined in `lang/std/text/string.ks`._

#### function `substringBytes`

```kestrel
public func substringBytes(from: Int64, to: Int64) -> String
```

Returns the substring spanning byte indices `[start, end)`.

Out-of-range, inverted, or empty ranges return the empty
string rather than panicking. The caller is responsible for
ensuring the bounds fall on UTF-8 boundaries — use
`s.bytes(checked: range)` for a validated alternative.

##### Examples

```
"hello".substringBytes(from: 1, to: 4);   // "ell"
"hello".substringBytes(from: 4, to: 1);   // ""    (inverted)
"hello".substringBytes(from: 0, to: 99);  // ""    (out of range)
```

_Defined in `lang/std/text/string.ks`._

#### function `titlecased`

```kestrel
public func titlecased() -> String
```

Returns the titlecase form using full Unicode case mapping.

Word boundaries are detected by `Char.isWhitespace`; the first
non-space character of each run is titlecased and the rest
lowercased. This is a coarse model — it doesn't handle
hyphenated names or apostrophe-internal capitals — but works
for plain whitespace-separated text.

##### Examples

```
"hello world".titlecased();  // "Hello World"
"FOO BAR".titlecased();      // "Foo Bar"
```

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

Recognises space, tab, LF, CR — same set as `Char.isWhitespace`
minus form feed (which `Char.isWhitespace` accepts but the
trim helpers do not). For Unicode-aware trimming, use the
`(matching:)` overloads with a custom predicate. Non-mutating
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
public mutating func trim(matching: (Char) -> Bool)
```

Removes leading and trailing code points matching `predicate`, in place.

##### Examples

```
var s = "***hi***";
s.trim(matching: |c| c == '*');
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
public mutating func trimEnd(matching: (Char) -> Bool)
```

Removes trailing code points matching `predicate`, in place.

Implemented by a forward scan that tracks the byte offset of
the last non-matching character — UTF-8 is awkward to walk
backwards without a side index.

_Defined in `lang/std/text/string.ks`._

#### function `trimStart`

```kestrel
public mutating func trimStart()
```

Removes leading ASCII whitespace in place.

_Defined in `lang/std/text/string.ks`._

#### function `trimStart`

```kestrel
public mutating func trimStart(matching: (Char) -> Bool)
```

Removes leading code points matching `predicate`, in place.

_Defined in `lang/std/text/string.ks`._

#### function `trimmed`

```kestrel
public func trimmed() -> String
```

Returns a copy with leading and trailing ASCII whitespace removed.

Non-mutating mirror of `trim()`.

_Defined in `lang/std/text/string.ks`._

#### function `trimmed`

```kestrel
public func trimmed(matching: (Char) -> Bool) -> String
```

Returns a copy with leading and trailing code points matching `predicate` removed.

_Defined in `lang/std/text/string.ks`._

#### function `trimmedEnd`

```kestrel
public func trimmedEnd() -> String
```

Returns a copy with trailing ASCII whitespace removed.

_Defined in `lang/std/text/string.ks`._

#### function `trimmedEnd`

```kestrel
public func trimmedEnd(matching: (Char) -> Bool) -> String
```

Returns a copy with trailing code points matching `predicate` removed.

_Defined in `lang/std/text/string.ks`._

#### function `trimmedStart`

```kestrel
public func trimmedStart() -> String
```

Returns a copy with leading ASCII whitespace removed.

_Defined in `lang/std/text/string.ks`._

#### function `trimmedStart`

```kestrel
public func trimmedStart(matching: (Char) -> Bool) -> String
```

Returns a copy with leading code points matching `predicate` removed.

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

#### function `uppercased`

```kestrel
public func uppercased() -> String
```

Returns the uppercase form using full Unicode case mapping.

Symmetric to `lowercased`; the same ASCII fast paths apply.
Multi-char expansions (e.g. `ß` → `SS`) are honoured.

##### Examples

```
"hello".uppercased();      // "HELLO"
"stra\u{00DF}e".uppercased();  // "STRASSE" (ß expands to SS)
```

_Defined in `lang/std/text/string.ks`._

#### function `uppercasedAscii`

```kestrel
public func uppercasedAscii() -> String
```

Returns a copy with ASCII letters uppercased; non-ASCII bytes pass through unchanged.

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
type TargetIterator = StringIterator
```

The iterator type returned by `iter()`.

_Defined in `lang/std/text/string.ks`._

#### function `iter`

```kestrel
public func iter() -> StringIterator
```

Returns a `StringIterator` over the code points starting at byte 0.

Required by `Iterable`. Each call returns a fresh iterator;
the string itself is reusable.

_Defined in `lang/std/text/string.ks`._

### Implements `Equatable`

#### function `equals`

```kestrel
public func equals(String) -> Bool
```

Returns true if both strings have the same byte sequence.

Pure byte-wise equality — not normalization-aware. For
case-insensitive comparison, see `equalsCaseInsensitive`.

##### Examples

```
"abc".equals("abc");  // true
"abc".equals("ABC");  // false
```

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

O(1). Mutation triggers `makeUnique` which performs a deep
copy.

_Defined in `lang/std/text/string.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(FormatOptions) -> String
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
"test".format(options: opts);   // "test      "
opts.alignment = .Right;
"test".format(options: opts);   // "      test"
opts.alignment = .Center;
"test".format(options: opts);   // "   test   "
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
init(stringLiteral: lang.str)
```

Builds an instance from a string literal.

_Defined in `lang/std/core/literals.ks`._

### Implements `Hash`

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

## struct `StringIterator`

```kestrel
public struct StringIterator { /* private fields */ }
```

Single-pass forward iterator over the Unicode code points of a `String`.

Produced by `String.iter()`. Decodes one UTF-8 character at a time,
advancing the cursor by the encoded byte length. On invalid UTF-8
the iterator returns `None` and skips one byte so the next call
can make progress; this differs from `CharsIterator` which yields
`U+FFFD` on bad input.

### Examples

```
var it = "hi".iter();
it.next();  // Some('h')
it.next();  // Some('i')
it.next();  // None
```

### Representation

A `(ptr, length, index)` triple. `index` advances in variable-width
steps according to the UTF-8 encoding.

### Memory Model

Value type. The pointer aliases the source string's storage; do not
retain across mutations of the source `String`.

_Defined in `lang/std/text/string.ks`._

### Members

#### initializer `From Pointer`

```kestrel
public init(ptr: Pointer[UInt8], length: Int64)
```

Constructs a string iterator from a buffer pointer and total byte count.

Prefer `someString.iter()` over calling this directly.

##### Safety

`ptr` must point to `length` valid UTF-8 bytes that remain live
for the iterator's lifetime.

_Defined in `lang/std/text/string.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = Char
```

The element type yielded by `next()` — always `Char`.

_Defined in `lang/std/text/string.ks`._

#### function `next`

```kestrel
public mutating func next() -> Char?
```

Returns the next code point, or `None` when the buffer is exhausted.

On invalid UTF-8 the iterator returns `None` and advances by one
byte to guarantee forward progress on subsequent calls.

_Defined in `lang/std/text/string.ks`._

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
// encodeUtf8(c: 'a',         ptr: buf, at: 0);  // 1
// encodeUtf8(c: '\u{1F600}', ptr: buf, at: 0);  // 4
```

_Defined in `lang/std/text/char.ks`._

