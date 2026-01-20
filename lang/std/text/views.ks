// String views for different representations

module std.text

import std.core.(Bool, Equatable, Comparable, Ordering)
import std.num.(Int64, UInt8, UInt32)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)
import std.text.(CodePoint, Char, Byte, decodeUtf8, String)
import std.collections.(Array)

// BytesIterator must be defined before BytesView for Iterable conformance
public struct BytesIterator: Iterator {
    type Item = UInt8

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64
    private var index: Int64

    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64, index index: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.index = index;
    }

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

// BytesView - raw UTF-8 bytes (O(1) indexing)
public struct BytesView: Iterable {
    type Item = UInt8
    type Iter = BytesIterator

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64

    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
    }

    public func count() -> Int64 { self.length }

    public func isEmpty() -> Bool { self.length == Int64(intLiteral: 0) }

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

    public func byteAtUnchecked(index: Int64) -> UInt8 {
        let rawOffset: lang.i64 = index.raw;
        let bytePtr: lang.ptr[lang.i8] = lang.ptr_offset[lang.i8](self.ptr, rawOffset);
        let signedByte: lang.i8 = lang.ptr_read(bytePtr);
        UInt8(raw: signedByte)
    }

    public func iter() -> BytesIterator {
        BytesIterator(ptr: self.ptr, length: self.length, index: Int64(intLiteral: 0))
    }
}

// CodePointsIterator must be defined before CodePointsView
public struct CodePointsIterator: Iterator {
    type Item = CodePoint

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64
    private var byteIndex: Int64

    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64, byteIndex byteIndex: Int64) {
        self.ptr = ptr;
        self.length = length;
        self.byteIndex = byteIndex;
    }

    public mutating func next() -> Optional[CodePoint] {
        if self.byteIndex >= self.length {
            return .None
        }

        let result = decodeUtf8(self.ptr, self.length, at: self.byteIndex);
        if result.isSome() {
            let decoded = result.unwrap();
            self.byteIndex = self.byteIndex + decoded.bytesConsumed;
            .Some(decoded.codePoint)
        } else {
            // Invalid UTF-8 - skip byte and return replacement character
            self.byteIndex = self.byteIndex + Int64(intLiteral: 1);
            let replacementValue = UInt32(raw: 0xFFFD);
            .Some(CodePoint(replacementValue))
        }
    }
}

// CodePointsView - Unicode code points (O(1) iteration, O(n) indexing)
public struct CodePointsView: Iterable {
    type Item = CodePoint
    type Iter = CodePointsIterator

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64

    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
    }

    public func iter() -> CodePointsIterator {
        CodePointsIterator(ptr: self.ptr, length: self.length, byteIndex: Int64(intLiteral: 0))
    }

    // Count is O(n) - must decode all code points
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

// CharsIterator must be defined before CharsView
public struct CharsIterator: Iterator {
    type Item = Char

    private var codePointsIter: CodePointsIterator

    public init(codePointsIter: CodePointsIterator) {
        self.codePointsIter = codePointsIter;
    }

    public mutating func next() -> Optional[Char] {
        // Simplified: treat each code point as a character
        // Full implementation would need grapheme cluster segmentation
        let maybeCP = self.codePointsIter.next();
        if maybeCP.isSome() {
            .Some(Char(codePoint: maybeCP.unwrap()))
        } else {
            .None
        }
    }
}

// CharsView - Extended grapheme clusters (O(1) iteration, O(n) indexing)
// Note: Full grapheme cluster support requires Unicode segmentation tables
// This is a simplified implementation that treats each code point as a char
public struct CharsView: Iterable {
    type Item = Char
    type Iter = CharsIterator

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64

    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
    }

    public func iter() -> CharsIterator {
        CharsIterator(CodePointsIterator(ptr: self.ptr, length: self.length, byteIndex: Int64(intLiteral: 0)))
    }

    // Count is O(n) - must process all grapheme clusters
    public func count() -> Int64 {
        // Simplified: same as code point count
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

// LinesIterator must be defined before LinesView
public struct LinesIterator: Iterator {
    type Item = String

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64
    private var byteIndex: Int64
    private var done: Bool

    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64, byteIndex byteIndex: Int64, done done: Bool) {
        self.ptr = ptr;
        self.length = length;
        self.byteIndex = byteIndex;
        self.done = done;
    }

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

    // Helper to create a substring from byte range
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

// LinesView - line iterator
public struct LinesView: Iterable {
    type Item = String
    type Iter = LinesIterator

    private var ptr: lang.ptr[lang.i8]
    private var length: Int64

    public init(ptr ptr: lang.ptr[lang.i8], length length: Int64) {
        self.ptr = ptr;
        self.length = length;
    }

    public func iter() -> LinesIterator {
        LinesIterator(ptr: self.ptr, length: self.length, byteIndex: Int64(intLiteral: 0), done: false)
    }
}

// String index types for O(1) access after initial scan
public struct ByteIndex: Equatable, Comparable {
    public var value: Int64

    public init(value: Int64) {
        self.value = value;
    }

    public func equals(other: ByteIndex) -> Bool {
        self.value == other.value
    }

    public func compare(other: ByteIndex) -> Ordering {
        self.value.compare(other.value)
    }
}

public struct CodePointIndex: Equatable {
    public var byteOffset: Int64

    public init(byteOffset: Int64) {
        self.byteOffset = byteOffset;
    }

    public func equals(other: CodePointIndex) -> Bool {
        self.byteOffset == other.byteOffset
    }
}

public struct CharIndex: Equatable {
    public var byteOffset: Int64

    public init(byteOffset: Int64) {
        self.byteOffset = byteOffset;
    }

    public func equals(other: CharIndex) -> Bool {
        self.byteOffset == other.byteOffset
    }
}
