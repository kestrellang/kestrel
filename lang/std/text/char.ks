// Character types - Unicode code points and bytes

module std.text

import std.core.(Equatable, Comparable, Ordering, Bool, ExpressibleByCharLiteral)
import std.num.(Int64, UInt8, UInt32)
import std.result.(Optional)
import std.collections.(Array)

// Byte - alias for UInt8, represents a single UTF-8 byte
public type Byte = UInt8

// CodePoint - Unicode code point (single scalar value, 0 to 0x10FFFF)
public struct CodePoint: Equatable, Comparable, ExpressibleByCharLiteral {
    private var _value: UInt32

    public init(value: UInt32) {
        self._value = value;
    }

    public init(charLiteral value: lang.i32) {
        self._value = UInt32(raw: value);
    }

    public func value() -> UInt32 { self._value }

    // Character classification (ASCII subset)
    public func isAscii() -> Bool {
        self < '\u{80}'
    }

    public func isAlphabetic() -> Bool {
        (self >= 'A' and self <= 'Z') or (self >= 'a' and self <= 'z')
    }

    public func isDigit() -> Bool {
        self >= '0' and self <= '9'
    }

    public func isAlphanumeric() -> Bool {
        self.isAlphabetic() or self.isDigit()
    }

    public func isWhitespace() -> Bool {
        self == ' ' or self == '\t' or self == '\n' or self == '\r' or self == '\x0C'
    }

    public func isControl() -> Bool {
        self < ' ' or self == '\x7F'
    }

    public func isUppercase() -> Bool {
        self >= 'A' and self <= 'Z'
    }

    public func isLowercase() -> Bool {
        self >= 'a' and self <= 'z'
    }

    public func toUppercase() -> CodePoint {
        if self.isLowercase() {
            // 'a' - 'A' = 32
            CodePoint(self.value() - UInt32(intLiteral: 32))
        } else {
            self
        }
    }

    public func toLowercase() -> CodePoint {
        if self.isUppercase() {
            // 'a' - 'A' = 32
            CodePoint(self.value() + UInt32(intLiteral: 32))
        } else {
            self
        }
    }

    // UTF-8 encoding length for this code point
    public func utf8Length() -> Int64 {
        let v = self._value;
        if v < UInt32(intLiteral: 128) { Int64(intLiteral: 1) }
        else if v < UInt32(intLiteral: 2048) { Int64(intLiteral: 2) }
        else if v < UInt32(intLiteral: 65536) { Int64(intLiteral: 3) }
        else { Int64(intLiteral: 4) }
    }

    // ASCII digit value (0-9), or None if not a digit
    public func digitValue() -> Optional[UInt32] {
        if self.isDigit() {
            let zero: CodePoint = '0';
            .Some(self.value() - zero.value())
        } else {
            .None
        }
    }

    // Create from ASCII digit (0-9)
    public static func fromDigit(d: UInt32) -> Optional[CodePoint] {
        if d <= UInt32(intLiteral: 9) {
            let zero: CodePoint = '0';
            .Some(CodePoint(d + zero.value()))
        } else {
            .None
        }
    }

    // Equatable
    public func equals(other: CodePoint) -> Bool {
        self._value == other._value
    }

    // Comparable
    public func compare(other: CodePoint) -> Ordering {
        self._value.compare(other._value)
    }
}

// Char - Extended grapheme cluster (user-perceived character)
// May be multiple code points (e.g., "é" as e + combining accent, or emoji sequences)
public struct Char: Equatable {
    private var _codePoints: Array[CodePoint]

    public init(codePoint codePoint: CodePoint) {
        self._codePoints = Array();
        self._codePoints.append(codePoint);
    }

    public init(codePoints codePoints: Array[CodePoint]) {
        self._codePoints = codePoints;
    }

    public func codePoints() -> Array[CodePoint] { self._codePoints }

    public func codePointCount() -> Int64 {
        self._codePoints.count()
    }

    public func firstCodePoint() -> Optional[CodePoint] {
        self._codePoints.first()
    }

    public func isAscii() -> Bool {
        let count = self._codePoints.count();
        if count == Int64(intLiteral: 1) {
            self._codePoints.getUnchecked(Int64(intLiteral: 0)).isAscii()
        } else {
            false
        }
    }

    // Byte length when encoded as UTF-8
    public func utf8Length() -> Int64 {
        var len: Int64 = Int64(intLiteral: 0);
        var i: Int64 = Int64(intLiteral: 0);
        let count = self._codePoints.count();
        while i < count {
            len = len + self._codePoints.getUnchecked(i).utf8Length();
            i = i + Int64(intLiteral: 1)
        }
        len
    }

    // Equatable
    public func equals(other: Char) -> Bool {
        let selfCount = self._codePoints.count();
        let otherCount = other._codePoints.count();
        if selfCount != otherCount {
            return false
        }
        var i: Int64 = Int64(intLiteral: 0);
        var equal: Bool = true;
        while i < selfCount and equal {
            if self._codePoints.getUnchecked(i).equals(other._codePoints.getUnchecked(i)) == false {
                equal = false
            }
            i = i + Int64(intLiteral: 1)
        }
        equal
    }
}

// Common ASCII code points as constants
public struct AsciiChars {
    public static func space() -> CodePoint { ' ' }
    public static func newline() -> CodePoint { '\n' }
    public static func carriageReturn() -> CodePoint { '\r' }
    public static func tab() -> CodePoint { '\t' }
    public static func nul() -> CodePoint { '\0' }
    public static func slash() -> CodePoint { '/' }
    public static func backslash() -> CodePoint { '\\' }
    public static func dot() -> CodePoint { '.' }
    public static func comma() -> CodePoint { ',' }
    public static func colon() -> CodePoint { ':' }
    public static func semicolon() -> CodePoint { ';' }
    public static func quote() -> CodePoint { '"' }
    public static func apostrophe() -> CodePoint { '\'' }
}

// UTF-8 decoding result
public struct Utf8DecodeResult {
    public var codePoint: CodePoint
    public var bytesConsumed: Int64

    public init(codePoint codePoint: CodePoint, bytesConsumed bytesConsumed: Int64) {
        self.codePoint = codePoint;
        self.bytesConsumed = bytesConsumed;
    }
}

// Helper to read byte at offset (returns unsigned byte value as i32)
func readByteAt(ptr: lang.ptr[lang.i8], offset: Int64) -> lang.i32 {
    let rawOffset: lang.i64 = offset.raw;
    let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](ptr, rawOffset);
    let signedByte: lang.i8 = lang.ptr_read(bytePtr);
    let asI32: lang.i32 = lang.cast_i8_i32(signedByte);
    lang.i32_and(asI32, 0xFF)
}

// Helper to write byte at offset
func writeByteAt(ptr: lang.ptr[lang.i8], offset: Int64, byte: lang.i8) {
    let rawOffset: lang.i64 = offset.raw;
    let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](ptr, rawOffset);
    lang.ptr_write(bytePtr, byte)
}

// Decode a single UTF-8 code point from raw bytes
// ptr: pointer to UTF-8 bytes, length: total byte count, index: starting position
// Returns Utf8DecodeResult or None if invalid
public func decodeUtf8(ptr: lang.ptr[lang.i8], length: Int64, at index: Int64) -> Optional[Utf8DecodeResult] {
    if index >= length {
        return .None
    }

    let firstU: lang.i32 = readByteAt(ptr, index);

    if lang.i32_unsigned_lt(firstU, 0x80) {
        // Single byte (ASCII): 0xxxxxxx
        let cp = CodePoint(UInt32(raw: firstU));
        return .Some(Utf8DecodeResult(codePoint: cp, bytesConsumed: Int64(intLiteral: 1)))
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
        let cp = CodePoint(UInt32(raw: v));
        return .Some(Utf8DecodeResult(codePoint: cp, bytesConsumed: Int64(intLiteral: 2)))
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
        let cp = CodePoint(UInt32(raw: v));
        return .Some(Utf8DecodeResult(codePoint: cp, bytesConsumed: Int64(intLiteral: 3)))
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
        let cp = CodePoint(UInt32(raw: v));
        return .Some(Utf8DecodeResult(codePoint: cp, bytesConsumed: Int64(intLiteral: 4)))
    } else {
        // Invalid start byte
        return .None
    }
}

// Encode a code point to UTF-8, writing to a buffer
// Returns number of bytes written (1-4)
public func encodeUtf8(cp: CodePoint, ptr: lang.ptr[lang.i8], at index: Int64) -> Int64 {
    let v: lang.i32 = cp.value().raw;

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
