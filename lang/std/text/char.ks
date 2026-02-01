// Character types - Unicode code points and bytes

module std.text

import std.core.(Equatable, Comparable, Ordering, Bool, Matchable, ExpressibleByCharLiteral, Hash, Hasher, RangeMatchable)
import std.num.(Int64, UInt8, UInt32)
import std.result.(Optional)
import std.collections.(Array)

// ============================================================================
// TYPE ALIASES
// ============================================================================

/// Alias for UInt8, representing a single UTF-8 byte.
public type Byte = UInt8

// ============================================================================
// CHAR
// ============================================================================

/// A Unicode code point (scalar value from 0 to 0x10FFFF).
///
/// Supports character literal syntax: 'a', '\n', '\u{1F600}'.
public struct Char: Equatable, Comparable, Matchable, ExpressibleByCharLiteral, Hash, RangeMatchable {
    private var _value: UInt32

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Creates a Char from a Unicode code point value.
    public init(value: UInt32) {
        self._value = value;
    }

    /// Creates a Char from a character literal.
    public init(charLiteral value: lang.i32) {
        self._value = UInt32(raw: value);
    }

    // ========================================================================
    // VALUE ACCESS
    // ========================================================================

    /// Returns the Unicode code point value.
    public func value() -> UInt32 { self._value }

    // ========================================================================
    // CHARACTER CLASSIFICATION
    // ========================================================================

    /// Returns true if this is an ASCII character (< 128).
    public func isAscii() -> Bool {
        self < '\u{80}'
    }

    /// Returns true if this is an alphabetic character (a-z, A-Z).
    public func isAlphabetic() -> Bool {
        (self >= 'A' and self <= 'Z') or (self >= 'a' and self <= 'z')
    }

    /// Returns true if this is a digit (0-9).
    public func isDigit() -> Bool {
        self >= '0' and self <= '9'
    }

    /// Returns true if this is alphanumeric.
    public func isAlphanumeric() -> Bool {
        self.isAlphabetic() or self.isDigit()
    }

    /// Returns true if this is whitespace (space, tab, newline, etc.).
    public func isWhitespace() -> Bool {
        self == ' ' or self == '\t' or self == '\n' or self == '\r' or self == '\x0C'
    }

    /// Returns true if this is a control character.
    public func isControl() -> Bool {
        self < ' ' or self == '\x7F'
    }

    /// Returns true if this is an uppercase letter (A-Z).
    public func isUppercase() -> Bool {
        self >= 'A' and self <= 'Z'
    }

    /// Returns true if this is a lowercase letter (a-z).
    public func isLowercase() -> Bool {
        self >= 'a' and self <= 'z'
    }

    // ========================================================================
    // CASE CONVERSION
    // ========================================================================

    /// Returns the uppercase version of this character.
    public func toUppercase() -> Char {
        if self.isLowercase() {
            // 'a' - 'A' = 32
            Char(self.value() - UInt32(intLiteral: 32))
        } else {
            self
        }
    }

    /// Returns the lowercase version of this character.
    public func toLowercase() -> Char {
        if self.isUppercase() {
            // 'a' - 'A' = 32
            Char(self.value() + UInt32(intLiteral: 32))
        } else {
            self
        }
    }

    // ========================================================================
    // UTF-8 ENCODING
    // ========================================================================

    /// Returns the number of bytes needed to encode this char in UTF-8 (1-4).
    public func utf8Length() -> Int64 {
        let v = self._value;
        if v < UInt32(intLiteral: 128) { Int64(intLiteral: 1) }
        else if v < UInt32(intLiteral: 2048) { Int64(intLiteral: 2) }
        else if v < UInt32(intLiteral: 65536) { Int64(intLiteral: 3) }
        else { Int64(intLiteral: 4) }
    }

    // ========================================================================
    // DIGIT CONVERSION
    // ========================================================================

    /// Returns the digit value (0-9) if this is a digit, otherwise None.
    public func digitValue() -> UInt32? {
        if self.isDigit() {
            let zero: Char = '0';
            .Some(self.value() - zero.value())
        } else {
            .None
        }
    }

    /// Creates a Char from a digit value (0-9).
    public static func fromDigit(d: UInt32) -> Char? {
        if d <= UInt32(intLiteral: 9) {
            let zero: Char = '0';
            .Some(Char(d + zero.value()))
        } else {
            .None
        }
    }

    // ========================================================================
    // PROTOCOL CONFORMANCES
    // ========================================================================

    /// Compares two characters for equality.
    public func equals(other: Char) -> Bool {
        self._value == other._value
    }

    /// Matches two characters for pattern matching.
    public func matches(other: Char) -> Bool {
        self._value == other._value
    }

    /// Compares two characters for ordering.
    public func compare(other: Char) -> Ordering {
        self._value.compare(other._value)
    }

    /// Hashes this character.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        let val = self._value;
        hasher.write(Slice(pointer: Pointer(to: val).asRaw().cast[UInt8](), count: Int64(intLiteral: 4)))
    }

    /// Returns true if self >= bound (for range pattern matching).
    public func isAtLeast(bound: Char) -> Bool {
        self.compare(bound) != Ordering.Less
    }

    /// Returns true if self <= bound (for range pattern matching).
    public func isAtMost(bound: Char) -> Bool {
        self.compare(bound) != Ordering.Greater
    }

    /// Returns true if self < bound (for range pattern matching).
    public func isBelow(bound: Char) -> Bool {
        self.compare(bound) == Ordering.Less
    }
}

// ============================================================================
// GRAPHEME
// ============================================================================

/// An extended grapheme cluster (user-perceived character).
///
/// May consist of multiple code points (e.g., emoji sequences).
public struct Grapheme: Equatable {
    private var _chars: Array[Char]

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Creates a Grapheme from a single Char.
    public init(char char: Char) {
        self._chars = Array[Char]();
        self._chars.append(char);
    }

    /// Creates a Grapheme from multiple Chars.
    public init(chars chars: Array[Char]) {
        self._chars = chars;
    }

    // ========================================================================
    // ACCESSORS
    // ========================================================================

    /// Returns the code points in this grapheme.
    public func chars() -> Array[Char] { self._chars }

    /// Returns the number of code points.
    public func charCount() -> Int64 {
        self._chars.count()
    }

    /// Returns the first code point, or None if empty.
    public func firstChar() -> Char? {
        self._chars.first()
    }

    /// Returns true if this is a single ASCII character.
    public func isAscii() -> Bool {
        let count = self._chars.count();
        if count == Int64(intLiteral: 1) {
            self._chars.getUnchecked(Int64(intLiteral: 0)).isAscii()
        } else {
            false
        }
    }

    /// Returns the byte length when encoded as UTF-8.
    public func utf8Length() -> Int64 {
        var len: Int64 = Int64(intLiteral: 0);
        let count = self._chars.count();
        for i in Int64(intLiteral: 0)..<count {
            len = len + self._chars.getUnchecked(i).utf8Length()
        }
        len
    }

    // ========================================================================
    // PROTOCOL CONFORMANCES
    // ========================================================================

    /// Compares two graphemes for equality.
    public func equals(other: Grapheme) -> Bool {
        let selfCount = self._chars.count();
        let otherCount = other._chars.count();
        if selfCount != otherCount {
            return false
        }
        var equal: Bool = true;
        for i in Int64(intLiteral: 0)..<selfCount {
            if self._chars.getUnchecked(i).equals(other._chars.getUnchecked(i)) == false {
                equal = false
            }
        }
        equal
    }
}

// ============================================================================
// ASCII CONSTANTS
// ============================================================================

/// Common ASCII characters as constants.
public struct AsciiChars {
    /// Space character (' ').
    public static func space() -> Char { ' ' }

    /// Newline character ('\n').
    public static func newline() -> Char { '\n' }

    /// Carriage return character ('\r').
    public static func carriageReturn() -> Char { '\r' }

    /// Tab character ('\t').
    public static func tab() -> Char { '\t' }

    /// Null character ('\0').
    public static func nul() -> Char { '\0' }

    /// Forward slash ('/').
    public static func slash() -> Char { '/' }

    /// Backslash ('\\').
    public static func backslash() -> Char { '\\' }

    /// Period ('.').
    public static func dot() -> Char { '.' }

    /// Comma (',').
    public static func comma() -> Char { ',' }

    /// Colon (':').
    public static func colon() -> Char { ':' }

    /// Semicolon (';').
    public static func semicolon() -> Char { ';' }

    /// Double quote ('"').
    public static func quote() -> Char { '"' }

    /// Single quote/apostrophe ('\'').
    public static func apostrophe() -> Char { '\'' }
}

// ============================================================================
// UTF-8 DECODING RESULT
// ============================================================================

/// Result of decoding a UTF-8 character.
public struct Utf8DecodeResult {
    /// The decoded character.
    public var char: Char

    /// Number of bytes consumed from the input.
    public var bytesConsumed: Int64

    /// Creates a decode result.
    public init(char char: Char, bytesConsumed bytesConsumed: Int64) {
        self.char = char;
        self.bytesConsumed = bytesConsumed;
    }
}

// ============================================================================
// UTF-8 ENCODING/DECODING FUNCTIONS
// ============================================================================

/// Helper to read byte at offset (returns unsigned byte value as i32).
func readByteAt(ptr: lang.ptr[lang.i8], offset: Int64) -> lang.i32 {
    let rawOffset: lang.i64 = offset.raw;
    let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](ptr, rawOffset);
    let signedByte: lang.i8 = lang.ptr_read(bytePtr);
    let asI32: lang.i32 = lang.cast_i8_i32(signedByte);
    lang.i32_and(asI32, 0xFF)
}

/// Helper to write byte at offset.
func writeByteAt(ptr: lang.ptr[lang.i8], offset: Int64, byte: lang.i8) {
    let rawOffset: lang.i64 = offset.raw;
    let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](ptr, rawOffset);
    lang.ptr_write(bytePtr, byte)
}

/// Decodes a single UTF-8 character from raw bytes.
///
/// Returns None if the encoding is invalid.
public func decodeUtf8(ptr: lang.ptr[lang.i8], length: Int64, at index: Int64) -> Utf8DecodeResult? {
    if index >= length {
        return .None
    }

    let firstU: lang.i32 = readByteAt(ptr, index);

    if lang.i32_unsigned_lt(firstU, 0x80) {
        // Single byte (ASCII): 0xxxxxxx
        let c = Char(UInt32(raw: firstU));
        return .Some(Utf8DecodeResult(char: c, bytesConsumed: Int64(intLiteral: 1)))
    } else if lang.i32_unsigned_lt(firstU, 0xC0) {
        // Continuation byte as start - invalid
        return .None
    } else if lang.i32_unsigned_lt(firstU, 0xE0) {
        // Two bytes: 110xxxxx 10xxxxxx
        let idx1 = index + Int64(intLiteral: 1);
        if idx1 >= length { return .None }
        let second: lang.i32 = readByteAt(ptr, idx1);
        if lang.i32_ne(lang.i32_and(second, 0xC0), 0x80) { return .None }
        let v: lang.i32 = lang.i32_or(
            lang.i32_shl(lang.i32_and(firstU, 0x1F), 6),
            lang.i32_and(second, 0x3F)
        );
        let c = Char(UInt32(raw: v));
        return .Some(Utf8DecodeResult(char: c, bytesConsumed: Int64(intLiteral: 2)))
    } else if lang.i32_unsigned_lt(firstU, 0xF0) {
        // Three bytes: 1110xxxx 10xxxxxx 10xxxxxx
        let idx1 = index + Int64(intLiteral: 1);
        let idx2 = index + Int64(intLiteral: 2);
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
        return .Some(Utf8DecodeResult(char: c, bytesConsumed: Int64(intLiteral: 3)))
    } else if lang.i32_unsigned_lt(firstU, 0xF8) {
        // Four bytes: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
        let idx1 = index + Int64(intLiteral: 1);
        let idx2 = index + Int64(intLiteral: 2);
        let idx3 = index + Int64(intLiteral: 3);
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
        return .Some(Utf8DecodeResult(char: c, bytesConsumed: Int64(intLiteral: 4)))
    } else {
        // Invalid start byte
        return .None
    }
}

/// Encodes a character to UTF-8, writing to a buffer.
///
/// Returns the number of bytes written (1-4).
public func encodeUtf8(c: Char, ptr: lang.ptr[lang.i8], at index: Int64) -> Int64 {
    let v: lang.i32 = c.value().raw;

    if lang.i32_unsigned_lt(v, 0x80) {
        // Single byte: 0xxxxxxx
        writeByteAt(ptr, index, lang.cast_i32_i8(v));
        Int64(intLiteral: 1)
    } else if lang.i32_unsigned_lt(v, 0x800) {
        // Two bytes: 110xxxxx 10xxxxxx
        let b1: lang.i8 = lang.cast_i32_i8(lang.i32_or(0xC0, lang.i32_and(lang.i32_unsigned_shr(v, 6), 0x1F)));
        let b2: lang.i8 = lang.cast_i32_i8(lang.i32_or(0x80, lang.i32_and(v, 0x3F)));
        let idx1 = index + Int64(intLiteral: 1);
        writeByteAt(ptr, index, b1);
        writeByteAt(ptr, idx1, b2);
        Int64(intLiteral: 2)
    } else if lang.i32_unsigned_lt(v, 0x10000) {
        // Three bytes: 1110xxxx 10xxxxxx 10xxxxxx
        let b1: lang.i8 = lang.cast_i32_i8(lang.i32_or(0xE0, lang.i32_and(lang.i32_unsigned_shr(v, 12), 0x0F)));
        let b2: lang.i8 = lang.cast_i32_i8(lang.i32_or(0x80, lang.i32_and(lang.i32_unsigned_shr(v, 6), 0x3F)));
        let b3: lang.i8 = lang.cast_i32_i8(lang.i32_or(0x80, lang.i32_and(v, 0x3F)));
        let idx1 = index + Int64(intLiteral: 1);
        let idx2 = index + Int64(intLiteral: 2);
        writeByteAt(ptr, index, b1);
        writeByteAt(ptr, idx1, b2);
        writeByteAt(ptr, idx2, b3);
        Int64(intLiteral: 3)
    } else {
        // Four bytes: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
        let b1: lang.i8 = lang.cast_i32_i8(lang.i32_or(0xF0, lang.i32_and(lang.i32_unsigned_shr(v, 18), 0x07)));
        let b2: lang.i8 = lang.cast_i32_i8(lang.i32_or(0x80, lang.i32_and(lang.i32_unsigned_shr(v, 12), 0x3F)));
        let b3: lang.i8 = lang.cast_i32_i8(lang.i32_or(0x80, lang.i32_and(lang.i32_unsigned_shr(v, 6), 0x3F)));
        let b4: lang.i8 = lang.cast_i32_i8(lang.i32_or(0x80, lang.i32_and(v, 0x3F)));
        let idx1 = index + Int64(intLiteral: 1);
        let idx2 = index + Int64(intLiteral: 2);
        let idx3 = index + Int64(intLiteral: 3);
        writeByteAt(ptr, index, b1);
        writeByteAt(ptr, idx1, b2);
        writeByteAt(ptr, idx2, b3);
        writeByteAt(ptr, idx3, b4);
        Int64(intLiteral: 4)
    }
}
