// Character types - Unicode code points and bytes

module std.text

import std.core.(Equatable, Comparable, Ordering, Bool, Matchable, ExpressibleByCharLiteral, Hashable, Hasher, RangeMatchable)
import std.numeric.(Int64, UInt8, UInt32)
import std.result.(Optional)
import std.collections.(Array)
import std.memory.(ArraySlice, Pointer)
import std.text.(String, StringBuilder, Formattable, FormatOptions)
import std.text.unicode as unicode

// ============================================================================
// TYPE ALIASES
// ============================================================================
// CHAR
// ============================================================================

/// A single Unicode scalar value (code point in `0..=0x10FFFF`, surrogates excluded).
///
/// `Char` is the unit yielded by `String.chars` / `CharsView`; iterating
/// graphemes (`String.graphemes`) instead returns `Grapheme` clusters
/// that may comprise multiple `Char`s. The character-literal syntax
/// constructs values directly: `'a'`, `'\n'`, `'\u{1F600}'`. For the
/// raw byte representation, see `utf8Length()` and the free
/// `encodeUtf8` / `decodeUtf8` functions.
///
/// # Examples
///
/// ```
/// let a: Char = 'a';
/// a.isAsciiLetter;    // true
/// a.utf8Length();      // 1
/// let smile: Char = '\u{1F600}';
/// smile.utf8Length();  // 4
/// ```
///
/// # Representation
///
/// A single `UInt32` holding the scalar value. Comparison and hashing
/// operate on that integer directly.
public struct Char: Equatable, Comparable, Matchable, ExpressibleByCharLiteral, Hashable, RangeMatchable {
    private var _value: UInt32

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// @name From Value
    /// Wraps a raw `UInt32` scalar value as a `Char`.
    ///
    /// No range or surrogate validation is performed; pass values you
    /// already know are valid Unicode scalars. Prefer the literal syntax
    /// (`'a'`, `'\u{...}'`) when the value is known at compile time.
    ///
    /// # Examples
    ///
    /// ```
    /// let c = Char(value: UInt32(intLiteral: 0x41));
    /// c == 'A';  // true
    /// ```
    public init(value: UInt32) {
        self._value = value;
    }

    /// @name Validated
    /// Returns a `Char` if the value is a valid Unicode scalar, `.None` otherwise.
    /// Rejects values > U+10FFFF and the surrogate range U+D800..U+DFFF.
    public static func validated(value: UInt32) -> Char? {
        if value > UInt32(intLiteral: 0x10FFFF) { return .None }
        if value >= UInt32(intLiteral: 0xD800) and value <= UInt32(intLiteral: 0xDFFF) { return .None }
        .Some(Char(value))
    }

    /// @name Char Literal
    /// Compiler-emitted constructor for character literals.
    ///
    /// Called when you write `'a'`, `'\n'`, `'\u{1F600}'`. Not intended
    /// for direct use — `Char(value:)` is the user-facing constructor.
    ///
    /// # Examples
    ///
    /// ```
    /// let c: Char = 'a';  // lowers to Char(charLiteral: ...)
    /// ```
    public init(charLiteral value: lang.i32) {
        self._value = UInt32(raw: value);
    }

    // ========================================================================
    // VALUE ACCESS
    // ========================================================================

    /// Returns the raw Unicode scalar as a `UInt32`.
    ///
    /// Useful for arithmetic on code points (e.g. `digitValue`'s offset
    /// trick) or interop with APIs that take a numeric code point.
    ///
    /// # Examples
    ///
    /// ```
    /// 'A'.value();  // 65
    /// '\u{1F600}'.value();  // 128512
    /// ```
    public func value() -> UInt32 { self._value }

    // ========================================================================
    // CHARACTER CLASSIFICATION
    // ========================================================================

    /// Returns true if the scalar is in the ASCII range (`< 0x80`).
    ///
    /// Cheap byte-range test; does not consult Unicode tables. For
    /// "alphabetic by Unicode" use `unicode.toLowercase` round-tripping
    /// or the property tables directly.
    ///
    /// # Examples
    ///
    /// ```
    /// 'A'.isAscii;          // true
    /// '\u{00E9}'.isAscii;   // false (é)
    /// ```
    public var isAscii: Bool {
        self < '\u{80}'
    }

    /// Returns true for ASCII letters `A`–`Z` / `a`–`z`.
    ///
    /// **ASCII-only.** Non-ASCII letters (e.g. `é`, `Ω`, `日`) return
    /// `false` even though they are letters in Unicode. For the full
    /// Unicode test, use the property tables in `std.text.unicode`.
    ///
    /// # Examples
    ///
    /// ```
    /// 'A'.isAsciiLetter;         // true
    /// '\u{00E9}'.isAsciiLetter;  // false (é — non-ASCII)
    /// '7'.isAsciiLetter;         // false
    /// ```
    public var isAsciiLetter: Bool {
        (self >= 'A' and self <= 'Z') or (self >= 'a' and self <= 'z')
    }

    /// Returns true for the ASCII digits `0`–`9`.
    ///
    /// **ASCII-only.** Other Unicode digit categories (Arabic-Indic,
    /// Devanagari, etc.) return `false`. See `digitValue()` for parsing
    /// to numeric value.
    ///
    /// # Examples
    ///
    /// ```
    /// '7'.isAsciiDigit;   // true
    /// 'a'.isAsciiDigit;   // false
    /// ```
    public var isAsciiDigit: Bool {
        self >= '0' and self <= '9'
    }

    /// Returns true for ASCII letters and ASCII digits.
    ///
    /// Composition of `isAsciiLetter` and `isAsciiDigit`; same ASCII-only
    /// caveats apply.
    ///
    /// # Examples
    ///
    /// ```
    /// 'a'.isAsciiAlphanumeric;  // true
    /// '7'.isAsciiAlphanumeric;  // true
    /// '_'.isAsciiAlphanumeric;  // false
    /// ```
    public var isAsciiAlphanumeric: Bool {
        self.isAsciiLetter or self.isAsciiDigit
    }

    /// Returns true for the common ASCII whitespace set: space, tab, LF, CR, form feed.
    ///
    /// Does not include Unicode whitespace such as `U+00A0` (no-break
    /// space) or `U+2028` (line separator). For Unicode-aware
    /// whitespace, consult the property tables.
    ///
    /// # Examples
    ///
    /// ```
    /// ' '.isWhitespace;    // true
    /// '\t'.isWhitespace;   // true
    /// '\n'.isWhitespace;   // true
    /// 'a'.isWhitespace;    // false
    /// ```
    public var isWhitespace: Bool {
        self == ' ' or self == '\t' or self == '\n' or self == '\r' or self == '\x0C'
    }

    /// Returns true for the C0 controls (`< U+0020`) and DEL (`U+007F`).
    ///
    /// Does not include the C1 controls (`U+0080`–`U+009F`); add a
    /// dedicated test if you need them.
    ///
    /// # Examples
    ///
    /// ```
    /// '\n'.isControl;     // true
    /// '\x7F'.isControl;   // true
    /// 'a'.isControl;      // false
    /// ```
    public var isControl: Bool {
        self < ' ' or self == '\x7F'
    }

    /// Returns true for ASCII uppercase letters `A`–`Z`.
    ///
    /// **ASCII-only.** Use `unicode.toUppercase` round-tripping for
    /// general Unicode case tests.
    ///
    /// # Examples
    ///
    /// ```
    /// 'A'.isAsciiUppercase;         // true
    /// 'a'.isAsciiUppercase;         // false
    /// '\u{00C9}'.isAsciiUppercase;  // false (É — non-ASCII)
    /// ```
    public var isAsciiUppercase: Bool {
        self >= 'A' and self <= 'Z'
    }

    /// Returns true for ASCII lowercase letters `a`–`z`.
    ///
    /// **ASCII-only.** Use `unicode.toLowercase` round-tripping for
    /// general Unicode case tests.
    ///
    /// # Examples
    ///
    /// ```
    /// 'a'.isAsciiLowercase;   // true
    /// 'A'.isAsciiLowercase;   // false
    /// ```
    public var isAsciiLowercase: Bool {
        self >= 'a' and self <= 'z'
    }

    // ========================================================================
    // CASE CONVERSION (Unicode)
    // ========================================================================

    /// Returns the uppercase form, using full Unicode case-mapping tables.
    ///
    /// For characters whose uppercase form is multiple `Char`s (e.g.
    /// German `ß` → `SS`), this returns only the first `Char`. Use
    /// `hasUppercaseExpansion()` plus `uppercaseExpansion()` to handle
    /// those cases correctly. Locale-independent — does not perform
    /// Turkish / Azeri tailoring.
    ///
    /// # Examples
    ///
    /// ```
    /// 'a'.uppercased();         // 'A'
    /// '\u{00DF}'.uppercased();  // 'S' (first char of "SS"; see hasUppercaseExpansion)
    /// ```
    public func uppercased() -> Char {
        unicode.toUppercase(self)
    }

    /// Returns the lowercase form, using full Unicode case-mapping tables.
    ///
    /// Locale-independent. For characters with multi-char lowercase
    /// expansions, see `lowercaseExpansion()`.
    ///
    /// # Examples
    ///
    /// ```
    /// 'A'.lowercased();         // 'a'
    /// '\u{0130}'.lowercased();  // 'i' (Turkish dotted I — first char only)
    /// ```
    public func lowercased() -> Char {
        unicode.toLowercase(self)
    }

    /// Returns the titlecase form, using full Unicode case-mapping tables.
    ///
    /// Titlecase differs from uppercase for some characters — e.g.
    /// ligatures like `ǳ` titlecase to `ǲ` (capital plus small) rather
    /// than `Ǳ` (full uppercase).
    ///
    /// # Examples
    ///
    /// ```
    /// 'a'.titlecased();   // 'A'
    /// ```
    public func titlecased() -> Char {
        unicode.toTitlecase(self)
    }

    /// Returns true if the uppercase form is multi-char (e.g. `ß` → `SS`).
    ///
    /// When `true`, prefer `uppercaseExpansion()` over `uppercased()`
    /// to avoid silently dropping characters.
    ///
    /// # Examples
    ///
    /// ```
    /// '\u{00DF}'.hasUppercaseExpansion();  // true (ß)
    /// 'a'.hasUppercaseExpansion();         // false
    /// ```
    public func hasUppercaseExpansion() -> Bool {
        unicode.hasUppercaseExpansion(self)
    }

    /// Returns the multi-char uppercase form as a `String`.
    ///
    /// For characters without an expansion this returns the empty
    /// string; use `hasUppercaseExpansion()` first to distinguish.
    ///
    /// # Examples
    ///
    /// ```
    /// '\u{00DF}'.uppercaseExpansion();  // "SS"
    /// 'a'.uppercaseExpansion();         // ""
    /// ```
    public func uppercaseExpansion() -> String {
        unicode.uppercaseExpansion(self)
    }

    /// Returns true if the lowercase form is multi-char.
    ///
    /// Rare in practice but exists for full Unicode round-tripping.
    public func hasLowercaseExpansion() -> Bool {
        unicode.hasLowercaseExpansion(self)
    }

    /// Returns the multi-char lowercase form as a `String`.
    ///
    /// Empty string if no expansion exists.
    public func lowercaseExpansion() -> String {
        unicode.lowercaseExpansion(self)
    }

    /// Returns true if the titlecase form is multi-char.
    public func hasTitlecaseExpansion() -> Bool {
        unicode.hasTitlecaseExpansion(self)
    }

    /// Returns the multi-char titlecase form as a `String`.
    ///
    /// Empty string if no expansion exists.
    public func titlecaseExpansion() -> String {
        unicode.titlecaseExpansion(self)
    }

    // ========================================================================
    // UTF-8 ENCODING
    // ========================================================================

    /// Returns how many UTF-8 bytes are required to encode this character (1–4).
    ///
    /// Constant time — branches on the scalar value alone. Use this to
    /// size buffers before calling `encodeUtf8`.
    ///
    /// # Examples
    ///
    /// ```
    /// 'a'.utf8Length();          // 1
    /// '\u{00E9}'.utf8Length();   // 2 (é)
    /// '\u{20AC}'.utf8Length();   // 3 (€)
    /// '\u{1F600}'.utf8Length();  // 4 (😀)
    /// ```
    public func utf8Length() -> Int64 {
        let v = self._value;
        if v < 128 { 1 }
        else if v < 2048 { 2 }
        else if v < 65536 { 3 }
        else { 4 }
    }

    // ========================================================================
    // DIGIT CONVERSION
    // ========================================================================

    /// Returns the numeric value `0`–`9` for ASCII digits, otherwise `None`.
    ///
    /// Inverse of `fromDigit`. Non-ASCII digit characters return `None`
    /// — match `isAsciiDigit` semantics.
    ///
    /// # Examples
    ///
    /// ```
    /// '7'.digitValue();  // Some(7)
    /// 'a'.digitValue();  // None
    /// ```
    public func digitValue() -> UInt32? {
        if self.isAsciiDigit {
            let zero: Char = '0';
            .Some(self.value() - zero.value())
        } else {
            .None
        }
    }

    /// Returns the ASCII digit `Char` for a numeric value `0`–`9`, otherwise `None`.
    ///
    /// Inverse of `digitValue`. Values outside `0..=9` return `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// Char.fromDigit(7);   // Some('7')
    /// Char.fromDigit(12);  // None
    /// ```
    public static func fromDigit(d: UInt32) -> Char? {
        if d <= 9 {
            let zero: Char = '0';
            .Some(Char(d + zero.value()))
        } else {
            .None
        }
    }

    // ========================================================================
    // PROTOCOL CONFORMANCES
    // ========================================================================

    /// Returns true if both characters are the same Unicode scalar.
    ///
    /// Pure scalar-value equality — no case folding, no normalization.
    ///
    /// # Examples
    ///
    /// ```
    /// 'a'.isEqual(to: 'a');  // true
    /// 'a'.isEqual(to: 'A');  // false
    /// ```
    public func isEqual(to other: Char) -> Bool {
        self._value == other._value
    }

    /// Pattern-match form of equality — delegates to `isEqual`.
    public func matches(other: Char) -> Bool {
        self._value == other._value
    }

    /// Compares two characters by scalar value.
    ///
    /// Yields code-point order, which agrees with byte order in UTF-8
    /// (UTF-8 is order-preserving). Not the same as locale-aware
    /// collation.
    ///
    /// # Examples
    ///
    /// ```
    /// 'a'.compare('b');  // Less
    /// 'b'.compare('a');  // Greater
    /// 'a'.compare('a');  // Equal
    /// ```
    public func compare(other: Char) -> Ordering {
        self._value.compare(other._value)
    }

    /// Hashes this character by writing its 4-byte scalar value to the hasher.
    ///
    /// Uses native byte order — fine for in-process hash maps; do not
    /// use the result for content-addressed storage.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        let val = self._value;
        hasher.write(ArraySlice(pointer: Pointer(to: val).asRaw().cast[UInt8](), count: 4))
    }

    /// Returns true if `self >= bound`. Used by `RangeMatchable` for `case 'a'...'z'`.
    public func isAtLeast(bound: Char) -> Bool {
        self.compare(bound) != Ordering.Less
    }

    /// Returns true if `self <= bound`. Used by `RangeMatchable` for `case 'a'...'z'`.
    public func isAtMost(bound: Char) -> Bool {
        self.compare(bound) != Ordering.Greater
    }

    /// Returns true if `self < bound`. Used by `RangeMatchable` for half-open patterns.
    public func isBelow(bound: Char) -> Bool {
        self.compare(bound) == Ordering.Less
    }

    /// Converts this code point to an owned `String`.
    public func toString() -> String {
        var s = String();
        s.appendChar(self);
        s
    }
}

// -- Formattable conformance -------------------------------------------------

extend Char: Formattable {
    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        writer.appendChar(self)
    }
}

// ============================================================================
// GRAPHEME
// ============================================================================

/// An extended grapheme cluster — what users perceive as a single character.
///
/// A grapheme may comprise one `Char` (e.g. `'a'`) or several
/// (combining marks, regional-indicator country flags, ZWJ-joined emoji
/// sequences). `String.graphemes` is the canonical producer; iteration
/// uses UAX #29 segmentation. Treat `Grapheme` as the right unit for
/// any user-visible operation (cursor movement, selection, truncation
/// for display).
///
/// # Examples
///
/// ```
/// let g = Grapheme(char: 'a');
/// g.charCount();   // 1
/// g.isAscii;     // true
/// g.utf8Length();  // 1
/// ```
///
/// # Representation
///
/// An `Array[Char]` of the constituent code points in scalar order.
public struct Grapheme: Equatable, Cloneable {
    // Single-char clusters store just `_first` and leave `_rest = .None`,
    // which is the common path for ASCII text. Multi-char clusters carry
    // the trailing code points in `_rest` (does not include `_first`).
    private var _first: Char
    private var _rest: Array[Char]?

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// @name Single Char
    /// Constructs a one-`Char` grapheme.
    ///
    /// Allocation-free — the common path for ASCII iteration through
    /// `GraphemesView`.
    ///
    /// # Examples
    ///
    /// ```
    /// let g = Grapheme(char: 'a');
    /// g.charCount();  // 1
    /// ```
    public init(char char: Char) {
        self._first = char;
        self._rest = .None;
    }

    /// @name From Chars
    /// Constructs a grapheme from a sequence of `Char`s.
    ///
    /// The caller is responsible for the chars actually forming a
    /// single UAX #29 cluster — the constructor does not segment or
    /// validate. `GraphemesIterator` is the canonical producer of valid
    /// clusters. Single-char input avoids allocating; multi-char input
    /// keeps the trailing code points in a separate array.
    ///
    /// # Examples
    ///
    /// ```
    /// var chars = Array[Char]();
    /// chars.append('e');
    /// chars.append('\u{0301}');  // combining acute
    /// let g = Grapheme(chars: chars);
    /// g.charCount();  // 2
    /// ```
    public init(chars chars: Array[Char]) {
        let n = chars.count;
        if n == 0 {
            self._first = Char(0);
            self._rest = .None
        } else if n == 1 {
            self._first = chars(unchecked: 0);
            self._rest = .None
        } else {
            self._first = chars(unchecked: 0);
            var rest = Array[Char]();
            var i: Int64 = 1;
            while i < n {
                rest.append(chars(unchecked: i));
                i = i + 1
            }
            self._rest = .Some(rest)
        }
    }

    /// Returns a deep copy of this grapheme.
    public func clone() -> Grapheme {
        match self._rest {
            .None => Grapheme(char: self._first),
            .Some(r) => {
                var g = Grapheme(char: self._first);
                g._rest = .Some(r.clone());
                g
            }
        }
    }

    // ========================================================================
    // ACCESSORS
    // ========================================================================

    /// The constituent code points in scalar order.
    ///
    /// Materializes a fresh `Array[Char]` on every access.
    public var chars: Array[Char] {
        var arr = Array[Char]();
        arr.append(self._first);
        if let .Some(r) = self._rest {
            for c in r {
                arr.append(c)
            }
        }
        arr
    }

    /// Returns the number of `Char`s in this cluster — `1` for plain ASCII, more for combining sequences and ZWJ-joined emoji.
    public func charCount() -> Int64 {
        if let .Some(r) = self._rest {
            1 + r.count
        } else {
            1
        }
    }

    /// Returns the first `Char` of the cluster.
    ///
    /// The first code point of this grapheme cluster.
    public var firstChar: Char { self._first }

    /// Returns true iff the cluster is exactly one ASCII `Char`.
    ///
    /// A single-`Char` non-ASCII grapheme (e.g. `é` as the precomposed
    /// `U+00E9`) returns `false`. Multi-`Char` clusters always return
    /// `false` even if every component is ASCII.
    public var isAscii: Bool {
        match self._rest {
            .None => self._first.isAscii,
            .Some(_) => false
        }
    }

    /// Returns the total UTF-8 byte length of all constituent `Char`s.
    ///
    /// Sum of each `Char.utf8Length()`. Use this to size a buffer
    /// before re-encoding the cluster.
    public func utf8Length() -> Int64 {
        var len = self._first.utf8Length();
        if let .Some(r) = self._rest {
            let n = r.count;
            var i: Int64 = 0;
            while i < n {
                len = len + r(unchecked: i).utf8Length();
                i = i + 1
            }
        }
        len
    }

    // ========================================================================
    // PROTOCOL CONFORMANCES
    // ========================================================================

    /// Returns true if the two graphemes are the same length and every `Char` is equal pairwise.
    ///
    /// **Not** Unicode normalization-aware: precomposed `é` (`U+00E9`)
    /// and decomposed `e` + `U+0301` are not equal under this check
    /// even though they represent the same user-perceived character.
    /// Normalize both sides first if you need that.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = Grapheme(char: 'a');
    /// let b = Grapheme(char: 'a');
    /// a.isEqual(to: b);  // true
    /// ```
    public func isEqual(to other: Grapheme) -> Bool {
        if self._first.isEqual(to: other._first) == false {
            return false
        }
        match (self._rest, other._rest) {
            (.None, .None) => true,
            (.Some(a), .Some(b)) => {
                let an = a.count;
                if an != b.count {
                    return false
                }
                var i: Int64 = 0;
                while i < an {
                    if a(unchecked: i).isEqual(to: b(unchecked: i)) == false {
                        return false
                    }
                    i = i + 1
                }
                true
            },
            _ => false
        }
    }
}

// -- Grapheme: Comparable, Hashable, Formattable ---------------------------------

extend Grapheme: Comparable {
    public func compare(other: Grapheme) -> Ordering {
        let myChars = self.chars;
        let otherChars = other.chars;
        let minLen = if myChars.count < otherChars.count { myChars.count } else { otherChars.count };
        for (i, c) in myChars.iter().enumerate() {
            if i >= minLen { break }
            let cmp = c.compare(otherChars(unchecked: i));
            if cmp != .Equal { return cmp }
        }
        myChars.count.compare(otherChars.count)
    }
}

extend Grapheme: Hashable {
    public func hash[H](mutating into hasher: H) where H: Hasher {
        for c in self.chars {
            c.hash(into: hasher)
        }
    }
}

extend Grapheme: Formattable {
    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        for c in self.chars {
            writer.appendChar(c)
        }
    }
}

// ============================================================================
// ASCII CONSTANTS
// ============================================================================


// ============================================================================
// UTF-8 DECODING RESULT
// ============================================================================

/// The output of decoding one UTF-8 character from a byte buffer.
///
/// Carries both the decoded `Char` and the number of bytes consumed,
/// so the caller can advance their cursor without re-running
/// `utf8Length()`. Returned as `Some` from `decodeUtf8`; `None`
/// indicates an invalid or truncated sequence.
///
/// # Examples
///
/// ```
/// let r = Utf8DecodeResult(char: 'a', bytesConsumed: 1);
/// r.char;           // 'a'
/// r.bytesConsumed;  // 1
/// ```
///
/// # Representation
///
/// A plain pair `(char: Char, bytesConsumed: Int64)`. Both fields are
/// public to keep the type cheap to inspect.
public struct Utf8DecodeResult {
    /// The decoded character.
    public var char: Char

    /// How many bytes the encoded form occupied (1–4).
    public var bytesConsumed: Int64

    /// @name From Fields
    /// Constructs a decode result from an already-decoded char and byte length.
    ///
    /// Mainly used by `decodeUtf8` itself; user code rarely needs to
    /// build one directly.
    public init(char char: Char, bytesConsumed bytesConsumed: Int64) {
        self.char = char;
        self.bytesConsumed = bytesConsumed;
    }
}

// ============================================================================
// UTF-8 ENCODING/DECODING FUNCTIONS
// ============================================================================

// TODO: replace lang.i32_*/lang.ptr_*/lang.cast_* intrinsics in UTF-8 codec
// with UInt8/Int32/RawPointer wrappers after LLVM switch

/// Reads the byte at `ptr + offset` as an unsigned `lang.i32` in `0..=255`.
///
/// Helper used by `decodeUtf8` so the bit-twiddling in the main path
/// can pretend bytes are unsigned. The pointer must be valid for the
/// requested offset.
///
/// # Safety
///
/// Caller-checked — no bounds testing happens here. Used only inside
/// this file after explicit length checks.
func readByteAt(ptr: lang.ptr[lang.i8], offset: Int64) -> lang.i32 {
    let rawOffset: lang.i64 = offset.raw;
    let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](ptr, rawOffset);
    let signedByte: lang.i8 = lang.ptr_read(bytePtr);
    let asI32: lang.i32 = lang.cast_i8_i32(signedByte);
    lang.i32_and(asI32, 0xFF)
}

/// Writes `byte` to `ptr + offset`. Companion to `readByteAt`.
///
/// # Safety
///
/// Caller must ensure the offset is within an allocated, writable
/// region of memory.
func writeByteAt(ptr: lang.ptr[lang.i8], offset: Int64, byte: lang.i8) {
    let rawOffset: lang.i64 = offset.raw;
    let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](ptr, rawOffset);
    lang.ptr_write(bytePtr, byte)
}

/// Decodes one UTF-8 character starting at `index` inside the buffer of `length` bytes pointed to by `ptr`.
///
/// Returns `Some(Utf8DecodeResult)` on success, where `bytesConsumed`
/// is `1`–`4`. Returns `None` for any of the malformed-input cases:
/// truncated multi-byte sequence, continuation byte where a leading
/// byte was expected, or invalid leading byte (`0xF8`–`0xFF`).
/// **Does not** validate against overlong encodings or surrogate-range
/// scalars — feed only well-formed UTF-8 if those matter.
///
/// # Safety
///
/// `ptr` must be valid for `length` bytes. The function bounds-checks
/// `index` and any continuation bytes against `length`.
///
/// # Examples
///
/// ```
/// var result = String("hé");
/// // Conceptually:
/// // decodeUtf8(rawPtr, 3, at: 0)  // Some(char: 'h', bytesConsumed: 1)
/// // decodeUtf8(rawPtr, 3, at: 1)  // Some(char: 'é', bytesConsumed: 2)
/// // decodeUtf8(rawPtr, 3, at: 3)  // None (past the end)
/// ```
public func decodeUtf8(ptr: lang.ptr[lang.i8], length: Int64, at index: Int64) -> Utf8DecodeResult? {
    if index >= length {
        return .None
    }

    let firstU: lang.i32 = readByteAt(ptr, index);

    if lang.i32_unsigned_lt(firstU, 0x80) {
        // Single byte (ASCII): 0xxxxxxx
        let c = Char(UInt32(raw: firstU));
        return .Some(Utf8DecodeResult(char: c, bytesConsumed: 1))
    } else if lang.i32_unsigned_lt(firstU, 0xC0) {
        // Continuation byte as start - invalid
        return .None
    } else if lang.i32_unsigned_lt(firstU, 0xE0) {
        // Two bytes: 110xxxxx 10xxxxxx
        let idx1 = index + 1;
        if idx1 >= length { return .None }
        let second: lang.i32 = readByteAt(ptr, idx1);
        if lang.i32_ne(lang.i32_and(second, 0xC0), 0x80) { return .None }
        let v: lang.i32 = lang.i32_or(
            lang.i32_shl(lang.i32_and(firstU, 0x1F), 6),
            lang.i32_and(second, 0x3F)
        );
        let c = Char(UInt32(raw: v));
        return .Some(Utf8DecodeResult(char: c, bytesConsumed: 2))
    } else if lang.i32_unsigned_lt(firstU, 0xF0) {
        // Three bytes: 1110xxxx 10xxxxxx 10xxxxxx
        let idx1 = index + 1;
        let idx2 = index + 2;
        if idx2 >= length { return .None }
        let second: lang.i32 = readByteAt(ptr, idx1);
        let third: lang.i32 = readByteAt(ptr, idx2);
        if lang.i32_ne(lang.i32_and(second, 0xC0), 0x80) { return .None }
        if lang.i32_ne(lang.i32_and(third, 0xC0), 0x80) { return .None }
        let v: lang.i32 = lang.i32_or(
            lang.i32_or(
                lang.i32_shl(lang.i32_and(firstU, 0x0F), 12),
                lang.i32_shl(lang.i32_and(second, 0x3F), 6)
            ),
            lang.i32_and(third, 0x3F)
        );
        let c = Char(UInt32(raw: v));
        return .Some(Utf8DecodeResult(char: c, bytesConsumed: 3))
    } else if lang.i32_unsigned_lt(firstU, 0xF8) {
        // Four bytes: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
        let idx1 = index + 1;
        let idx2 = index + 2;
        let idx3 = index + 3;
        if idx3 >= length { return .None }
        let second: lang.i32 = readByteAt(ptr, idx1);
        let third: lang.i32 = readByteAt(ptr, idx2);
        let fourth: lang.i32 = readByteAt(ptr, idx3);
        if lang.i32_ne(lang.i32_and(second, 0xC0), 0x80) { return .None }
        if lang.i32_ne(lang.i32_and(third, 0xC0), 0x80) { return .None }
        if lang.i32_ne(lang.i32_and(fourth, 0xC0), 0x80) { return .None }
        let v: lang.i32 = lang.i32_or(
            lang.i32_or(
                lang.i32_or(
                    lang.i32_shl(lang.i32_and(firstU, 0x07), 18),
                    lang.i32_shl(lang.i32_and(second, 0x3F), 12)
                ),
                lang.i32_shl(lang.i32_and(third, 0x3F), 6)
            ),
            lang.i32_and(fourth, 0x3F)
        );
        let c = Char(UInt32(raw: v));
        return .Some(Utf8DecodeResult(char: c, bytesConsumed: 4))
    } else {
        // Invalid start byte
        return .None
    }
}

/// Encodes `c` as UTF-8 starting at `ptr + index`, returning the number of bytes written (1–4).
///
/// Companion of `decodeUtf8`. `c.utf8Length()` predicts the same byte
/// count without writing — call it first to ensure the buffer has
/// room.
///
/// # Safety
///
/// `ptr + index` through `ptr + index + utf8Length() - 1` must lie
/// within an allocated, writable region. No bounds checking happens
/// here.
///
/// # Examples
///
/// ```
/// // Conceptually, given a buffer `buf` of length 4:
/// // encodeUtf8('a',         buf, at: 0);  // 1
/// // encodeUtf8('\u{1F600}', buf, at: 0);  // 4
/// ```
public func encodeUtf8(c: Char, ptr: lang.ptr[lang.i8], at index: Int64) -> Int64 {
    let v: lang.i32 = c.value().raw;

    if lang.i32_unsigned_lt(v, 0x80) {
        // Single byte: 0xxxxxxx
        writeByteAt(ptr, index, lang.cast_i32_i8(v));
        1
    } else if lang.i32_unsigned_lt(v, 0x800) {
        // Two bytes: 110xxxxx 10xxxxxx
        let b1: lang.i8 = lang.cast_i32_i8(lang.i32_or(0xC0, lang.i32_and(lang.i32_unsigned_shr(v, 6), 0x1F)));
        let b2: lang.i8 = lang.cast_i32_i8(lang.i32_or(0x80, lang.i32_and(v, 0x3F)));
        let idx1 = index + 1;
        writeByteAt(ptr, index, b1);
        writeByteAt(ptr, idx1, b2);
        2
    } else if lang.i32_unsigned_lt(v, 0x10000) {
        // Three bytes: 1110xxxx 10xxxxxx 10xxxxxx
        let b1: lang.i8 = lang.cast_i32_i8(lang.i32_or(0xE0, lang.i32_and(lang.i32_unsigned_shr(v, 12), 0x0F)));
        let b2: lang.i8 = lang.cast_i32_i8(lang.i32_or(0x80, lang.i32_and(lang.i32_unsigned_shr(v, 6), 0x3F)));
        let b3: lang.i8 = lang.cast_i32_i8(lang.i32_or(0x80, lang.i32_and(v, 0x3F)));
        let idx1 = index + 1;
        let idx2 = index + 2;
        writeByteAt(ptr, index, b1);
        writeByteAt(ptr, idx1, b2);
        writeByteAt(ptr, idx2, b3);
        3
    } else {
        // Four bytes: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
        let b1: lang.i8 = lang.cast_i32_i8(lang.i32_or(0xF0, lang.i32_and(lang.i32_unsigned_shr(v, 18), 0x07)));
        let b2: lang.i8 = lang.cast_i32_i8(lang.i32_or(0x80, lang.i32_and(lang.i32_unsigned_shr(v, 12), 0x3F)));
        let b3: lang.i8 = lang.cast_i32_i8(lang.i32_or(0x80, lang.i32_and(lang.i32_unsigned_shr(v, 6), 0x3F)));
        let b4: lang.i8 = lang.cast_i32_i8(lang.i32_or(0x80, lang.i32_and(v, 0x3F)));
        let idx1 = index + 1;
        let idx2 = index + 2;
        let idx3 = index + 3;
        writeByteAt(ptr, index, b1);
        writeByteAt(ptr, idx1, b2);
        writeByteAt(ptr, idx2, b3);
        writeByteAt(ptr, idx3, b4);
        4
    }
}
