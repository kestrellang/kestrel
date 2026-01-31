// String views for different representations

module std.text

import std.core.(Bool, Equatable, Comparable, Ordering)
import std.num.(Int64, UInt8, UInt32)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)
import std.text.(Char, Grapheme, Byte, decodeUtf8, String)
import std.collections.(Array)

// ============================================================================
// BYTES VIEW
// ============================================================================

/// Iterator over raw UTF-8 bytes.
public struct BytesIterator: Iterator {
    type Item = UInt8

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64
    private var index: Int64

    /// Creates a bytes iterator.
    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64, index index: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.index = index;
    }

    /// Returns the next byte, or None if exhausted.
    public mutating func next() -> Optional[UInt8] {
        if self.index < self.length {
            let rawOffset: lang.i64 = self.index.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let signedByte: lang.i8 = lang.ptr_read(bytePtr);
            self.index = self.index + Int64(intLiteral: 1);
            .Some(UInt8(raw: signedByte))
        } else {
            .None
        }
    }
}

/// A view over the raw UTF-8 bytes of a string.
///
/// Provides O(1) indexing by byte position.
public struct BytesView: Iterable {
    type Item = UInt8
    type Iter = BytesIterator

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64

    /// Creates a bytes view.
    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
    }

    /// Returns the number of bytes.
    public func count() -> Int64 { self.length }

    /// Returns true if the view is empty.
    public func isEmpty() -> Bool { self.length == Int64(intLiteral: 0) }

    /// Returns the raw pointer to the bytes.
    public func asRaw() -> lang.ptr[lang.i8] { self.ptr }

    /// Returns the byte at the given index, or None if out of bounds.
    public func byteAt(index: Int64) -> Optional[UInt8] {
        if index >= Int64(intLiteral: 0) and index < self.length {
            let rawOffset: lang.i64 = index.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let signedByte: lang.i8 = lang.ptr_read(bytePtr);
            .Some(UInt8(raw: signedByte))
        } else {
            .None
        }
    }

    /// Returns the byte at the given index without bounds checking.
    public func byteAtUnchecked(index: Int64) -> UInt8 {
        let rawOffset: lang.i64 = index.raw;
        let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
        let signedByte: lang.i8 = lang.ptr_read(bytePtr);
        UInt8(raw: signedByte)
    }

    /// Returns an iterator over the bytes.
    public func iter() -> BytesIterator {
        BytesIterator(ptr: self.ptr, length: self.length, index: Int64(intLiteral: 0))
    }
}

// ============================================================================
// CHARS VIEW
// ============================================================================

/// Iterator over Unicode code points.
public struct CharsIterator: Iterator {
    type Item = Char

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64
    private var byteIndex: Int64

    /// Creates a chars iterator.
    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64, byteIndex byteIndex: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.byteIndex = byteIndex;
    }

    /// Returns the next character, or None if exhausted.
    public mutating func next() -> Optional[Char] {
        if self.byteIndex >= self.length {
            return .None
        }

        let result = decodeUtf8(self.ptr, self.length, at: self.byteIndex);
        if result.isSome() {
            let decoded = result.unwrap();
            self.byteIndex = self.byteIndex + decoded.bytesConsumed;
            .Some(decoded.char)
        } else {
            // Invalid UTF-8 - skip byte and return replacement character
            self.byteIndex = self.byteIndex + Int64(intLiteral: 1);
            let replacementValue = UInt32(raw: 0xFFFD);
            .Some(Char(replacementValue))
        }
    }
}

/// A view over the Unicode code points in a string.
///
/// Iteration is O(1) per character, but indexing is O(n).
public struct CharsView: Iterable {
    type Item = Char
    type Iter = CharsIterator

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64

    /// Creates a chars view.
    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
    }

    /// Returns an iterator over the characters.
    public func iter() -> CharsIterator {
        CharsIterator(ptr: self.ptr, length: self.length, byteIndex: Int64(intLiteral: 0))
    }

    /// Returns the number of characters (O(n) - must decode all).
    public func count() -> Int64 {
        var n: Int64 = Int64(intLiteral: 0);
        var i: Int64 = Int64(intLiteral: 0);
        while i < self.length {
            // Count leading bytes only (not continuation bytes 10xxxxxx)
            let rawOffset: lang.i64 = i.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let signedByte: lang.i8 = lang.ptr_read(bytePtr);
            let byteVal: lang.i32 = lang.cast_i8_i32(signedByte);
            if lang.i32_ne(lang.i32_and(byteVal, 0xC0), 0x80) {
                n = n + Int64(intLiteral: 1)
            }
            i = i + Int64(intLiteral: 1)
        }
        n
    }
}

// ============================================================================
// GRAPHEMES VIEW
// ============================================================================

/// Iterator over grapheme clusters.
public struct GraphemesIterator: Iterator {
    type Item = Grapheme

    private var charsIter: CharsIterator

    /// Creates a graphemes iterator.
    public init(charsIter: CharsIterator) {
        self.charsIter = charsIter;
    }

    /// Returns the next grapheme cluster, or None if exhausted.
    public mutating func next() -> Optional[Grapheme] {
        // Simplified: treat each char as a grapheme
        // Full implementation would need grapheme cluster segmentation
        let maybeChar = self.charsIter.next();
        if maybeChar.isSome() {
            .Some(Grapheme(char: maybeChar.unwrap()))
        } else {
            .None
        }
    }
}

/// A view over the extended grapheme clusters in a string.
///
/// Note: Simplified implementation treats each char as a grapheme.
public struct GraphemesView: Iterable {
    type Item = Grapheme
    type Iter = GraphemesIterator

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64

    /// Creates a graphemes view.
    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
    }

    /// Returns an iterator over the grapheme clusters.
    public func iter() -> GraphemesIterator {
        GraphemesIterator(CharsIterator(ptr: self.ptr, length: self.length, byteIndex: Int64(intLiteral: 0)))
    }

    /// Returns the number of grapheme clusters (O(n)).
    public func count() -> Int64 {
        // Simplified: same as char count
        var n: Int64 = Int64(intLiteral: 0);
        var i: Int64 = Int64(intLiteral: 0);
        while i < self.length {
            let rawOffset: lang.i64 = i.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let signedByte: lang.i8 = lang.ptr_read(bytePtr);
            let byteVal: lang.i32 = lang.cast_i8_i32(signedByte);
            if lang.i32_ne(lang.i32_and(byteVal, 0xC0), 0x80) {
                n = n + Int64(intLiteral: 1)
            }
            i = i + Int64(intLiteral: 1)
        }
        n
    }
}

// ============================================================================
// LINES VIEW
// ============================================================================

/// Iterator over lines in a string.
public struct LinesIterator: Iterator {
    type Item = String

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64
    private var byteIndex: Int64
    private var done: Bool

    /// Creates a lines iterator.
    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64, byteIndex byteIndex: Int64, done done: Bool) {
        self.ptr = ptr;
        self.length = length;
        self.byteIndex = byteIndex;
        self.done = done;
    }

    /// Returns the next line, or None if exhausted.
    public mutating func next() -> Optional[String] {
        if self.done or self.byteIndex >= self.length {
            return .None
        }

        let start = self.byteIndex;

        // Find next newline
        var foundNewline: Bool = false;
        var lineEnd: Int64 = self.byteIndex;

        while self.byteIndex < self.length and foundNewline == false {
            let rawOffset: lang.i64 = self.byteIndex.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let byte: lang.i8 = lang.ptr_read(bytePtr);
            let byteVal: lang.i32 = lang.cast_i8_i32(byte);
            let unsignedByte: lang.i32 = lang.i32_and(byteVal, 0xFF);

            if Bool(boolLiteral: lang.i32_eq(unsignedByte, 10)) {  // \n
                lineEnd = self.byteIndex;
                self.byteIndex = self.byteIndex + Int64(intLiteral: 1);
                foundNewline = true
            } else if Bool(boolLiteral: lang.i32_eq(unsignedByte, 13)) {  // \r
                lineEnd = self.byteIndex;
                self.byteIndex = self.byteIndex + Int64(intLiteral: 1);
                // Handle \r\n
                if self.byteIndex < self.length {
                    let nextOffset: lang.i64 = self.byteIndex.raw;
                    let nextBytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, nextOffset);
                    let nextByte: lang.i8 = lang.ptr_read(nextBytePtr);
                    let nextByteVal: lang.i32 = lang.cast_i8_i32(nextByte);
                    let nextUnsigned: lang.i32 = lang.i32_and(nextByteVal, 0xFF);
                    if Bool(boolLiteral: lang.i32_eq(nextUnsigned, 10)) {
                        self.byteIndex = self.byteIndex + Int64(intLiteral: 1)
                    }
                }
                foundNewline = true
            } else {
                self.byteIndex = self.byteIndex + Int64(intLiteral: 1)
            }
        }

        if foundNewline {
            // Return the line without newline characters
            return .Some(self.createSubstring(start, lineEnd))
        }

        // Last line (no trailing newline)
        if start < self.length {
            self.done = true;
            return .Some(self.createSubstring(start, self.length))
        }

        let none: Optional[String] = .None;
        none
    }

    /// Helper to create a substring from byte range.
    private func createSubstring(start: Int64, end: Int64) -> String {
        let count = end - start;
        if count == Int64(intLiteral: 0) {
            return String()
        }
        // Create string from bytes
        var result = String(capacity: count);
        var currentIndex: Int64 = start;
        while currentIndex < end {
            let rawOffset: lang.i64 = currentIndex.raw;
            let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
            let byte: lang.i8 = lang.ptr_read(bytePtr);
            result.appendByte(UInt8(raw: byte));
            currentIndex = currentIndex + Int64(intLiteral: 1)
        }
        result
    }
}

/// A view that iterates over lines in a string.
///
/// Handles both \n and \r\n line endings.
public struct LinesView: Iterable {
    type Item = String
    type Iter = LinesIterator

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64

    /// Creates a lines view.
    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
    }

    /// Returns an iterator over the lines.
    public func iter() -> LinesIterator {
        LinesIterator(ptr: self.ptr, length: self.length, byteIndex: Int64(intLiteral: 0), done: false)
    }
}

// ============================================================================
// STRING INDEX TYPES
// ============================================================================

/// An index into a string by byte position.
public struct ByteIndex: Equatable, Comparable {
    /// The byte offset value.
    public var value: Int64

    /// Creates a byte index.
    public init(value: Int64) {
        self.value = value;
    }

    /// Compares two byte indices for equality.
    public func equals(other: ByteIndex) -> Bool {
        self.value == other.value
    }

    /// Compares two byte indices for ordering.
    public func compare(other: ByteIndex) -> Ordering {
        self.value.compare(other.value)
    }
}

/// An index into a string by character position.
public struct CharIndex: Equatable {
    /// The byte offset of this character.
    public var byteOffset: Int64

    /// Creates a char index.
    public init(byteOffset: Int64) {
        self.byteOffset = byteOffset;
    }

    /// Compares two char indices for equality.
    public func equals(other: CharIndex) -> Bool {
        self.byteOffset == other.byteOffset
    }
}

/// An index into a string by grapheme cluster position.
public struct GraphemeIndex: Equatable {
    /// The byte offset of this grapheme.
    public var byteOffset: Int64

    /// Creates a grapheme index.
    public init(byteOffset: Int64) {
        self.byteOffset = byteOffset;
    }

    /// Compares two grapheme indices for equality.
    public func equals(other: GraphemeIndex) -> Bool {
        self.byteOffset == other.byteOffset
    }
}
